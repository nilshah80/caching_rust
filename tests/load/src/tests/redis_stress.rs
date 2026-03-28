//! Redis Stress Tests
//!
//! Tests focused on Redis-specific stress scenarios including large datasets,
//! connection stress, key expiration, and memory pressure.

use crate::client::LoadTestClient;
use crate::metrics::{create_metrics, SharedMetrics};
use crate::runner::{LoadTestConfig, LoadTestRunner};
use crate::tests::service_stress::TestResult;
use anyhow::Result;
use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;

/// Large Dataset Test
///
/// Tests performance with 10K to 1M keys in Redis.
/// Duration: ~30 minutes
pub async fn test_large_dataset(base_url: &str) -> Result<TestResult> {
    let test_name = "LargeDataset";
    println!("\n=== Starting {} Test ===", test_name);

    let client = LoadTestClient::new(base_url)?;
    let metrics = create_metrics(test_name);

    let dataset_sizes = [10_000, 50_000, 100_000, 500_000];

    for size in dataset_sizes {
        println!("Loading {} keys...", size);
        metrics.reset();

        // Write phase - populate dataset
        let write_config = LoadTestConfig {
            concurrency: 50,
            duration: Duration::from_secs((size / 1000).max(30) as u64),
            target_rps: 5000,
            warmup: Duration::ZERO,
            ..Default::default()
        };

        run_write_phase(&client, &metrics, write_config, size).await?;

        println!("  Write complete. Testing reads...");

        // Read phase - test read performance at scale
        metrics.reset();
        let read_config = LoadTestConfig {
            concurrency: 100,
            duration: Duration::from_secs(60),
            target_rps: 10000,
            warmup: Duration::from_secs(5),
            ..Default::default()
        };

        run_read_phase(&client, &metrics, read_config, size).await?;

        println!(
            "  Dataset Size: {}, RPS: {:.0}, P99: {}µs",
            size,
            metrics.rps(),
            metrics.latency_percentile(99.0)
        );
    }

    let summary = metrics.summary();
    let passed = summary.error_rate < 0.01;
    let message = format!(
        "Max Dataset: 500K keys, RPS: {:.0}, Error Rate: {:.2}%",
        summary.rps,
        summary.error_rate * 100.0
    );

    Ok(TestResult {
        name: test_name.to_string(),
        passed,
        summary,
        message,
    })
}

/// Connection Stress Test
///
/// Tests behavior with 100-2000 concurrent connections.
/// Duration: ~10 minutes
pub async fn test_connection_stress(base_url: &str) -> Result<TestResult> {
    let test_name = "ConnectionStress";
    println!("\n=== Starting {} Test ===", test_name);

    let client = LoadTestClient::new(base_url)?;
    let metrics = create_metrics(test_name);

    let connection_levels = [100, 200, 500, 1000, 2000];
    let mut max_sustainable = 0;

    for connections in connection_levels {
        println!("Testing with {} concurrent connections...", connections);
        metrics.reset();

        let config = LoadTestConfig {
            concurrency: connections,
            duration: Duration::from_secs(60),
            target_rps: 0, // Unlimited
            warmup: Duration::from_secs(10),
            ..Default::default()
        };

        run_connection_phase(&client, &metrics, config).await?;

        let error_rate = metrics.error_rate();
        let p99 = metrics.latency_percentile(99.0);

        println!("  Connections: {}, Error Rate: {:.2}%, P99: {}µs",
            connections, error_rate * 100.0, p99);

        if error_rate < 0.05 && p99 < 100_000 {
            max_sustainable = connections;
        } else {
            println!("  Connection limit reached!");
            break;
        }
    }

    let summary = metrics.summary();
    let passed = max_sustainable >= 500;
    let message = format!(
        "Max Sustainable Connections: {}, P99: {}µs",
        max_sustainable, summary.latency_p99_us
    );

    Ok(TestResult {
        name: test_name.to_string(),
        passed,
        summary,
        message,
    })
}

