//! Load Test Runner
//!
//! CLI for running load tests against the caching service.

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::time::Instant;

mod client;
mod metrics;
mod runner;
mod tests;

use tests::{redis_stress, service_stress};

#[derive(Parser)]
#[command(name = "load_test")]
#[command(about = "Load testing tool for Redis caching service")]
struct Cli {
    /// Base URL of the caching service
    #[arg(short, long, default_value = "http://localhost:8080")]
    url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all service stress tests
    Service,
    /// Run all Redis stress tests
    Redis,
    /// Run all tests
    All,
    /// Run quick test suite (subset of tests)
    Quick,
    /// Run specific test
    Test {
        /// Test name
        #[arg(value_name = "NAME")]
        name: String,
    },
    /// List available tests
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let start = Instant::now();

    println!("{}", "=".repeat(60).blue());
    println!("{}", " Redis Caching Service Load Tests ".blue().bold());
    println!("{}", "=".repeat(60).blue());
    println!("Target: {}", cli.url.cyan());
    println!();

    // Check service health
    let client = client::LoadTestClient::new(&cli.url)?;
    match client.health().await {
        Ok(true) => println!("{} Service is healthy\n", "✓".green()),
        Ok(false) => {
            println!("{} Service health check failed", "✗".red());
            return Ok(());
        }
        Err(e) => {
            println!("{} Cannot connect to service: {}", "✗".red(), e);
            return Ok(());
        }
    }

    let results = match cli.command {
        Commands::Service => {
            println!("{}", "Running Service Stress Tests...".yellow());
            service_stress::run_all(&cli.url).await
        }
        Commands::Redis => {
            println!("{}", "Running Redis Stress Tests...".yellow());
            redis_stress::run_all(&cli.url).await
        }
        Commands::All => {
            println!("{}", "Running All Tests...".yellow());
            let mut results = service_stress::run_all(&cli.url).await;
            results.extend(redis_stress::run_all(&cli.url).await);
            results
        }
        Commands::Quick => {
            println!("{}", "Running Quick Test Suite...".yellow());
            let mut results = Vec::new();

            // Run subset of quick tests
            if let Ok(r) = service_stress::test_spike_load(&cli.url).await {
                results.push(r);
            }
            if let Ok(r) = service_stress::test_batch_operations(&cli.url).await {
                results.push(r);
            }
            if let Ok(r) = redis_stress::test_key_expiration(&cli.url).await {
                results.push(r);
            }

            results
        }
        Commands::Test { name } => {
            println!("Running test: {}", name.cyan());
            let mut results = Vec::new();

            match name.to_lowercase().as_str() {
                "spike" | "spikeload" => {
                    if let Ok(r) = service_stress::test_spike_load(&cli.url).await {
                        results.push(r);
                    }
                }
                "overload" => {
                    if let Ok(r) = service_stress::test_overload(&cli.url).await {
                        results.push(r);
                    }
                }
                "memory" | "memorystress" => {
                    if let Ok(r) = service_stress::test_memory_stress(&cli.url).await {
                        results.push(r);
                    }
                }
                "batch" | "batchoperations" => {
                    if let Ok(r) = service_stress::test_batch_operations(&cli.url).await {
                        results.push(r);
                    }
                }
                "hotkey" => {
                    if let Ok(r) = service_stress::test_hot_key(&cli.url).await {
                        results.push(r);
                    }
                }
                "largedataset" | "dataset" => {
                    if let Ok(r) = redis_stress::test_large_dataset(&cli.url).await {
                        results.push(r);
                    }
                }
                "connection" | "connections" => {
                    if let Ok(r) = redis_stress::test_connection_stress(&cli.url).await {
                        results.push(r);
                    }
                }
                "expiration" | "ttl" => {
                    if let Ok(r) = redis_stress::test_key_expiration(&cli.url).await {
                        results.push(r);
                    }
                }
                "pressure" | "memorypressure" => {
                    if let Ok(r) = redis_stress::test_memory_pressure(&cli.url).await {
                        results.push(r);
                    }
                }
                "pipeline" => {
                    if let Ok(r) = redis_stress::test_command_pipeline(&cli.url).await {
                        results.push(r);
                    }
                }
                _ => {
                    println!("{} Unknown test: {}", "✗".red(), name);
                    println!("Use 'load_test list' to see available tests");
                    return Ok(());
                }
            }

            results
        }
        Commands::List => {
            println!("\n{}", "Available Tests:".yellow().bold());
            println!("\n{}", "Service Stress Tests:".cyan());
            println!("  spike        - Spike load test (100 → 1000 RPS)");
            println!("  overload     - Progressive overload test");
            println!("  memory       - Memory stress with large values");
            println!("  batch        - Batch operations (MSET/MGET)");
            println!("  hotkey       - Hot key contention test");
            println!("\n{}", "Redis Stress Tests:".cyan());
            println!("  dataset      - Large dataset test (10K-1M keys)");
            println!("  connection   - Connection stress test");
            println!("  expiration   - Key expiration under load");
            println!("  pressure     - Memory pressure test");
            println!("  pipeline     - Command pipeline test");
            return Ok(());
        }
    };

    // Print summary
    println!("\n{}", "=".repeat(60).blue());
    println!("{}", " Test Results Summary ".blue().bold());
    println!("{}", "=".repeat(60).blue());

    let mut passed = 0;
    let mut failed = 0;

    for result in &results {
        let status = if result.passed {
            passed += 1;
            "PASSED".green()
        } else {
            failed += 1;
            "FAILED".red()
        };

        println!("\n{}: {}", result.name.cyan(), status);
        println!("  {}", result.message);
        println!("  RPS: {:.0}, P99: {:.2}ms, Errors: {:.2}%",
            result.summary.rps,
            result.summary.latency_p99_us as f64 / 1000.0,
            result.summary.error_rate * 100.0
        );
    }

    println!("\n{}", "-".repeat(60));
    println!(
        "Total: {} tests, {} passed, {} failed",
        results.len(),
        passed.to_string().green(),
        if failed > 0 { failed.to_string().red() } else { failed.to_string().green() }
    );
    println!("Duration: {:.2}s", start.elapsed().as_secs_f64());

    // Save results to JSON
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let results_file = format!("load_test_results_{}.json", timestamp);

    let summaries: Vec<_> = results.iter().map(|r| &r.summary).collect();
    if let Ok(json) = serde_json::to_string_pretty(&summaries) {
        if std::fs::write(&results_file, json).is_ok() {
            println!("Results saved to: {}", results_file.cyan());
        }
    }

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}
