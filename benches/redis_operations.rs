//! Redis Operations Benchmarks
//!
//! Performance benchmarks for Redis caching operations using the service layer.
//!
//! Requires a running Redis instance. Connect to `redis://localhost:6379` by default,
//! or set the `REDIS_URL` environment variable to override.
//!
//! Run with:
//!   cargo bench --features test-utils
//!
//! List benchmarks without running:
//!   cargo bench --features test-utils -- --list

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::wildcard_imports
)]

use std::sync::{Arc, OnceLock};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;

use redis_caching_service::application::services::{
    HashService, ListService, SortedSetService, StringService,
};
use redis_caching_service::domain::repositories::ScoredMember;
use redis_caching_service::infrastructure::redis::connection::InstrumentedPool;

// ---------------------------------------------------------------------------
// Shared Redis connection pool (created once, reused across all benchmarks)
// ---------------------------------------------------------------------------

static POOL: OnceLock<Arc<InstrumentedPool>> = OnceLock::new();
static SETUP_RT: OnceLock<Runtime> = OnceLock::new();

fn setup_runtime() -> &'static Runtime {
    SETUP_RT.get_or_init(|| Runtime::new().expect("failed to create tokio runtime"))
}

static REDIS_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Try to create a pool connected to a real Redis instance.
/// Returns `None` if Redis is not reachable (prints a warning and skips).
fn get_pool() -> Option<Arc<InstrumentedPool>> {
    let available = *REDIS_AVAILABLE.get_or_init(|| {
        let url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let pool = match InstrumentedPool::new_for_tests_with_url(&url) {
            Ok(p) => Arc::new(p),
            Err(e) => {
                eprintln!("WARNING: Failed to build Redis pool ({e}). Benchmarks will be skipped.");
                return false;
            }
        };

        // Verify connectivity
        let p = pool.clone();
        let ok = setup_runtime().block_on(async move {
            let mut conn = match p.get().await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("WARNING: Redis not reachable ({e}). Benchmarks will be skipped. Start Redis or set REDIS_URL.");
                    return false;
                }
            };
            match redis::cmd("PING").query_async::<String>(&mut conn).await {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("WARNING: Redis PING failed ({e}). Benchmarks will be skipped.");
                    false
                }
            }
        });

        if ok {
            // Store pool for later use
            let _ = POOL.set(pool);
        }
        ok
    });

    if available { POOL.get().cloned() } else { None }
}

/// Get the shared pool or skip the benchmark gracefully.
macro_rules! require_redis {
    () => {
        match get_pool() {
            Some(p) => p,
            None => return,
        }
    };
}

/// Helper: create a new tokio runtime for each bench iteration group.
/// Criterion's `to_async` needs an owned runtime.
fn bench_rt() -> Runtime {
    Runtime::new().unwrap()
}

// ---------------------------------------------------------------------------
// String benchmarks
// ---------------------------------------------------------------------------