/// Key Expiration Stress Test
///
/// Tests 10K keys with TTL expiring under load.
/// Duration: ~5 minutes
pub async fn test_key_expiration(base_url: &str) -> Result<TestResult> {
    let test_name = "KeyExpiration";
    println!("\n=== Starting {} Test ===", test_name);

    let client = LoadTestClient::new(base_url)?;
    let metrics = create_metrics(test_name);

    let key_count = 10_000;
    let ttl_seconds = 30;

    println!("Setting {} keys with {}s TTL...", key_count, ttl_seconds);

    // Write keys with TTL
    let write_config = LoadTestConfig {
        concurrency: 50,
        duration: Duration::from_secs(30),
        target_rps: 1000,
        warmup: Duration::ZERO,
        ..Default::default()
    };

    run_expiring_write_phase(&client, &metrics, write_config, key_count, ttl_seconds).await?;

    println!("Waiting for keys to expire while reading...");
    metrics.reset();

    // Read while keys expire
    let read_config = LoadTestConfig {
        concurrency: 50,
        duration: Duration::from_secs(60),
        target_rps: 500,
        warmup: Duration::ZERO,
        ..Default::default()
    };

    run_expiring_read_phase(&client, &metrics, read_config, key_count).await?;

    // Continue reading after most keys should be expired
    println!("Testing reads after expiration...");
    metrics.reset();

    let post_expire_config = LoadTestConfig {
        concurrency: 20,
        duration: Duration::from_secs(30),
        target_rps: 200,
        warmup: Duration::ZERO,
        ..Default::default()
    };

    run_expiring_read_phase(&client, &metrics, post_expire_config, key_count).await?;

    let summary = metrics.summary();
    let passed = summary.error_rate < 0.01; // Cache misses are not errors
    let message = format!(
        "Keys: {}, TTL: {}s, Error Rate: {:.2}%",
        key_count, ttl_seconds,
        summary.error_rate * 100.0
    );

    Ok(TestResult {
        name: test_name.to_string(),
        passed,
        summary,
        message,
    })
}

/// Memory Pressure Test
///
/// Progressive value sizes from 100B to 100KB to test memory limits.
/// Duration: ~10 minutes
pub async fn test_memory_pressure(base_url: &str) -> Result<TestResult> {
    let test_name = "MemoryPressure";
    println!("\n=== Starting {} Test ===", test_name);

    let client = LoadTestClient::new(base_url)?;
    let metrics = create_metrics(test_name);

    let value_sizes = [
        (100, "100B"),
        (1_024, "1KB"),
        (10_240, "10KB"),
        (51_200, "50KB"),
        (102_400, "100KB"),
    ];

    for (size, label) in value_sizes {
        println!("Testing with {} values...", label);
        metrics.reset();

        let config = LoadTestConfig {
            concurrency: 30,
            duration: Duration::from_secs(90),
            target_rps: 200,
            warmup: Duration::from_secs(10),
            ..Default::default()
        };

        run_memory_pressure_phase(&client, &metrics, config, size).await?;

        println!(
            "  Value Size: {}, Memory: {:.2}MB, P99: {}µs",
            label,
            metrics.max_memory_mb(),
            metrics.latency_percentile(99.0)
        );
    }

    let summary = metrics.summary();
    let passed = summary.error_rate < 0.05;
    let message = format!(
        "Max Value: 100KB, Max Memory: {:.2}MB, Error Rate: {:.2}%",
        summary.max_memory_mb,
        summary.error_rate * 100.0
    );

    Ok(TestResult {
        name: test_name.to_string(),
        passed,
        summary,
        message,
    })
}

/// Command Pipeline Test
///
/// Tests batch operations with pipeline-like behavior (MSET/MGET).
/// Duration: ~5 minutes
pub async fn test_command_pipeline(base_url: &str) -> Result<TestResult> {
    let test_name = "CommandPipeline";
    println!("\n=== Starting {} Test ===", test_name);

    let client = LoadTestClient::new(base_url)?;
    let metrics = create_metrics(test_name);

    let batch_sizes = [10, 25, 50, 100];

    for batch_size in batch_sizes {
        println!("Testing pipeline with batch size {}...", batch_size);
        metrics.reset();

        let config = LoadTestConfig {
            concurrency: 30,
            duration: Duration::from_secs(60),
            target_rps: 500,
            warmup: Duration::from_secs(5),
            ..Default::default()
        };

        run_pipeline_phase(&client, &metrics, config, batch_size).await?;

        let ops_per_request = batch_size as f64;
        let effective_rps = metrics.rps() * ops_per_request;

        println!(
            "  Batch: {}, Effective OPS: {:.0}, P99: {}µs",
            batch_size,
            effective_rps,
            metrics.latency_percentile(99.0)
        );
    }

    let summary = metrics.summary();
    let passed = summary.error_rate < 0.01;
    let message = format!(
        "Max Batch: 100, Total Ops: {}, Error Rate: {:.2}%",
        summary.total_requests,
        summary.error_rate * 100.0
    );

    Ok(TestResult {
        name: test_name.to_string(),
        passed,
        summary,
        message,
    })
}

// Helper functions

async fn run_write_phase(
    client: &LoadTestClient,
    metrics: &SharedMetrics,
    config: LoadTestConfig,
    key_count: usize,
) -> Result<()> {
    let runner = LoadTestRunner::with_shared_metrics(client.clone(), config, metrics.clone());

    runner
        .run(move |c| async move {
            let key_idx: u64 = rand::random::<u64>() % key_count as u64;
            let value_idx: u64 = rand::random::<u64>() % 1000000;
            let key = format!("dataset_key_{}", key_idx);
            let value = format!("value_{}", value_idx);
            c.set_string(&key, &value, Some(3600)).await
        })
        .await
}

