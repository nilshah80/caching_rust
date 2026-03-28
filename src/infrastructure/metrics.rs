//! Prometheus Metrics
//!
//! Application metrics exported in Prometheus text format.
//! Uses the `metrics` facade crate with a Prometheus exporter backend.

use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::time::Duration;

/// Install the Prometheus metrics recorder and return a handle for rendering.
///
/// Must be called exactly once at startup, before any metrics are recorded.
///
/// # Errors
///
/// Returns an error if the recorder has already been installed.
pub fn install_prometheus_recorder()
-> Result<PrometheusHandle, metrics_exporter_prometheus::BuildError> {
    PrometheusBuilder::new().install_recorder()
}

/// Record an HTTP request completing.
pub fn record_http_request(method: &str, path: &str, status: u16, duration: Duration) {
    let labels = [
        ("method", method.to_string()),
        ("path", path.to_string()),
        ("status", status.to_string()),
    ];
    counter!("http_requests_total", &labels).increment(1);
    histogram!("http_request_duration_seconds", &labels).record(duration.as_secs_f64());
}

/// Record current pool stats as gauges.
pub fn record_pool_stats(
    size: usize,
    available: usize,
    max_size: usize,
    waiting: usize,
    failed_checkouts: u64,
) {
    gauge!("redis_pool_connections_current").set(size as f64);
    gauge!("redis_pool_connections_available").set(available as f64);
    gauge!("redis_pool_connections_max").set(max_size as f64);
    gauge!("redis_pool_connections_waiting").set(waiting as f64);
    counter!("redis_pool_checkout_failures_total").absolute(failed_checkouts);
}

/// Record current pub/sub stats as gauges.
#[allow(clippy::too_many_arguments)]
pub fn record_pubsub_stats(
    active: usize,
    max: usize,
    total_created: u64,
    total_messages: u64,
    errors: u64,
) {
    gauge!("redis_pubsub_subscriptions_active").set(active as f64);
    gauge!("redis_pubsub_subscriptions_max").set(max as f64);
    counter!("redis_pubsub_subscriptions_created_total").absolute(total_created);
    counter!("redis_pubsub_messages_total").absolute(total_messages);
    counter!("redis_pubsub_errors_total").absolute(errors);
}

#[cfg(test)]
mod tests {
    use super::*;

    // These functions call the metrics facade. Without a recorder installed,
    // the calls are no-ops (metrics crate handles this gracefully).
    // We test that they don't panic.

    #[test]
    fn test_record_http_request_does_not_panic() {
        record_http_request("GET", "/health", 200, Duration::from_millis(5));
        record_http_request("POST", "/api/v1/strings/:key", 404, Duration::from_secs(1));
    }

    #[test]
    fn test_record_pool_stats_does_not_panic() {
        record_pool_stats(5, 3, 10, 0, 0);
        record_pool_stats(10, 0, 10, 5, 42);
    }

    #[test]
    fn test_record_pubsub_stats_does_not_panic() {
        record_pubsub_stats(10, 100, 50, 1000, 3);
        record_pubsub_stats(0, 100, 0, 0, 0);
    }
}
