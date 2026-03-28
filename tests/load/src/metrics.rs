//! Load Test Metrics Collection
//!
//! Comprehensive metrics tracking for load tests including latency percentiles,
//! error rates, throughput, and resource usage (memory, CPU, GC).
//!
//! Memory-bounded: Uses ring buffers for resource snapshots and limits error
//! type cardinality to prevent unbounded memory growth in long-running tests.

use hdrhistogram::Histogram;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use sysinfo::{Pid, System};

/// Maximum number of resource snapshots to keep (ring buffer size)
/// At 1 snapshot/second, this covers ~16 minutes of data
const MAX_RESOURCE_SNAPSHOTS: usize = 1000;

/// Maximum number of unique error types to track
/// Prevents memory growth from high-cardinality error messages
const MAX_ERROR_TYPES: usize = 100;

/// Resource usage snapshot
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct ResourceSnapshot {
    /// Memory usage in MB
    pub memory_mb: f64,
    /// CPU usage percentage (0-100)
    pub cpu_percent: f32,
    /// Number of threads
    pub thread_count: usize,
    /// Timestamp of the snapshot
    pub timestamp: u64,
}

/// Comprehensive metrics for load testing
pub struct LoadMetrics {
    /// Test name
    pub name: String,
    /// Start time (wrapped for phase resets)
    start_time: RwLock<Instant>,
    /// Total requests
    total_requests: AtomicU64,
    /// Successful requests
    successful_requests: AtomicU64,
    /// Failed requests
    failed_requests: AtomicU64,
    /// Latency histogram (microseconds)
    latency_histogram: RwLock<Histogram<u64>>,
    /// Error counts by type
    error_counts: RwLock<HashMap<String, u64>>,
    /// Resource snapshots (VecDeque for O(1) pop_front)
    resource_snapshots: RwLock<VecDeque<ResourceSnapshot>>,
    /// Bytes sent
    bytes_sent: AtomicU64,
    /// Bytes received
    bytes_received: AtomicU64,
}

