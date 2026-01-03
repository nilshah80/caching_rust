//! Health Check Routes
//!
//! Endpoints for liveness and readiness probes.

use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::shared::app_state::AppState;
use crate::infrastructure::redis::connection::PoolStats;
use crate::infrastructure::redis::capabilities::RedisCapabilities;

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
/// Returns 200 if the service is ready to receive traffic.
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
async fn readiness(State(state): State<AppState>) -> Json<ReadinessResponse> {
    let pool_stats = state.pool.get_stats();

    // Check if we can get a connection
    let connected = state.pool.get().await.is_ok();

    Json(ReadinessResponse {
        status: if connected { "ready" } else { "not_ready" }.to_string(),
        redis: RedisHealthStatus {
            connected,
            pool: pool_stats,
        },
        capabilities: (*state.capabilities).clone(),
    })
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
        let (state, _, _) = test_state();
        let response = readiness(State(state)).await;
        assert!(matches!(response.0.status.as_str(), "ready" | "not_ready"));
    }
}
