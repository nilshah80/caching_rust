//! Load Test Runner
//!
//! Concurrent request execution with rate limiting and resource monitoring.

use crate::client::LoadTestClient;
use crate::metrics::{create_metrics, ResourceMonitor, SharedMetrics};
use anyhow::Result;
use futures::future::join_all;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::interval;

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    tokens: Arc<Semaphore>,
    refill_rate: u64,
    running: Arc<AtomicBool>,
}

impl RateLimiter {
    /// Create new rate limiter with specified RPS
    pub fn new(rps: u64) -> Self {
        let tokens = Arc::new(Semaphore::new(rps as usize));
        let running = Arc::new(AtomicBool::new(true));

        // Spawn token refiller
        let tokens_clone = tokens.clone();
        let running_clone = running.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(1));
            while running_clone.load(Ordering::Relaxed) {
                ticker.tick().await;
                // Refill tokens up to capacity
                let available = tokens_clone.available_permits();
                let to_add = (rps as usize).saturating_sub(available);
                tokens_clone.add_permits(to_add);
            }
        });

        Self {
            tokens,
            refill_rate: rps,
            running,
        }
    }

    /// Acquire a token (wait if necessary)
    pub async fn acquire(&self) {
        let _ = self.tokens.acquire().await;
    }

    /// Get current RPS setting
    pub fn rps(&self) -> u64 {
        self.refill_rate
    }
}

impl Drop for RateLimiter {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// Load test configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    /// Number of concurrent workers
    pub concurrency: usize,
    /// Test duration
    pub duration: Duration,
    /// Target RPS (0 = unlimited)
    pub target_rps: u64,
    /// Resource monitoring interval
    pub monitor_interval: Duration,
    /// Warmup duration
    pub warmup: Duration,
    /// Ramp-up duration (gradual increase to target concurrency)
    pub ramp_up: Duration,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            concurrency: 10,
            duration: Duration::from_secs(30),
            target_rps: 0,
            monitor_interval: Duration::from_secs(2),
            warmup: Duration::from_secs(5),
            ramp_up: Duration::from_secs(0),
        }
    }
}

/// Request function type
pub type RequestFn = Box<dyn Fn(LoadTestClient) -> futures::future::BoxFuture<'static, Result<()>> + Send + Sync>;

/// Load test runner
pub struct LoadTestRunner {
    client: LoadTestClient,
    config: LoadTestConfig,
    metrics: SharedMetrics,
    target_pid: Option<u32>,
}

impl LoadTestRunner {
    /// Create new runner
    pub fn new(client: LoadTestClient, config: LoadTestConfig, test_name: &str) -> Self {
        Self {
            client,
            config,
            metrics: create_metrics(test_name),
            target_pid: None,
        }
    }

    /// Set target PID for resource monitoring
    pub fn with_target_pid(mut self, pid: u32) -> Self {
        self.target_pid = Some(pid);
        self
    }

    /// Get metrics
    pub fn metrics(&self) -> SharedMetrics {
        self.metrics.clone()
    }

    /// Run the load test
    pub async fn run<F, Fut>(&self, request_fn: F) -> Result<()>
    where
        F: Fn(LoadTestClient) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let running = Arc::new(AtomicBool::new(true));

        // Start resource monitor
        let monitor_handle = self.start_resource_monitor(running.clone());

        // Warmup phase
        if !self.config.warmup.is_zero() {
            println!("Warmup phase ({:?})...", self.config.warmup);
            self.run_workers(
                request_fn.clone(),
                self.config.warmup,
                self.config.concurrency / 2,
                false,
            )
            .await;
        }

        // Main test phase
        println!(
            "Running test for {:?} with {} workers...",
            self.config.duration, self.config.concurrency
        );
        self.run_workers(
            request_fn,
            self.config.duration,
            self.config.concurrency,
            true,
        )
        .await;

        // Stop monitoring
        running.store(false, Ordering::Relaxed);
        let _ = monitor_handle.await;