impl LoadMetrics {
    /// Create new metrics instance
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start_time: RwLock::new(Instant::now()),
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            latency_histogram: RwLock::new(
                Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).unwrap(),
            ),
            error_counts: RwLock::new(HashMap::new()),
            resource_snapshots: RwLock::new(VecDeque::new()),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
        }
    }

    /// Reset all counters, histograms, and the measurement window for a new phase.
    /// Resource snapshots are preserved (they span the full test).
    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.bytes_sent.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        if let Ok(mut t) = self.start_time.write() {
            *t = Instant::now();
        }
        if let Ok(mut hist) = self.latency_histogram.write() {
            hist.reset();
        }
        if let Ok(mut errors) = self.error_counts.write() {
            errors.clear();
        }
    }

    /// Record a successful request
    pub fn record_success(&self, latency: Duration) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
        let micros = latency.as_micros() as u64;
        if let Ok(mut hist) = self.latency_histogram.write() {
            let _ = hist.record(micros.min(60_000_000));
        }
    }

    /// Record a failed request (limits unique error types to prevent memory growth)
    pub fn record_failure(&self, latency: Duration, error: &str) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        let micros = latency.as_micros() as u64;
        if let Ok(mut hist) = self.latency_histogram.write() {
            let _ = hist.record(micros.min(60_000_000));
        }
        // Truncate error message
        let error_key = if error.len() > 100 {
            format!("{}...", &error[..97])
        } else {
            error.to_string()
        };
        if let Ok(mut errors) = self.error_counts.write() {
            // If key exists, increment it; otherwise only add if under cardinality limit
            if errors.contains_key(&error_key) {
                *errors.get_mut(&error_key).unwrap() += 1;
            } else if errors.len() < MAX_ERROR_TYPES {
                errors.insert(error_key, 1);
            } else {
                // At cardinality limit: increment "other" bucket
                *errors.entry("(other errors)".to_string()).or_insert(0) += 1;
            }
        }
    }

    /// Record bytes transferred
    #[allow(dead_code)]
    pub fn record_bytes(&self, sent: u64, received: u64) {
        self.bytes_sent.fetch_add(sent, Ordering::Relaxed);
        self.bytes_received.fetch_add(received, Ordering::Relaxed);
    }

    /// Record resource usage snapshot (ring buffer - removes oldest when full)
    pub fn record_resource_usage(&self, snapshot: ResourceSnapshot) {
        if let Ok(mut snapshots) = self.resource_snapshots.write() {
            // Ring buffer: pop_front is O(1) for VecDeque
            if snapshots.len() >= MAX_RESOURCE_SNAPSHOTS {
                snapshots.pop_front();
            }
            snapshots.push_back(snapshot);
        }
    }

    /// Get elapsed duration since last reset (or creation)
    pub fn elapsed(&self) -> Duration {
        self.start_time.read().map_or(Duration::ZERO, |t| t.elapsed())
    }

    /// Get total requests
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    /// Get successful requests
    pub fn successful_requests(&self) -> u64 {
        self.successful_requests.load(Ordering::Relaxed)
    }

    /// Get failed requests
    pub fn failed_requests(&self) -> u64 {
        self.failed_requests.load(Ordering::Relaxed)
    }

    /// Get requests per second
    pub fn rps(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.total_requests() as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Get error rate (0.0 - 1.0)
    pub fn error_rate(&self) -> f64 {
        let total = self.total_requests();
        if total > 0 {
            self.failed_requests() as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Get latency percentile in microseconds
    pub fn latency_percentile(&self, percentile: f64) -> u64 {
        if let Ok(hist) = self.latency_histogram.read() {
            hist.value_at_percentile(percentile)
        } else {
            0
        }
    }

    /// Get min latency in microseconds
    pub fn latency_min(&self) -> u64 {
        if let Ok(hist) = self.latency_histogram.read() {
            hist.min()
        } else {
            0
        }
    }

    /// Get max latency in microseconds
    pub fn latency_max(&self) -> u64 {
        if let Ok(hist) = self.latency_histogram.read() {
            hist.max()
        } else {
            0
        }
    }

    /// Get mean latency in microseconds
    pub fn latency_mean(&self) -> f64 {
        if let Ok(hist) = self.latency_histogram.read() {
            hist.mean()
        } else {
            0.0
        }
    }

    /// Get standard deviation of latency
    pub fn latency_stddev(&self) -> f64 {
        if let Ok(hist) = self.latency_histogram.read() {
            hist.stdev()
        } else {
            0.0
        }
    }

    /// Get top errors
    pub fn top_errors(&self, limit: usize) -> Vec<(String, u64)> {
        if let Ok(errors) = self.error_counts.read() {
            let mut sorted: Vec<_> = errors.iter().map(|(k, v)| (k.clone(), *v)).collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            sorted.truncate(limit);
            sorted
        } else {
            Vec::new()
        }
    }

    /// Get average memory usage in MB
    pub fn avg_memory_mb(&self) -> f64 {
        if let Ok(snapshots) = self.resource_snapshots.read() {
            if snapshots.is_empty() {
                return 0.0;
            }
            let sum: f64 = snapshots.iter().map(|s| s.memory_mb).sum();
            sum / snapshots.len() as f64
        } else {
            0.0
        }
    }

    /// Get max memory usage in MB
    pub fn max_memory_mb(&self) -> f64 {
        if let Ok(snapshots) = self.resource_snapshots.read() {
            snapshots
                .iter()
                .map(|s| s.memory_mb)
                .fold(0.0, f64::max)
        } else {
            0.0
        }
    }

    /// Get average CPU usage percentage
    pub fn avg_cpu_percent(&self) -> f32 {
        if let Ok(snapshots) = self.resource_snapshots.read() {
            if snapshots.is_empty() {
                return 0.0;
            }
            let sum: f32 = snapshots.iter().map(|s| s.cpu_percent).sum();
            sum / snapshots.len() as f32
        } else {
            0.0
        }
    }

    /// Get max CPU usage percentage
    pub fn max_cpu_percent(&self) -> f32 {
        if let Ok(snapshots) = self.resource_snapshots.read() {
            snapshots
                .iter()
                .map(|s| s.cpu_percent)
                .fold(0.0, f32::max)
        } else {
            0.0
        }
    }

    /// Get bytes sent
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    /// Get bytes received
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    /// Generate summary report
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            name: self.name.clone(),
            duration_secs: self.elapsed().as_secs_f64(),
            total_requests: self.total_requests(),
            successful_requests: self.successful_requests(),
            failed_requests: self.failed_requests(),
            rps: self.rps(),
            error_rate: self.error_rate(),
            latency_min_us: self.latency_min(),
            latency_p50_us: self.latency_percentile(50.0),
            latency_p95_us: self.latency_percentile(95.0),
            latency_p99_us: self.latency_percentile(99.0),
            latency_p999_us: self.latency_percentile(99.9),
            latency_max_us: self.latency_max(),
            latency_mean_us: self.latency_mean(),
            latency_stddev_us: self.latency_stddev(),
            avg_memory_mb: self.avg_memory_mb(),
            max_memory_mb: self.max_memory_mb(),
            avg_cpu_percent: self.avg_cpu_percent(),
            max_cpu_percent: self.max_cpu_percent(),
            bytes_sent: self.bytes_sent(),
            bytes_received: self.bytes_received(),
            top_errors: self.top_errors(5),
        }
    }
}

