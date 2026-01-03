//! Benchmark Comparison Tool
//!
//! Compares performance metrics between Rust and Go caching services.
//! Captures latency, throughput, memory, CPU, and other metrics.

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

mod client;
mod metrics;
mod runner;

use client::LoadTestClient;
use metrics::MetricsSummary;
use runner::{LoadTestConfig, LoadTestRunner};

#[derive(Parser)]
#[command(name = "benchmark_compare")]
#[command(about = "Compare performance between Rust and Go caching services")]
struct Cli {
    /// Rust service URL
    #[arg(long, default_value = "http://localhost:8080")]
    rust_url: String,

    /// Go service URL
    #[arg(long, default_value = "http://localhost:8081")]
    go_url: String,

    /// Number of concurrent workers
    #[arg(short, long, default_value = "50")]
    concurrency: usize,

    /// Test duration in seconds
    #[arg(short, long, default_value = "60")]
    duration: u64,

    /// Target RPS (0 = unlimited)
    #[arg(short, long, default_value = "0")]
    rps: u64,

    /// Warmup duration in seconds
    #[arg(short, long, default_value = "10")]
    warmup: u64,

    /// Output format (text, json, csv)
    #[arg(short, long, default_value = "text")]
    format: String,

    /// Container name for Rust service (for resource monitoring)
    #[arg(long, default_value = "rust-caching-service")]
    rust_container: String,

    /// Container name for Go service (for resource monitoring)
    #[arg(long, default_value = "go-caching-service")]
    go_container: String,
}

/// Service benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceBenchmark {
    pub name: String,
    pub url: String,
    pub metrics: MetricsSummary,
    pub container_stats: Option<ContainerStats>,
}

/// Container resource statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub memory_mb: f64,
    pub cpu_percent: f32,
    pub memory_limit_mb: f64,
}

/// Comparison result
#[derive(Debug, Clone, Serialize)]
pub struct ComparisonResult {
    pub rust: ServiceBenchmark,
    pub go: ServiceBenchmark,
    pub comparison: MetricsComparison,
}

/// Metrics comparison (Rust vs Go)
#[derive(Debug, Clone, Serialize)]
pub struct MetricsComparison {
    /// RPS difference (positive = Rust faster)
    pub rps_diff_percent: f64,
    /// P50 latency difference (positive = Rust faster)
    pub p50_diff_percent: f64,
    /// P99 latency difference (positive = Rust faster)
    pub p99_diff_percent: f64,
    /// Memory difference (positive = Rust uses less)
    pub memory_diff_percent: f64,
    /// CPU difference (positive = Rust uses less)
    pub cpu_diff_percent: f64,
    /// Winner for each category
    pub winners: HashMap<String, String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    println!("{}", "=".repeat(70).blue());
    println!("{}", " Rust vs Go Caching Service Benchmark ".blue().bold());
    println!("{}", "=".repeat(70).blue());
    println!();
    println!("Configuration:");
    println!("  Rust URL:    {}", cli.rust_url.cyan());
    println!("  Go URL:      {}", cli.go_url.cyan());
    println!("  Concurrency: {}", cli.concurrency);
    println!("  Duration:    {}s", cli.duration);
    println!("  Target RPS:  {}", if cli.rps == 0 { "unlimited".to_string() } else { cli.rps.to_string() });
    println!("  Warmup:      {}s", cli.warmup);
    println!();

    // Check service health
    let rust_client = LoadTestClient::new(&cli.rust_url)?;
    let go_client = LoadTestClient::new(&cli.go_url)?;

    print!("Checking Rust service... ");
    match rust_client.health().await {
        Ok(true) => println!("{}", "OK".green()),
        _ => {
            println!("{}", "FAILED".red());
            return Err(anyhow::anyhow!("Rust service not available"));
        }
    }

    print!("Checking Go service... ");
    match go_client.health().await {
        Ok(true) => println!("{}", "OK".green()),
        _ => {
            println!("{}", "FAILED".red());
            return Err(anyhow::anyhow!("Go service not available"));
        }
    }

    println!();

    // Run benchmarks
    let tests = vec![
        ("GET", TestType::Get),
        ("SET", TestType::Set),
        ("Mixed (70/30)", TestType::Mixed),
        ("MSET (10 keys)", TestType::MSet(10)),
        ("MGET (10 keys)", TestType::MGet(10)),
        ("INCR", TestType::Incr),
    ];

    let mut all_results = Vec::new();

    for (name, test_type) in tests {
        println!("{}", format!("Running {} benchmark...", name).yellow());

        // Benchmark Rust service
        print!("  Rust: ");
        let rust_result = run_benchmark(
            &rust_client,
            "Rust",
            &cli,
            test_type.clone(),
        ).await?;
        println!(
            "RPS: {:.0}, P99: {:.2}ms",
            rust_result.metrics.rps,
            rust_result.metrics.latency_p99_us as f64 / 1000.0
        );

        // Benchmark Go service
        print!("  Go:   ");
        let go_result = run_benchmark(
            &go_client,
            "Go",
            &cli,
            test_type,
        ).await?;
        println!(
            "RPS: {:.0}, P99: {:.2}ms",
            go_result.metrics.rps,
            go_result.metrics.latency_p99_us as f64 / 1000.0
        );

        let comparison = compare_metrics(&rust_result, &go_result);
        all_results.push((name.to_string(), ComparisonResult {
            rust: rust_result,
            go: go_result,
            comparison,
        }));

        println!();
    }