fn string_benchmarks(c: &mut Criterion) {
    let pool = require_redis!();
    let service = Arc::new(StringService::new(pool));
    let rt = setup_runtime();

    let mut group = c.benchmark_group("string");

    // SET single key
    {
        let svc = service.clone();
        group.bench_function("set", |b| {
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                async move {
                    svc.set(
                        "bench:str:key1",
                        "hello-world",
                        None,
                        None,
                        false,
                        false,
                        false,
                        false,
                    )
                    .await
                    .unwrap();
                }
            });
        });
    }

    // GET single key
    rt.block_on(async {
        service
            .set(
                "bench:str:get-key",
                "value-for-get",
                None,
                None,
                false,
                false,
                false,
                false,
            )
            .await
            .unwrap();
    });
    {
        let svc = service.clone();
        group.bench_function("get", |b| {
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                async move {
                    svc.get("bench:str:get-key").await.unwrap();
                }
            });
        });
    }

    // MSET batch of 10 keys
    {
        let svc = service.clone();
        group.bench_function("mset_10", |b| {
            let pairs: Vec<(String, String)> = (0..10)
                .map(|i| (format!("bench:str:mset:{i}"), format!("value-{i}")))
                .collect();
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                let pairs = pairs.clone();
                async move {
                    svc.mset(pairs).await.unwrap();
                }
            });
        });
    }

    // MGET batch of 10 keys
    rt.block_on(async {
        let pairs: Vec<(String, String)> = (0..10)
            .map(|i| (format!("bench:str:mget:{i}"), format!("value-{i}")))
            .collect();
        service.mset(pairs).await.unwrap();
    });
    {
        let svc = service.clone();
        group.bench_function("mget_10", |b| {
            let keys: Vec<String> = (0..10).map(|i| format!("bench:str:mget:{i}")).collect();
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                let keys = keys.clone();
                async move {
                    svc.mget(keys).await.unwrap();
                }
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Hash benchmarks
// ---------------------------------------------------------------------------

fn hash_benchmarks(c: &mut Criterion) {
    let pool = require_redis!();
    let service = Arc::new(HashService::new(pool));
    let rt = setup_runtime();

    let mut group = c.benchmark_group("hash");

    // HSET multiple fields
    {
        let svc = service.clone();
        group.bench_function("hset_5_fields", |b| {
            let pairs: Vec<(String, String)> = (0..5)
                .map(|i| (format!("field-{i}"), format!("value-{i}")))
                .collect();
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                let pairs = pairs.clone();
                async move {
                    svc.hset("bench:hash:key1", pairs).await.unwrap();
                }
            });
        });
    }

    // HGETALL
    rt.block_on(async {
        let pairs: Vec<(String, String)> = (0..5)
            .map(|i| (format!("field-{i}"), format!("value-{i}")))
            .collect();
        service.hset("bench:hash:getall", pairs).await.unwrap();
    });
    {
        let svc = service.clone();
        group.bench_function("hgetall_5_fields", |b| {
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                async move {
                    svc.hgetall("bench:hash:getall").await.unwrap();
                }
            });
        });
    }

    // HGET single field
    {
        let svc = service.clone();
        group.bench_function("hget_single", |b| {
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                async move {
                    svc.hget("bench:hash:getall", "field-0").await.unwrap();
                }
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// List benchmarks
// ---------------------------------------------------------------------------

fn list_benchmarks(c: &mut Criterion) {
    let pool = require_redis!();
    let service = Arc::new(ListService::new(pool));
    let rt = setup_runtime();

    let mut group = c.benchmark_group("list");

    // RPUSH single element
    {
        let svc = service.clone();
        group.bench_function("rpush_1", |b| {
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                async move {
                    svc.rpush("bench:list:rpush", vec!["item".to_string()])
                        .await
                        .unwrap();
                }
            });
        });
    }

    // RPUSH + LPOP cycle (push then pop to keep list bounded)
    rt.block_on(async {
        service
            .rpush("bench:list:cycle", vec!["seed".to_string()])
            .await
            .unwrap();
    });
    {
        let svc = service.clone();
        group.bench_function("rpush_lpop_cycle", |b| {
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                async move {
                    svc.rpush("bench:list:cycle", vec!["item".to_string()])
                        .await
                        .unwrap();
                    svc.lpop("bench:list:cycle", Some(1)).await.unwrap();
                }
            });
        });
    }

    // RPUSH batch of 10 elements
    {
        let svc = service.clone();
        group.bench_function("rpush_10", |b| {
            let values: Vec<String> = (0..10).map(|i| format!("item-{i}")).collect();
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                let values = values.clone();
                async move {
                    svc.rpush("bench:list:rpush10", values).await.unwrap();
                }
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Sorted set benchmarks
// ---------------------------------------------------------------------------

fn sorted_set_benchmarks(c: &mut Criterion) {
    let pool = require_redis!();
    let service = Arc::new(SortedSetService::new(pool));
    let rt = setup_runtime();

    let mut group = c.benchmark_group("sorted_set");

    // ZADD single member
    {
        let svc = service.clone();
        group.bench_function("zadd_1", |b| {
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                let member = ScoredMember {
                    member: "member-bench".to_string(),
                    score: 42.0,
                };
                async move {
                    svc.zadd("bench:zset:zadd", vec![member], None)
                        .await
                        .unwrap();
                }
            });
        });
    }

    // ZADD batch of 10 members
    {
        let svc = service.clone();
        group.bench_function("zadd_10", |b| {
            let members: Vec<ScoredMember> = (0..10)
                .map(|i| ScoredMember {
                    member: format!("member-{i}"),
                    score: i as f64,
                })
                .collect();
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                let members = members.clone();
                async move {
                    svc.zadd("bench:zset:zadd10", members, None).await.unwrap();
                }
            });
        });
    }

    // ZRANGE after populating
    rt.block_on(async {
        let members: Vec<ScoredMember> = (0..20)
            .map(|i| ScoredMember {
                member: format!("member-{i}"),
                score: i as f64,
            })
            .collect();
        service
            .zadd("bench:zset:zrange", members, None)
            .await
            .unwrap();
    });
    {
        let svc = service.clone();
        group.bench_function("zrange_20", |b| {
            b.to_async(bench_rt()).iter(|| {
                let svc = svc.clone();
                async move {
                    svc.zrange("bench:zset:zrange", 0, -1, None).await.unwrap();
                }
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Connection pool benchmarks
// ---------------------------------------------------------------------------

fn pool_benchmarks(c: &mut Criterion) {
    let pool = require_redis!();

    let mut group = c.benchmark_group("pool");

    // Pool checkout + release (just get a connection and drop it)
    {
        let p = pool.clone();
        group.bench_function("checkout_release", |b| {
            b.to_async(bench_rt()).iter(|| {
                let p = p.clone();
                async move {
                    let _conn = p.get().await.unwrap();
                    // connection returned to pool on drop
                }
            });
        });
    }

    // Pool checkout + PING (minimal round-trip)
    {
        let p = pool.clone();
        group.bench_function("checkout_ping", |b| {
            b.to_async(bench_rt()).iter(|| {
                let p = p.clone();
                async move {
                    let mut conn = p.get().await.unwrap();
                    let _: String = redis::cmd("PING").query_async(&mut conn).await.unwrap();
                }
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Parameterized benchmarks (scaling with input size)
// ---------------------------------------------------------------------------

fn scaling_benchmarks(c: &mut Criterion) {
    let pool = require_redis!();
    let service = Arc::new(StringService::new(pool));
    let rt = setup_runtime();

    let mut group = c.benchmark_group("scaling_mset_mget");

    for size in [1, 10, 50, 100] {
        // MSET with varying batch sizes
        {
            let svc = service.clone();
            group.bench_with_input(BenchmarkId::new("mset", size), &size, move |b, &size| {
                let pairs: Vec<(String, String)> = (0..size)
                    .map(|i| (format!("bench:scale:mset:{size}:{i}"), format!("v{i}")))
                    .collect();
                b.to_async(bench_rt()).iter(|| {
                    let svc = svc.clone();
                    let pairs = pairs.clone();
                    async move {
                        svc.mset(pairs).await.unwrap();
                    }
                });
            });
        }

        // MGET with varying batch sizes (pre-populate first)
        rt.block_on(async {
            let pairs: Vec<(String, String)> = (0..size)
                .map(|i| (format!("bench:scale:mget:{size}:{i}"), format!("v{i}")))
                .collect();
            service.mset(pairs).await.unwrap();
        });
        {
            let svc = service.clone();
            group.bench_with_input(BenchmarkId::new("mget", size), &size, move |b, &size| {
                let keys: Vec<String> = (0..size)
                    .map(|i| format!("bench:scale:mget:{size}:{i}"))
                    .collect();
                b.to_async(bench_rt()).iter(|| {
                    let svc = svc.clone();
                    let keys = keys.clone();
                    async move {
                        svc.mget(keys).await.unwrap();
                    }
                });
            });
        }
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// No-op benchmark (always runs, ensures criterion exits 0 even without Redis)
// ---------------------------------------------------------------------------

fn noop_benchmark(c: &mut Criterion) {
    c.bench_function("noop_baseline", |b| b.iter(|| 1 + 1));
}

// ---------------------------------------------------------------------------
// Criterion configuration and main
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    noop_benchmark,
    string_benchmarks,
    hash_benchmarks,
    list_benchmarks,
    sorted_set_benchmarks,
    pool_benchmarks,
    scaling_benchmarks,
);
criterion_main!(benches);