/// Metrics summary for reporting
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSummary {
    pub name: String,
    pub duration_secs: f64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub rps: f64,
    pub error_rate: f64,
    pub latency_min_us: u64,
    pub latency_p50_us: u64,
    pub latency_p95_us: u64,
    pub latency_p99_us: u64,
    pub latency_p999_us: u64,
    pub latency_max_us: u64,
    pub latency_mean_us: f64,
    pub latency_stddev_us: f64,
    pub avg_memory_mb: f64,
    pub max_memory_mb: f64,
    pub avg_cpu_percent: f32,
    pub max_cpu_percent: f32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub top_errors: Vec<(String, u64)>,
}

#[allow(dead_code)]
impl MetricsSummary {
    /// Format as human-readable report
    pub fn format_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("\n{}\n", "=".repeat(60)));
        report.push_str(&format!(" {} Results\n", self.name));
        report.push_str(&format!("{}\n\n", "=".repeat(60)));

        report.push_str("Throughput:\n");
        report.push_str(&format!("  Duration:     {:.2}s\n", self.duration_secs));
        report.push_str(&format!("  Total:        {} requests\n", self.total_requests));
        report.push_str(&format!("  Successful:   {} requests\n", self.successful_requests));
        report.push_str(&format!("  Failed:       {} requests\n", self.failed_requests));
        report.push_str(&format!("  RPS:          {:.2} req/s\n", self.rps));
        report.push_str(&format!("  Error Rate:   {:.2}%\n", self.error_rate * 100.0));
        report.push('\n');

        report.push_str("Latency (milliseconds):\n");
        report.push_str(&format!("  Min:    {:>10.2} ms\n", self.latency_min_us as f64 / 1000.0));
        report.push_str(&format!("  P50:    {:>10.2} ms\n", self.latency_p50_us as f64 / 1000.0));
        report.push_str(&format!("  P95:    {:>10.2} ms\n", self.latency_p95_us as f64 / 1000.0));
        report.push_str(&format!("  P99:    {:>10.2} ms\n", self.latency_p99_us as f64 / 1000.0));
        report.push_str(&format!("  P99.9:  {:>10.2} ms\n", self.latency_p999_us as f64 / 1000.0));
        report.push_str(&format!("  Max:    {:>10.2} ms\n", self.latency_max_us as f64 / 1000.0));
        report.push_str(&format!("  Mean:   {:>10.2} ms\n", self.latency_mean_us / 1000.0));
        report.push_str(&format!("  StdDev: {:>10.2} ms\n", self.latency_stddev_us / 1000.0));
        report.push('\n');

        report.push_str("Resource Usage:\n");
        report.push_str(&format!("  Avg Memory:   {:.2} MB\n", self.avg_memory_mb));
        report.push_str(&format!("  Max Memory:   {:.2} MB\n", self.max_memory_mb));
        report.push_str(&format!("  Avg CPU:      {:.1}%\n", self.avg_cpu_percent));
        report.push_str(&format!("  Max CPU:      {:.1}%\n", self.max_cpu_percent));
        report.push('\n');

        report.push_str("Data Transfer:\n");
        report.push_str(&format!(
            "  Sent:     {:.2} MB\n",
            self.bytes_sent as f64 / 1_048_576.0
        ));
        report.push_str(&format!(
            "  Received: {:.2} MB\n",
            self.bytes_received as f64 / 1_048_576.0
        ));
        report.push('\n');

        if !self.top_errors.is_empty() {
            report.push_str("Top Errors:\n");
            for (error, count) in &self.top_errors {
                report.push_str(&format!("  [{:>5}] {}\n", count, error));
            }
        }

        report
    }
}

/// Resource monitor for tracking process metrics
pub struct ResourceMonitor {
    system: System,
    pid: Pid,
}

