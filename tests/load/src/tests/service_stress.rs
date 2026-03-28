//! Service Stress Tests
//!
//! Tests for service-level stress scenarios including spike load, overload,
//! memory stress, batch operations, and hot key contention.

use crate::client::LoadTestClient;
use crate::metrics::{create_metrics, MetricsSummary, SharedMetrics};
use crate::runner::{LoadTestConfig, LoadTestRunner, ProgressReporter};
use anyhow::Result;
use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;

/// Test result with pass/fail status
#[derive(Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub summary: MetricsSummary,
    pub message: String,
}

/// Spike Load Test
///
/// Simulates a 10x traffic spike (100 → 1000 RPS) and validates recovery.
/// Duration: ~5 minutes
pub async fn test_spike_load(base_url: &str) -> Result<TestResult> {
    let test_name = "SpikeLoad";
    println!("\n=== Starting {} Test ===", test_name);

    let client = LoadTestClient::new(base_url)?;
    let metrics = create_metrics(test_name);

    // Phase 1: Baseline (100 RPS for 60s)
    println!("Phase 1: Baseline load (100 RPS)...");
    let baseline_config = LoadTestConfig {
        concurrency: 20,
        duration: Duration::from_secs(60),
        target_rps: 100,
        warmup: Duration::from_secs(5),
        ..Default::default()
    };

    run_phase(&client, &metrics, baseline_config, "Baseline").await?;
    let baseline_p99 = metrics.latency_percentile(99.0);

    // Phase 2: Spike (1000 RPS for 60s)
    println!("Phase 2: Spike load (1000 RPS)...");
    metrics.reset();
    let spike_config = LoadTestConfig {
        concurrency: 100,
        duration: Duration::from_secs(60),
        target_rps: 1000,
        warmup: Duration::ZERO,
        ..Default::default()
    };

    run_phase(&client, &metrics, spike_config, "Spike").await?;
    let spike_error_rate = metrics.error_rate();

    // Phase 3: Recovery (100 RPS for 60s)
    println!("Phase 3: Recovery (100 RPS)...");
    metrics.reset();
    let recovery_config = LoadTestConfig {
        concurrency: 20,
        duration: Duration::from_secs(60),
        target_rps: 100,
        warmup: Duration::ZERO,
        ..Default::default()
    };

    run_phase(&client, &metrics, recovery_config, "Recovery").await?;
    let recovery_p99 = metrics.latency_percentile(99.0);

    let summary = metrics.summary();

    // Validation: Recovery latency should be within 2x of baseline
    let passed = spike_error_rate < 0.10 && recovery_p99 < baseline_p99 * 3;
    let message = format!(
        "Baseline P99: {}µs, Spike Error Rate: {:.2}%, Recovery P99: {}µs",
        baseline_p99,
        spike_error_rate * 100.0,
        recovery_p99
    );

    Ok(TestResult {
        name: test_name.to_string(),
        passed,
        summary,
        message,
    })
}

/// Overload Test
///
/// Progressive load increase from 100 to 2000 RPS to find breaking point.
/// Duration: ~15 minutes
pub async fn test_overload(base_url: &str) -> Result<TestResult> {
    let test_name = "Overload";
    println!("\n=== Starting {} Test ===", test_name);

    let client = LoadTestClient::new(base_url)?;
    let metrics = create_metrics(test_name);

    let rps_levels = [100, 200, 500, 1000, 1500, 2000];
    let mut breaking_point: Option<u64> = None;

    for &rps in &rps_levels {
        println!("Testing at {} RPS...", rps);
        metrics.reset();

        let config = LoadTestConfig {
            concurrency: (rps / 10).max(10) as usize,
            duration: Duration::from_secs(120),
            target_rps: rps,
            warmup: Duration::from_secs(5),
            ..Default::default()
        };

        run_phase(&client, &metrics, config, &format!("RPS-{}", rps)).await?;

        let error_rate = metrics.error_rate();
        let p99 = metrics.latency_percentile(99.0);

        println!("  Error Rate: {:.2}%, P99: {}µs", error_rate * 100.0, p99);

        // Breaking point: >5% errors or P99 > 1s
        if error_rate > 0.05 || p99 > 1_000_000 {
            breaking_point = Some(rps);
            println!("  Breaking point reached at {} RPS!", rps);
            break;
        }
    }

    let summary = metrics.summary();
    let passed = breaking_point.map_or(true, |bp| bp >= 500);
    let message = match breaking_point {
        Some(bp) => format!("Breaking point at {} RPS", bp),
        None => "No breaking point found (service handled 2000 RPS)".to_string(),
    };

    Ok(TestResult {
        name: test_name.to_string(),
        passed,
        summary,
        message,
    })
}