    // Print results
    match cli.format.as_str() {
        "json" => print_json_results(&all_results)?,
        "csv" => print_csv_results(&all_results),
        _ => print_text_results(&all_results),
    }

    // Save results
    save_results(&all_results)?;

    Ok(())
}

#[derive(Clone)]
enum TestType {
    Get,
    Set,
    Mixed,
    MSet(usize),
    MGet(usize),
    Incr,
}

async fn run_benchmark(
    client: &LoadTestClient,
    name: &str,
    cli: &Cli,
    test_type: TestType,
) -> Result<ServiceBenchmark> {
    let config = LoadTestConfig {
        concurrency: cli.concurrency,
        duration: Duration::from_secs(cli.duration),
        target_rps: cli.rps,
        warmup: Duration::from_secs(cli.warmup),
        monitor_interval: Duration::from_secs(2),
        ..Default::default()
    };

    let runner = LoadTestRunner::new(client.clone(), config, name);
    let metrics = runner.metrics();

    match test_type {
        TestType::Get => {
            // Pre-populate some keys
            for i in 0..1000 {
                let _ = client.set_string(&format!("bench_key_{}", i), "test_value", Some(3600)).await;
            }

            runner.run(|c| async move {
                let key = format!("bench_key_{}", rand::random::<u64>() % 1000);
                c.get_string(&key).await?;
                Ok(())
            }).await?;
        }
        TestType::Set => {
            runner.run(|c| async move {
                let key = format!("bench_key_{}", rand::random::<u64>() % 10000);
                c.set_string(&key, "benchmark_value", Some(300)).await
            }).await?;
        }
        TestType::Mixed => {
            // Pre-populate
            for i in 0..1000 {
                let _ = client.set_string(&format!("bench_key_{}", i), "test_value", Some(3600)).await;
            }

            runner.run(|c| async move {
                let key = format!("bench_key_{}", rand::random::<u64>() % 1000);
                if rand::random::<f64>() < 0.7 {
                    c.get_string(&key).await?;
                } else {
                    c.set_string(&key, "updated_value", Some(300)).await?;
                }
                Ok(())
            }).await?;
        }
        TestType::MSet(batch_size) => {
            runner.run(move |c| async move {
                let base = rand::random::<u64>() % 10000;
                let pairs: HashMap<String, String> = (0..batch_size)
                    .map(|i| (format!("mset_{}_{}", base, i), format!("value_{}", i)))
                    .collect();
                c.mset(pairs).await
            }).await?;
        }
        TestType::MGet(batch_size) => {
            // Pre-populate
            for i in 0..100 {
                let pairs: HashMap<String, String> = (0..batch_size)
                    .map(|j| (format!("mget_{}_{}", i, j), format!("value_{}", j)))
                    .collect();
                let _ = client.mset(pairs).await;
            }

            runner.run(move |c| async move {
                let base = rand::random::<u64>() % 100;
                let keys: Vec<String> = (0..batch_size)
                    .map(|i| format!("mget_{}_{}", base, i))
                    .collect();
                c.mget(keys).await?;
                Ok(())
            }).await?;
        }
        TestType::Incr => {
            // Initialize counters
            for i in 0..100 {
                let _ = client.set_string(&format!("counter_{}", i), "0", None).await;
            }

            runner.run(|c| async move {
                let key = format!("counter_{}", rand::random::<u64>() % 100);
                c.incr(&key, Some(1)).await?;
                Ok(())
            }).await?;
        }
    }

    let summary = metrics.summary();

    Ok(ServiceBenchmark {
        name: name.to_string(),
        url: client.base_url().to_string(),
        metrics: summary,
        container_stats: None, // TODO: Implement container stats via docker API
    })
}