impl ResourceMonitor {
    /// Create new resource monitor for the current process
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            pid: Pid::from_u32(std::process::id()),
        }
    }

    /// Create resource monitor for a specific PID
    pub fn for_pid(pid: u32) -> Self {
        Self {
            system: System::new_all(),
            pid: Pid::from_u32(pid),
        }
    }

    /// Take a resource snapshot
    pub fn snapshot(&mut self) -> ResourceSnapshot {
        self.system.refresh_all();

        if let Some(process) = self.system.process(self.pid) {
            ResourceSnapshot {
                memory_mb: process.memory() as f64 / 1_048_576.0,
                cpu_percent: process.cpu_usage(),
                thread_count: process.tasks().map_or(1, |t| t.len()),
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
            }
        } else {
            ResourceSnapshot::default()
        }
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared metrics wrapper for concurrent access
pub type SharedMetrics = Arc<LoadMetrics>;

/// Create shared metrics instance
pub fn create_metrics(name: &str) -> SharedMetrics {
    Arc::new(LoadMetrics::new(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let metrics = LoadMetrics::new("test");

        metrics.record_success(Duration::from_micros(100));
        metrics.record_success(Duration::from_micros(200));
        metrics.record_failure(Duration::from_micros(500), "timeout");

        assert_eq!(metrics.total_requests(), 3);
        assert_eq!(metrics.successful_requests(), 2);
        assert_eq!(metrics.failed_requests(), 1);
        assert!(metrics.error_rate() > 0.3 && metrics.error_rate() < 0.4);
    }

    #[test]
    fn test_latency_percentiles() {
        let metrics = LoadMetrics::new("test");

        for i in 1..=100 {
            metrics.record_success(Duration::from_micros(i * 10));
        }

        assert!(metrics.latency_percentile(50.0) >= 490 && metrics.latency_percentile(50.0) <= 510);
        assert!(metrics.latency_percentile(99.0) >= 980);
    }

    #[test]
    fn test_resource_snapshots() {
        let metrics = LoadMetrics::new("test");

        metrics.record_resource_usage(ResourceSnapshot {
            memory_mb: 100.0,
            cpu_percent: 50.0,
            thread_count: 4,
            timestamp: 0,
        });
        metrics.record_resource_usage(ResourceSnapshot {
            memory_mb: 200.0,
            cpu_percent: 80.0,
            thread_count: 8,
            timestamp: 1000,
        });

        assert_eq!(metrics.avg_memory_mb(), 150.0);
        assert_eq!(metrics.max_memory_mb(), 200.0);
        assert_eq!(metrics.avg_cpu_percent(), 65.0);
        assert_eq!(metrics.max_cpu_percent(), 80.0);
    }

    #[test]
    fn test_resource_snapshots_ring_buffer() {
        let metrics = LoadMetrics::new("test");

        // Fill to capacity + 10 to test ring buffer behavior
        for i in 0..(MAX_RESOURCE_SNAPSHOTS + 10) {
            metrics.record_resource_usage(ResourceSnapshot {
                memory_mb: i as f64,
                cpu_percent: 0.0,
                thread_count: 1,
                timestamp: i as u64,
            });
        }

        // Should be capped at MAX_RESOURCE_SNAPSHOTS
        let snapshots = metrics.resource_snapshots.read().unwrap();
        assert_eq!(snapshots.len(), MAX_RESOURCE_SNAPSHOTS);

        // Oldest snapshots (0-9) should have been evicted
        // First snapshot should now be index 10
        assert_eq!(snapshots[0].timestamp, 10);
        // Last snapshot should be the most recent
        assert_eq!(snapshots[MAX_RESOURCE_SNAPSHOTS - 1].timestamp, (MAX_RESOURCE_SNAPSHOTS + 9) as u64);
    }

    #[test]
    fn test_error_cardinality_limit() {
        let metrics = LoadMetrics::new("test");

        // Record MAX_ERROR_TYPES unique errors
        for i in 0..MAX_ERROR_TYPES {
            metrics.record_failure(Duration::from_micros(100), &format!("error_{}", i));
        }

        // Record more unique errors - these should go to "other" bucket
        for i in 0..50 {
            metrics.record_failure(Duration::from_micros(100), &format!("overflow_error_{}", i));
        }

        let errors = metrics.error_counts.read().unwrap();
        // Should have MAX_ERROR_TYPES original errors + 1 "other" bucket
        assert_eq!(errors.len(), MAX_ERROR_TYPES + 1);
        // "other" bucket should have 50 errors
        assert_eq!(errors.get("(other errors)"), Some(&50));
    }

    #[test]
    fn test_existing_error_increments_over_limit() {
        let metrics = LoadMetrics::new("test");

        // Fill up to cardinality limit
        for i in 0..MAX_ERROR_TYPES {
            metrics.record_failure(Duration::from_micros(100), &format!("error_{}", i));
        }

        // Recording an existing error should still work
        metrics.record_failure(Duration::from_micros(100), "error_0");
        metrics.record_failure(Duration::from_micros(100), "error_0");

        let errors = metrics.error_counts.read().unwrap();
        assert_eq!(errors.get("error_0"), Some(&3)); // 1 original + 2 additional
        assert_eq!(errors.len(), MAX_ERROR_TYPES); // No new keys added
    }
}