async fn run_read_phase(
    client: &LoadTestClient,
    metrics: &SharedMetrics,
    config: LoadTestConfig,
    key_count: usize,
) -> Result<()> {
    let runner = LoadTestRunner::with_shared_metrics(client.clone(), config, metrics.clone());

    runner
        .run(move |c| async move {
            let key_idx: u64 = rand::random::<u64>() % key_count as u64;
            let key = format!("dataset_key_{}", key_idx);
            c.get_string(&key).await?;
            Ok(())
        })
        .await
}

async fn run_connection_phase(
    client: &LoadTestClient,
    metrics: &SharedMetrics,
    config: LoadTestConfig,
) -> Result<()> {
    let runner = LoadTestRunner::with_shared_metrics(client.clone(), config, metrics.clone());

    runner
        .run(|c| async move {
            let key_idx: u64 = rand::random::<u64>() % 10000;
            let key = format!("conn_test_{}", key_idx);

            // Simple SET/GET
            if rand::random::<bool>() {
                c.set_string(&key, "test_value", Some(60)).await?;
            } else {
                c.get_string(&key).await?;
            }
            Ok(())
        })
        .await
}

async fn run_expiring_write_phase(
    client: &LoadTestClient,
    metrics: &SharedMetrics,
    config: LoadTestConfig,
    key_count: usize,
    ttl: u64,
) -> Result<()> {
    let runner = LoadTestRunner::with_shared_metrics(client.clone(), config, metrics.clone());

    runner
        .run(move |c| async move {
            let key_idx: u64 = rand::random::<u64>() % key_count as u64;
            let value_idx: u64 = rand::random::<u64>() % 1000000;
            let key = format!("expiring_key_{}", key_idx);
            let value = format!("value_{}", value_idx);
            c.set_string(&key, &value, Some(ttl)).await
        })
        .await
}

async fn run_expiring_read_phase(
    client: &LoadTestClient,
    metrics: &SharedMetrics,
    config: LoadTestConfig,
    key_count: usize,
) -> Result<()> {
    let runner = LoadTestRunner::with_shared_metrics(client.clone(), config, metrics.clone());

    runner
        .run(move |c| async move {
            let key_idx: u64 = rand::random::<u64>() % key_count as u64;
            let key = format!("expiring_key_{}", key_idx);
            // Cache miss is OK (key may have expired)
            let _ = c.get_string(&key).await;
            Ok(())
        })
        .await
}

async fn run_memory_pressure_phase(
    client: &LoadTestClient,
    metrics: &SharedMetrics,
    config: LoadTestConfig,
    value_size: usize,
) -> Result<()> {
    let runner = LoadTestRunner::with_shared_metrics(client.clone(), config, metrics.clone());
    let value = generate_value(value_size);

    runner
        .run(move |c| {
            let v = value.clone();
            async move {
                let key_idx: u64 = rand::random::<u64>() % 1000;
                let key = format!("pressure_key_{}", key_idx);

                // 70% writes, 30% reads
                if rand::random::<f64>() < 0.7 {
                    c.set_string(&key, &v, Some(120)).await?;
                } else {
                    c.get_string(&key).await?;
                }
                Ok(())
            }
        })
        .await
}

async fn run_pipeline_phase(
    client: &LoadTestClient,
    metrics: &SharedMetrics,
    config: LoadTestConfig,
    batch_size: usize,
) -> Result<()> {
    let runner = LoadTestRunner::with_shared_metrics(client.clone(), config, metrics.clone());

    runner
        .run(move |c| async move {
            let base: u64 = rand::random::<u64>() % 100000;

            // Create batch
            let mut pairs: HashMap<String, String> = HashMap::new();
            let keys: Vec<String> = (0..batch_size)
                .map(|i| {
                    let key = format!("pipe_{}_{}", base, i);
                    pairs.insert(key.clone(), format!("value_{}", i));
                    key
                })
                .collect();

            // Alternate between MSET and MGET
            if rand::random::<bool>() {
                c.mset(pairs).await?;
            } else {
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

/// Run all Redis stress tests
pub async fn run_all(base_url: &str) -> Vec<TestResult> {
    let mut results = Vec::new();

    if let Ok(result) = test_large_dataset(base_url).await {
        results.push(result);
    }

    if let Ok(result) = test_connection_stress(base_url).await {
        results.push(result);
    }

    if let Ok(result) = test_key_expiration(base_url).await {
        results.push(result);
    }

    if let Ok(result) = test_memory_pressure(base_url).await {
        results.push(result);
    }

    if let Ok(result) = test_command_pipeline(base_url).await {
        results.push(result);
    }

    results
}