        Ok(())
    }

    /// Run workers for specified duration
    async fn run_workers<F, Fut>(
        &self,
        request_fn: F,
        duration: Duration,
        concurrency: usize,
        record_metrics: bool,
    ) where
        F: Fn(LoadTestClient) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let deadline = Instant::now() + duration;
        let running = Arc::new(AtomicBool::new(true));

        // Optional rate limiter
        let rate_limiter = if self.config.target_rps > 0 {
            Some(Arc::new(RateLimiter::new(self.config.target_rps)))
        } else {
            None
        };

        // Spawn workers
        let mut handles = Vec::with_capacity(concurrency);

        for _ in 0..concurrency {
            let client = self.client.clone();
            let metrics = self.metrics.clone();
            let running = running.clone();
            let request_fn = request_fn.clone();
            let rate_limiter = rate_limiter.clone();
            let record = record_metrics;

            let handle = tokio::spawn(async move {
                while running.load(Ordering::Relaxed) && Instant::now() < deadline {
                    // Rate limiting
                    if let Some(ref limiter) = rate_limiter {
                        limiter.acquire().await;
                    }

                    let start = Instant::now();
                    let result = request_fn(client.clone()).await;
                    let elapsed = start.elapsed();

                    if record {
                        match result {
                            Ok(()) => metrics.record_success(elapsed),
                            Err(e) => metrics.record_failure(elapsed, &e.to_string()),
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for deadline
        tokio::time::sleep(duration).await;
        running.store(false, Ordering::Relaxed);

        // Wait for all workers
        join_all(handles).await;
    }

    /// Start resource monitoring
    fn start_resource_monitor(&self, running: Arc<AtomicBool>) -> tokio::task::JoinHandle<()> {
        let metrics = self.metrics.clone();
        let interval_duration = self.config.monitor_interval;
        let target_pid = self.target_pid;

        tokio::spawn(async move {
            let mut monitor = target_pid
                .map(ResourceMonitor::for_pid)
                .unwrap_or_default();
            let mut ticker = interval(interval_duration);

            while running.load(Ordering::Relaxed) {
                ticker.tick().await;
                let snapshot = monitor.snapshot();
                metrics.record_resource_usage(snapshot);
            }
        })
    }
}

/// Run concurrent requests with custom function
pub async fn run_concurrent<F, Fut>(
    client: LoadTestClient,
    concurrency: usize,
    duration: Duration,
    request_fn: F,
) -> SharedMetrics
where
    F: Fn(LoadTestClient) -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = Result<()>> + Send + 'static,
{
    let config = LoadTestConfig {
        concurrency,
        duration,
        warmup: Duration::ZERO,
        ..Default::default()
    };

    let runner = LoadTestRunner::new(client, config, "concurrent_test");
    let metrics = runner.metrics();

    let _ = runner.run(request_fn).await;

    metrics
}

/// Progress reporter for long-running tests
pub struct ProgressReporter {
    test_name: String,
    total_duration: Duration,
    start_time: Instant,
    metrics: SharedMetrics,
    running: Arc<AtomicBool>,
}

impl ProgressReporter {
    /// Create new progress reporter
    pub fn new(test_name: &str, total_duration: Duration, metrics: SharedMetrics) -> Self {
        Self {
            test_name: test_name.to_string(),
            total_duration,
            start_time: Instant::now(),
            metrics,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Start reporting progress
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let test_name = self.test_name.clone();
        let total_duration = self.total_duration;
        let start_time = self.start_time;
        let metrics = self.metrics.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(5));

            while running.load(Ordering::Relaxed) {
                ticker.tick().await;

                let elapsed = start_time.elapsed();
                let progress = (elapsed.as_secs_f64() / total_duration.as_secs_f64() * 100.0).min(100.0);
                let rps = metrics.rps();
                let errors = metrics.failed_requests();
                let p99 = metrics.latency_percentile(99.0);

                println!(
                    "[{}] Progress: {:.1}% | RPS: {:.0} | Errors: {} | P99: {} µs",
                    test_name, progress, rps, errors, p99
                );

                if elapsed >= total_duration {
                    break;
                }
            }
        })
    }

    /// Stop reporting
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Drop for ProgressReporter {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(100);
        assert_eq!(limiter.rps(), 100);

        // Should be able to acquire immediately
        limiter.acquire().await;
    }

    #[test]
    fn test_load_test_config_default() {
        let config = LoadTestConfig::default();
        assert_eq!(config.concurrency, 10);
        assert_eq!(config.duration, Duration::from_secs(30));
    }
}
