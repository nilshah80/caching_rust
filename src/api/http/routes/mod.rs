//! HTTP Routes
//!
//! Route definitions for all API endpoints.

mod admin;
mod health;
mod openapi;
mod strings;

pub use admin::admin_routes;
pub use health::health_routes;
pub use openapi::openapi_routes;
pub use strings::string_routes;

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
    // - key_routes()
    // - stream_routes() (Redis 5.0+)
    // - json_routes() (requires RedisJSON module)
    // - search_routes() (requires RediSearch module)
    // - bloom_routes() (requires RedisBloom module)
    // - timeseries_routes() (requires RedisTimeSeries module)
    // - functions_routes() (Redis 7.0+)

    router.with_state(state)
}