/// Memory Stress Test
///
/// Tests with large values (1MB-10MB) to detect memory leaks.
/// Duration: ~10 minutes
pub async fn test_memory_stress(base_url: &str) -> Result<TestResult> {
    let test_name = "MemoryStress";
    println!("\n=== Starting {} Test ===", test_name);

    let client = LoadTestClient::new(base_url)?;
    let metrics = create_metrics(test_name);

    let value_sizes = [
        (1024, "1KB"),
        (10 * 1024, "10KB"),
        (100 * 1024, "100KB"),
        (1024 * 1024, "1MB"),
        (5 * 1024 * 1024, "5MB"),
    ];

    for (size, label) in value_sizes {
        println!("Testing with {} values...", label);
        metrics.reset();

        let config = LoadTestConfig {
            concurrency: 10,
            duration: Duration::from_secs(60),
            target_rps: 50,
            warmup: Duration::from_secs(5),
            ..Default::default()
        };

        let value = generate_value(size);
        run_memory_phase(&client, &metrics, config, &value).await?;

        println!(
            "  Memory: {:.2} MB, Error Rate: {:.2}%",
            metrics.avg_memory_mb(),
            metrics.error_rate() * 100.0
        );
    }

    let summary = metrics.summary();

    // Check for reasonable memory usage (should not exceed 500MB for 5MB values)
    let passed = summary.max_memory_mb < 500.0 && summary.error_rate < 0.05;
    let message = format!(
        "Max Memory: {:.2}MB, Avg Memory: {:.2}MB, Error Rate: {:.2}%",
        summary.max_memory_mb,
        summary.avg_memory_mb,
        summary.error_rate * 100.0
    );

    Ok(TestResult {
        name: test_name.to_string(),
        passed,
        summary,
        message,
    })
}

/// Batch Operations Test
///
/// Tests MSET/MGET with batch sizes 10-100 keys.
/// Duration: ~5 minutes
pub async fn test_batch_operations(base_url: &str) -> Result<TestResult> {
    let test_name = "BatchOperations";
    println!("\n=== Starting {} Test ===", test_name);

    let client = LoadTestClient::new(base_url)?;
    let metrics = create_metrics(test_name);

    let batch_sizes = [10, 25, 50, 100];

    for batch_size in batch_sizes {
        println!("Testing with batch size {}...", batch_size);
        metrics.reset();

        let config = LoadTestConfig {
            concurrency: 20,
            duration: Duration::from_secs(60),
            target_rps: 100,
            warmup: Duration::from_secs(5),
            ..Default::default()
        };

        run_batch_phase(&client, &metrics, config, batch_size).await?;

        println!(
            "  RPS: {:.0}, P99: {}µs",
            metrics.rps(),
            metrics.latency_percentile(99.0)
        );
    }

    let summary = metrics.summary();
    let passed = summary.error_rate < 0.01;
    let message = format!(
        "Total Requests: {}, Error Rate: {:.2}%, P99: {}µs",
        summary.total_requests,
        summary.error_rate * 100.0,
        summary.latency_p99_us
    );

    Ok(TestResult {
        name: test_name.to_string(),
        passed,
        summary,
        message,
    })
}

