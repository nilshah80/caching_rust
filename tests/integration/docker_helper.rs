//! Docker container helpers for external integration tests.
//!
//! Mirrors the skip/fail behavior of `src/test_support.rs`:
//! - By default, returns `None` when Docker is unavailable (test skips).
//! - When `REQUIRE_DOCKER=1`, panics so CI pipelines don't silently skip.

#![allow(dead_code)] // create_pool_with_tag is only used by Redis-8.6 phase11 tests.

use std::sync::Arc;

use testcontainers::ContainerAsync;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::redis::{REDIS_PORT, Redis};

use redis_caching_service::infrastructure::redis::connection::InstrumentedPool;

/// Start a Redis container and return a pool, or `None` if Docker is unavailable.
///
/// Set `REQUIRE_DOCKER=1` to panic instead of skipping.
pub async fn create_pool() -> Option<(ContainerAsync<Redis>, Arc<InstrumentedPool>)> {
    let container = match Redis::default().start().await {
        Ok(c) => c,
        Err(err) => return handle_docker_unavailable(&err),
    };
    let host = match container.get_host().await {
        Ok(h) => h,
        Err(err) => return handle_docker_unavailable(&err),
    };
    let port = match container.get_host_port_ipv4(REDIS_PORT).await {
        Ok(p) => p,
        Err(err) => return handle_docker_unavailable(&err),
    };
    let url = format!("redis://{host}:{port}");
    let pool = Arc::new(InstrumentedPool::new_for_tests_with_url(&url).unwrap());
    Some((container, pool))
}

/// Start a Redis container of an explicit tag (e.g. `"8.6"`) and return a pool,
/// or `None` if Docker is unavailable or the image cannot be pulled.
///
/// Useful for tests that depend on features that only exist in a specific
/// Redis release (e.g. HOTKEYS in Redis 8.6+). The `testcontainers_modules`
/// `Redis` image is reused so the "Ready to accept connections" stdout wait
/// condition still applies; only the tag is overridden.
pub async fn create_pool_with_tag(
    tag: &str,
) -> Option<(ContainerAsync<Redis>, Arc<InstrumentedPool>)> {
    let container = match Redis::default().with_tag(tag).start().await {
        Ok(c) => c,
        Err(err) => return handle_docker_unavailable(&err),
    };
    let host = match container.get_host().await {
        Ok(h) => h,
        Err(err) => return handle_docker_unavailable(&err),
    };
    let port = match container.get_host_port_ipv4(REDIS_PORT).await {
        Ok(p) => p,
        Err(err) => return handle_docker_unavailable(&err),
    };
    let url = format!("redis://{host}:{port}");
    let pool = Arc::new(InstrumentedPool::new_for_tests_with_url(&url).unwrap());
    Some((container, pool))
}

fn handle_docker_unavailable<T>(err: &dyn std::fmt::Display) -> Option<T> {
    if std::env::var("REQUIRE_DOCKER").as_deref() == Ok("1") {
        panic!("Docker-dependent Redis test failed and REQUIRE_DOCKER=1 is set: {err}");
    }
    // Write directly to the stderr fd via std::io::Write, bypassing the test
    // harness's macro-level capture so the warning is visible even for passing
    // tests (eprintln! is swallowed by default).
    use std::io::Write;
    let _ = writeln!(
        std::io::stderr(),
        "WARNING: Skipping Docker-dependent Redis test (set REQUIRE_DOCKER=1 to fail instead): {err}"
    );
    None
}
