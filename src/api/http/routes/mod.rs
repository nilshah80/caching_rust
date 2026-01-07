//! HTTP Routes
//!
//! Route definitions for all API endpoints.

mod admin;
mod bitmaps;
mod bloom;
mod hashes;
mod health;
mod json;
mod keys;
mod lists;
mod openapi;
mod probabilistic;
mod search;
mod sets;
mod sorted_sets;
mod streams;
mod strings;

pub use admin::admin_routes;
pub use bitmaps::bitmap_routes;
pub use bloom::bloom_routes;
pub use hashes::hash_routes;
pub use health::health_routes;
pub use json::json_routes;
pub use keys::key_routes;
pub use lists::list_routes;
pub use openapi::openapi_routes;
pub use probabilistic::{cms_routes, hyperloglog_routes, topk_routes};
pub use search::search_routes;
pub use sets::set_routes;
pub use sorted_sets::sorted_set_routes;
pub use streams::{stream_admin_routes, stream_routes};
pub use strings::string_routes;

use axum::Router;
use crate::shared::app_state::AppState;

/// Build the complete API router based on detected capabilities
pub fn build_router(state: AppState) -> Router {
    let capabilities = state.capabilities.clone();

    let mut router = Router::new()
        // Always available - health checks
        .merge(health_routes())
        // Always available - core Redis types
        .merge(string_routes())
        // Hash operations
        .merge(hash_routes())
        // List operations
        .merge(list_routes())
        // Set operations
        .merge(set_routes())
        // Sorted Set operations
        .merge(sorted_set_routes())
        // Bitmap operations (core Redis)
        .merge(bitmap_routes())
        // Key management operations
        .merge(key_routes())
        // Admin endpoints
        .merge(admin_routes())
        // OpenAPI documentation
        .merge(openapi_routes());

    // Conditionally add stream routes (Redis 5.0+)
    if capabilities.features.streams {
        router = router
            .merge(stream_routes())
            .merge(stream_admin_routes());
    }

    // Conditionally add JSON routes (requires RedisJSON module)
    if capabilities.modules.json {
        router = router.merge(json_routes());
    }

    // Conditionally add Search routes (requires RediSearch module)
    if capabilities.modules.search {
        router = router.merge(search_routes());
    }

    // Conditionally add Bloom routes (requires RedisBloom module)
    if capabilities.modules.bloom {
        router = router
            .merge(bloom_routes())
            // CMS and Top-K are part of RedisBloom module
            .merge(cms_routes())
            .merge(topk_routes());
    }

    // HyperLogLog is always available (core Redis)
    router = router.merge(hyperloglog_routes());

    // TODO: Add more routes as they are implemented
    // Conditionally add routes based on capabilities:
    // - timeseries_routes() (requires RedisTimeSeries module)
    // - functions_routes() (Redis 7.0+)

    router.with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::redis::capabilities::RedisCapabilities;
    use crate::test_support::{test_state_with_bloom_repo, test_state_with_json_repo, test_state_with_search_repo};
    use axum::http::Request;
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_build_router_health() {
        let (state, _) = test_state_with_json_repo();
        let app = build_router(state);
        let response = app
            .oneshot(Request::builder().uri("/health").body(axum::body::Body::empty()).unwrap())
            .await
            .expect("response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_build_router_with_json_routes() {
        let (mut state, _) = test_state_with_json_repo();
        let mut capabilities = RedisCapabilities::default_capabilities();
        capabilities.modules.json = true;
        state.capabilities = Arc::new(capabilities);

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/json/key?path=$")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_build_router_with_search_routes() {
        let (mut state, _) = test_state_with_search_repo();
        let mut capabilities = RedisCapabilities::default_capabilities();
        capabilities.modules.search = true;
        state.capabilities = Arc::new(capabilities);

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/search/indices")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_build_router_with_bloom_routes() {
        let (mut state, _) = test_state_with_bloom_repo();
        let mut capabilities = RedisCapabilities::default_capabilities();
        capabilities.modules.bloom = true;
        state.capabilities = Arc::new(capabilities);

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/bloom/test-key")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
