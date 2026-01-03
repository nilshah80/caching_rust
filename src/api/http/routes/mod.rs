//! HTTP Routes
//!
//! Route definitions for all API endpoints.

mod admin;
mod health;
mod keys;
mod openapi;
mod strings;
mod hashes;

pub use admin::admin_routes;
pub use health::health_routes;
pub use keys::key_routes;
pub use openapi::openapi_routes;
pub use strings::string_routes;
pub use hashes::hash_routes;

use axum::Router;
use crate::shared::app_state::AppState;

/// Build the complete API router based on detected capabilities
pub fn build_router(state: AppState) -> Router {
    let _capabilities = state.capabilities.clone();

    let router = Router::new()
        // Always available - health checks
        .merge(health_routes())
        // Always available - core Redis types
        .merge(string_routes())
        // Hash operations
        .merge(hash_routes())
        // Key management operations
        .merge(key_routes())
        // Admin endpoints
        .merge(admin_routes())
        // OpenAPI documentation
        .merge(openapi_routes());

    // TODO: Add more routes as they are implemented
    // Conditionally add routes based on capabilities:
    // - hash_routes()
    // - list_routes()
    // - set_routes()
    // - sorted_set_routes()
    // - stream_routes() (Redis 5.0+)
    // - json_routes() (requires RedisJSON module)
    // - search_routes() (requires RediSearch module)
    // - bloom_routes() (requires RedisBloom module)
    // - timeseries_routes() (requires RedisTimeSeries module)
    // - functions_routes() (Redis 7.0+)

    router.with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_build_router_health() {
        let (state, _, _, _) = test_state();
        let app = build_router(state);
        let response = app
            .oneshot(Request::builder().uri("/health").body(axum::body::Body::empty()).unwrap())
            .await
            .expect("response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
