//! Health Check Routes
//!
//! Endpoints for liveness and readiness probes.

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;
use utoipa::ToSchema;

use crate::infrastructure::metrics::{record_pool_stats, record_pubsub_stats};
use crate::infrastructure::redis::capabilities::RedisCapabilities;
use crate::infrastructure::redis::connection::PoolStats;
use crate::shared::app_state::AppState;

/// Health check response
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Readiness check response
#[derive(Debug, Serialize, ToSchema)]
pub struct ReadinessResponse {
    pub status: String,
    pub redis: RedisHealthStatus,
    pub capabilities: RedisCapabilities,
}

/// Redis health status
#[derive(Debug, Serialize, ToSchema)]
pub struct RedisHealthStatus {
    pub connected: bool,
    pub pool: PoolStats,
}

/// Create health routes
pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/health/live", get(liveness))
        .route("/health/ready", get(readiness))
        .route("/metrics", get(prometheus_metrics))
}

/// Basic health check endpoint
///
/// Returns 200 if the service is running.
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "Health"
)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Liveness probe endpoint
///
/// Returns 200 if the service is alive. Used by Kubernetes.
#[utoipa::path(
    get,
    path = "/health/live",
    responses(
        (status = 200, description = "Service is alive", body = HealthResponse)
    ),
    tag = "Health"
)]
async fn liveness() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "alive".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Readiness probe endpoint
///
/// Returns 200 if the service is ready to receive traffic, or 503 if Redis is unavailable.
/// Checks Redis connectivity and returns capabilities.
#[utoipa::path(
    get,
    path = "/health/ready",
    responses(
        (status = 200, description = "Service is ready", body = ReadinessResponse),
        (status = 503, description = "Service is not ready")
    ),
    tag = "Health"
)]
async fn readiness(State(state): State<AppState>) -> (StatusCode, Json<ReadinessResponse>) {
    let pool_stats = state.pool.get_stats();

    // Check if we can get a connection
    let connected = state.pool.get().await.is_ok();

    let status = if connected {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(ReadinessResponse {
            status: if connected { "ready" } else { "not_ready" }.to_string(),
            redis: RedisHealthStatus {
                connected,
                pool: pool_stats,
            },
            capabilities: (*state.capabilities).clone(),
        }),
    )
}

/// Prometheus metrics endpoint
///
/// Returns metrics in Prometheus text exposition format.
/// Updates pool and pub/sub gauges on each scrape.
async fn prometheus_metrics(
    State(state): State<AppState>,
) -> (
    StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    String,
) {
    // Update pool gauges
    let pool = state.pool.get_stats();
    record_pool_stats(
        pool.size,
        pool.available,
        pool.max_size,
        pool.current_waiting,
        pool.failed_checkouts,
    );

    // Update pub/sub gauges
    let ps = state.pubsub_service.get_stats();
    record_pubsub_stats(
        ps.active_subscriptions,
        ps.max_subscriptions,
        ps.total_created,
        ps.total_messages,
        ps.errors,
    );

    let content_type = [(
        axum::http::header::CONTENT_TYPE,
        "text/plain; version=0.0.4; charset=utf-8",
    )];

    match &state.metrics_handle {
        Some(handle) => (StatusCode::OK, content_type, handle.render()),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            content_type,
            "metrics not configured".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state;

    #[tokio::test]
    async fn test_health_endpoints() {
        let health = health().await;
        assert_eq!(health.0.status, "healthy");

        let live = liveness().await;
        assert_eq!(live.0.status, "alive");
    }

    #[tokio::test]
    async fn test_readiness_endpoint() {
        let (state, _, _, _) = test_state();
        let (status, response) = readiness(State(state)).await;
        assert!(matches!(response.0.status.as_str(), "ready" | "not_ready"));
        assert_eq!(
            status,
            if response.0.redis.connected {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            }
        );
    }

    #[tokio::test]
    async fn test_prometheus_metrics_no_handle() {
        let (mut state, _, _, _) = test_state();
        state.metrics_handle = None;
        let (status, headers, body) = prometheus_metrics(State(state)).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(headers[0].1, "text/plain; version=0.0.4; charset=utf-8");
        assert_eq!(body, "metrics not configured");
    }

    #[tokio::test]
    async fn test_prometheus_metrics_with_handle() {
        let (mut state, _, _, _) = test_state();
        let handle = crate::infrastructure::metrics::install_prometheus_recorder().ok();
        state.metrics_handle = handle.map(std::sync::Arc::new);
        let (status, headers, body) = prometheus_metrics(State(state)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[0].1, "text/plain; version=0.0.4; charset=utf-8");
        // Body should contain at least some metric output (may be empty if no metrics recorded)
        assert!(body.is_empty() || body.contains("# TYPE") || body.contains("redis_pool"));
    }
}
