//! HTTP Routes
//!
//! Route definitions for all API endpoints.

mod admin;
mod bitmaps;
mod bloom;
mod cluster;
mod functions;
mod geo;
mod hashes;
mod health;
mod json;
mod keys;
mod lists;
mod openapi;
mod probabilistic;
mod pubsub;
mod scripting;
mod search;
mod sets;
mod sorted_sets;
mod streams;
mod strings;
mod timeseries;
mod transactions;

pub use admin::admin_routes;
pub use bitmaps::bitmap_routes;
pub use bloom::bloom_routes;
pub use cluster::cluster_routes;
pub use functions::functions_routes;
pub use geo::geo_routes;
pub use hashes::hash_routes;
pub use health::health_routes;
pub use json::json_routes;
pub use keys::key_routes;
pub use lists::list_routes;
pub use openapi::openapi_routes;
pub use probabilistic::{cms_routes, hyperloglog_routes, topk_routes};
pub use pubsub::pubsub_routes;
pub use scripting::scripting_routes;
pub use search::search_routes;
pub use sets::set_routes;
pub use sorted_sets::sorted_set_routes;
pub use streams::{stream_admin_routes, stream_routes};
pub use strings::string_routes;
pub use timeseries::timeseries_routes;
pub use transactions::transaction_routes;

use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use axum::Router;
use axum::routing::any;

/// Build operational routes that must be exempt from rate limiting
/// (health probes, metrics, liveness/readiness for Kubernetes).
pub fn operational_routes() -> Router<AppState> {
    health_routes()
}

/// Build the complete API router based on detected capabilities
pub fn build_router(state: AppState) -> Router {
    let capabilities = state.capabilities.clone();

    let mut router = Router::new()
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
        // Geo operations (core Redis)
        .merge(geo_routes())
        // Key management operations
        .merge(key_routes())
        // Admin endpoints
        .merge(admin_routes())
        // OpenAPI documentation (filtered by detected capabilities)
        .merge(openapi_routes(&capabilities));

    // Conditionally add stream routes (Redis 5.0+)
    if capabilities.features.streams {
        router = router.merge(stream_routes()).merge(stream_admin_routes());
    } else {
        router = router.merge(unavailable_feature_routes(
            "/api/v1/streams",
            "/api/v1/streams/{*path}",
            "Redis Streams require Redis 5.0+",
        ));
    }

    // Conditionally add JSON routes (requires RedisJSON module)
    if capabilities.modules.json {
        router = router.merge(json_routes());
    } else {
        router = router.merge(unavailable_feature_routes(
            "/api/v1/json",
            "/api/v1/json/{*path}",
            "RedisJSON module is not available",
        ));
    }

    // Conditionally add Search routes (requires RediSearch module)
    if capabilities.modules.search {
        router = router.merge(search_routes());
    } else {
        router = router.merge(unavailable_feature_routes(
            "/api/v1/search",
            "/api/v1/search/{*path}",
            "RediSearch module is not available",
        ));
    }

    // Conditionally add Bloom routes (requires RedisBloom module)
    if capabilities.modules.bloom {
        router = router
            .merge(bloom_routes())
            // CMS and Top-K are part of RedisBloom module
            .merge(cms_routes())
            .merge(topk_routes());
    } else {
        router = router
            .merge(unavailable_feature_routes(
                "/api/v1/bloom",
                "/api/v1/bloom/{*path}",
                "RedisBloom module is not available",
            ))
            .merge(unavailable_feature_routes(
                "/api/v1/cms",
                "/api/v1/cms/{*path}",
                "RedisBloom Count-Min Sketch commands are not available",
            ))
            .merge(unavailable_feature_routes(
                "/api/v1/topk",
                "/api/v1/topk/{*path}",
                "RedisBloom Top-K commands are not available",
            ));
    }

    // HyperLogLog is always available (core Redis)
    router = router.merge(hyperloglog_routes());

    // Pub/Sub is always available (core Redis)
    router = router.merge(pubsub_routes());

    // Transactions are always available (core Redis)
    router = router.merge(transaction_routes());

    // Scripting is always available (core Redis)
    router = router.merge(scripting_routes());

    if capabilities.features.functions {
        router = router.merge(functions_routes());
    } else {
        router = router.merge(unavailable_feature_routes(
            "/api/v1/functions",
            "/api/v1/functions/{*path}",
            "Redis Functions require Redis 7.0+",
        ));
    }

    if capabilities.modules.timeseries {
        router = router.merge(timeseries_routes());
    } else {
        router = router.merge(unavailable_feature_routes(
            "/api/v1/timeseries",
            "/api/v1/timeseries/{*path}",
            "RedisTimeSeries module is not available",
        ));
    }

    // Cluster info endpoints (only when connected to a cluster)
    if capabilities.features.cluster {
        router = router.merge(cluster_routes());
    } else {
        router = router.merge(unavailable_feature_routes(
            "/api/v1/cluster",
            "/api/v1/cluster/{*path}",
            "Redis Cluster mode is not available",
        ));
    }

    router.with_state(state)
}

fn unavailable_feature_routes(
    base_path: &'static str,
    wildcard_path: &'static str,
    message: &'static str,
) -> Router<AppState> {
    Router::new()
        .route(
            base_path,
            any(move || async move {
                Err::<(), CacheError>(CacheError::ModuleNotAvailable(message.to_string()))
            }),
        )
        .route(
            wildcard_path,
            any(move || async move {
                Err::<(), CacheError>(CacheError::ModuleNotAvailable(message.to_string()))
            }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::redis::capabilities::RedisCapabilities;
    use crate::test_support::{
        test_state_with_bloom_repo, test_state_with_function_repo, test_state_with_json_repo,
        test_state_with_search_repo, test_state_with_timeseries_repo,
    };
    use axum::http::Request;
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_build_router_health() {
        let (state, _) = test_state_with_json_repo();
        // Health routes are in operational_routes(), not build_router()
        let app = operational_routes().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
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
    async fn test_build_router_returns_501_for_unavailable_json_module() {
        let (mut state, _) = test_state_with_json_repo();
        let mut capabilities = RedisCapabilities::default_capabilities();
        capabilities.modules.json = false;
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
        assert_eq!(response.status(), axum::http::StatusCode::NOT_IMPLEMENTED);
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

    #[tokio::test]
    async fn test_build_router_with_functions_routes() {
        let (mut state, _) = test_state_with_function_repo();
        let mut capabilities = RedisCapabilities::default_capabilities();
        capabilities.features.functions = true;
        state.capabilities = Arc::new(capabilities);

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/functions")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_ne!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_build_router_with_timeseries_routes() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut capabilities = RedisCapabilities::default_capabilities();
        capabilities.modules.timeseries = true;
        state.capabilities = Arc::new(capabilities);

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/timeseries/example")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_ne!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_build_router_public_capabilities_alias() {
        let (state, _) = test_state_with_json_repo();
        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/capabilities")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