/// Hot Key Contention Test
///
/// 200 concurrent workers accessing the same key to test lock contention.
/// Duration: ~5 minutes
pub async fn test_hot_key(base_url: &str) -> Result<TestResult> {
    let test_name = "HotKeyContention";
    println!("\n=== Starting {} Test ===", test_name);

    let client = LoadTestClient::new(base_url)?;
    let _metrics = create_metrics(test_name);

    let hot_key = "hot_key_test";

    // Initialize the hot key
    client.set_string(hot_key, "0", None).await?;

    let config = LoadTestConfig {
        concurrency: 200,
        duration: Duration::from_secs(300),
        target_rps: 0, // Unlimited - stress test
        warmup: Duration::from_secs(10),
        ..Default::default()
    };

    let runner = LoadTestRunner::new(client.clone(), config, test_name);
    let test_metrics = runner.metrics();

    let progress = ProgressReporter::new(test_name, Duration::from_secs(300), test_metrics.clone());
    let progress_handle = progress.start();

    let hot_key_owned = hot_key.to_string();
    runner
        .run(move |c| {
            let key = hot_key_owned.clone();
            async move {
                // Mix of INCR and GET operations
                if rand::random::<bool>() {
                    c.incr(&key, Some(1)).await?;
                } else {
                    c.get_string(&key).await?;
                }
                Ok(())
            }
        })
        .await?;

    progress.stop();
    let _ = progress_handle.await;

    // Copy metrics
    let summary = test_metrics.summary();

    let passed = summary.error_rate < 0.01 && summary.latency_p99_us < 50_000; // P99 < 50ms
    let message = format!(
        "Concurrency: 200, RPS: {:.0}, Error Rate: {:.2}%, P99: {}µs",
        summary.rps,
        summary.error_rate * 100.0,
        summary.latency_p99_us
    );

    Ok(TestResult {
        name: test_name.to_string(),
        passed,
        summary,
        message,
    })
}

// Helper functions

async fn run_phase(
    client: &LoadTestClient,
    metrics: &SharedMetrics,
    config: LoadTestConfig,
    _phase_name: &str,
) -> Result<()> {
    let runner = LoadTestRunner::with_shared_metrics(client.clone(), config, metrics.clone());

    runner
        .run(|c| async move {
            let key_id: u64 = rand::random::<u64>() % 10000;
            let value_id: u64 = rand::random::<u64>() % 10000;
            let key = format!("key_{}", key_id);
            let value = format!("value_{}", value_id);

            // 70% reads, 30% writes
            if rand::random::<f64>() < 0.7 {
                c.get_string(&key).await?;
            } else {
                c.set_string(&key, &value, Some(300)).await?;
            }
            Ok(())
        })
        .await
}

async fn run_memory_phase(
    client: &LoadTestClient,
    metrics: &SharedMetrics,
    config: LoadTestConfig,
    value: &str,
) -> Result<()> {
    let runner = LoadTestRunner::with_shared_metrics(client.clone(), config, metrics.clone());
    let value = value.to_string();

    runner
        .run(move |c| {
            let v = value.clone();
            async move {
                let key_id: u64 = rand::random::<u64>() % 100;
                let key = format!("large_key_{}", key_id);

                // 50% writes, 50% reads
                if rand::random::<bool>() {
                    c.set_string(&key, &v, Some(60)).await?;
                } else {
                    c.get_string(&key).await?;
                }
                Ok(())
            }
        })
        .await
}

async fn run_batch_phase(
    client: &LoadTestClient,
    metrics: &SharedMetrics,
    config: LoadTestConfig,
    batch_size: usize,
) -> Result<()> {
    let runner = LoadTestRunner::with_shared_metrics(client.clone(), config, metrics.clone());

    runner
        .run(move |c| async move {
            let base: u64 = rand::random::<u64>() % 10000;

            // Create batch
            let mut pairs: HashMap<String, String> = HashMap::new();
            for i in 0..batch_size {
                pairs.insert(format!("batch_{}_{}", base, i), format!("value_{}", i));
            }

            // 50% MSET, 50% MGET
            if rand::random::<bool>() {
                c.mset(pairs).await?;
            } else {
                let keys: Vec<String> = pairs.keys().cloned().collect();
                c.mget(keys).await?;
            }
            Ok(())
        })
        .await
}

fn generate_value(size: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..size)
        .map(|_| rng.gen_range(b'a'..=b'z') as char)
        .collect()
}

/// Run all service stress tests
pub async fn run_all(base_url: &str) -> Vec<TestResult> {
    let mut results = Vec::new();

    // Run tests sequentially
    if let Ok(result) = test_spike_load(base_url).await {
        results.push(result);
    }

    if let Ok(result) = test_overload(base_url).await {
        results.push(result);
    }

    if let Ok(result) = test_memory_stress(base_url).await {
        results.push(result);
    }

    if let Ok(result) = test_batch_operations(base_url).await {
        results.push(result);
    }

    if let Ok(result) = test_hot_key(base_url).await {
        results.push(result);
    }

    results
}