fn compare_metrics(rust: &ServiceBenchmark, go: &ServiceBenchmark) -> MetricsComparison {
    let rps_diff = if go.metrics.rps > 0.0 {
        ((rust.metrics.rps - go.metrics.rps) / go.metrics.rps) * 100.0
    } else {
        0.0
    };

    let p50_diff = if go.metrics.latency_p50_us > 0 {
        ((go.metrics.latency_p50_us as f64 - rust.metrics.latency_p50_us as f64)
            / go.metrics.latency_p50_us as f64) * 100.0
    } else {
        0.0
    };

    let p99_diff = if go.metrics.latency_p99_us > 0 {
        ((go.metrics.latency_p99_us as f64 - rust.metrics.latency_p99_us as f64)
            / go.metrics.latency_p99_us as f64) * 100.0
    } else {
        0.0
    };

    let memory_diff = if go.metrics.avg_memory_mb > 0.0 {
        ((go.metrics.avg_memory_mb - rust.metrics.avg_memory_mb)
            / go.metrics.avg_memory_mb) * 100.0
    } else {
        0.0
    };

    let cpu_diff = if go.metrics.avg_cpu_percent > 0.0 {
        ((go.metrics.avg_cpu_percent as f64 - rust.metrics.avg_cpu_percent as f64)
            / go.metrics.avg_cpu_percent as f64) * 100.0
    } else {
        0.0
    };

    let mut winners = HashMap::new();
    winners.insert("rps".to_string(), if rps_diff > 0.0 { "Rust" } else { "Go" }.to_string());
    winners.insert("p50".to_string(), if p50_diff > 0.0 { "Rust" } else { "Go" }.to_string());
    winners.insert("p99".to_string(), if p99_diff > 0.0 { "Rust" } else { "Go" }.to_string());
    winners.insert("memory".to_string(), if memory_diff > 0.0 { "Rust" } else { "Go" }.to_string());
    winners.insert("cpu".to_string(), if cpu_diff > 0.0 { "Rust" } else { "Go" }.to_string());

    MetricsComparison {
        rps_diff_percent: rps_diff,
        p50_diff_percent: p50_diff,
        p99_diff_percent: p99_diff,
        memory_diff_percent: memory_diff,
        cpu_diff_percent: cpu_diff,
        winners,
    }
}

fn print_text_results(results: &[(String, ComparisonResult)]) {
    println!("{}", "=".repeat(70).blue());
    println!("{}", " Benchmark Comparison Results ".blue().bold());
    println!("{}", "=".repeat(70).blue());
    println!();

    // Header
    println!(
        "{:<20} {:>12} {:>12} {:>12} {:>12}",
        "Test".bold(),
        "Rust RPS".bold(),
        "Go RPS".bold(),
        "Rust P99".bold(),
        "Go P99".bold()
    );
    println!("{:<20} {:>12} {:>12} {:>12} {:>12}", "", "", "", "(ms)", "(ms)");
    println!("{}", "-".repeat(70));

    for (name, result) in results {
        let rps_winner = if result.comparison.rps_diff_percent > 0.0 {
            format!("{:.0}", result.rust.metrics.rps).green()
        } else {
            format!("{:.0}", result.rust.metrics.rps).normal()
        };

        let go_rps = if result.comparison.rps_diff_percent <= 0.0 {
            format!("{:.0}", result.go.metrics.rps).green()
        } else {
            format!("{:.0}", result.go.metrics.rps).normal()
        };

        let rust_p99_ms = result.rust.metrics.latency_p99_us as f64 / 1000.0;
        let go_p99_ms = result.go.metrics.latency_p99_us as f64 / 1000.0;

        let p99_winner = if result.comparison.p99_diff_percent > 0.0 {
            format!("{:.2}", rust_p99_ms).green()
        } else {
            format!("{:.2}", rust_p99_ms).normal()
        };

        let go_p99 = if result.comparison.p99_diff_percent <= 0.0 {
            format!("{:.2}", go_p99_ms).green()
        } else {
            format!("{:.2}", go_p99_ms).normal()
        };

        println!(
            "{:<20} {:>12} {:>12} {:>12} {:>12}",
            name, rps_winner, go_rps, p99_winner, go_p99
        );
    }

    println!();
    println!("{}", "Summary:".yellow().bold());

    let rust_throughput_wins = results.iter()
        .filter(|(_, r)| r.comparison.rps_diff_percent > 0.0)
        .count();

    println!(
        "  Throughput: Rust wins {}/{} tests",
        rust_throughput_wins,
        results.len()
    );

    let rust_latency_wins = results.iter()
        .filter(|(_, r)| r.comparison.p99_diff_percent > 0.0)
        .count();

    println!(
        "  Latency:    Rust wins {}/{} tests",
        rust_latency_wins,
        results.len()
    );
}

fn print_json_results(results: &[(String, ComparisonResult)]) -> Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    println!("{}", json);
    Ok(())
}

fn print_csv_results(results: &[(String, ComparisonResult)]) {
    println!("test,rust_rps,go_rps,rps_diff%,rust_p50_ms,go_p50_ms,p50_diff%,rust_p99_ms,go_p99_ms,p99_diff%");

    for (name, result) in results {
        println!(
            "{},{:.0},{:.0},{:.1},{:.2},{:.2},{:.1},{:.2},{:.2},{:.1}",
            name,
            result.rust.metrics.rps,
            result.go.metrics.rps,
            result.comparison.rps_diff_percent,
            result.rust.metrics.latency_p50_us as f64 / 1000.0,
            result.go.metrics.latency_p50_us as f64 / 1000.0,
            result.comparison.p50_diff_percent,
            result.rust.metrics.latency_p99_us as f64 / 1000.0,
            result.go.metrics.latency_p99_us as f64 / 1000.0,
            result.comparison.p99_diff_percent,
        );
    }
}

fn save_results(results: &[(String, ComparisonResult)]) -> Result<()> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("benchmark_comparison_{}.json", timestamp);

    let json = serde_json::to_string_pretty(results)?;
    std::fs::write(&filename, json)?;

    println!("\nResults saved to: {}", filename.cyan());
    Ok(())
}
