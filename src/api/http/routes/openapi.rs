//! OpenAPI Documentation
//!
//! Swagger UI and OpenAPI specification endpoints.

use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::http::schemas::strings::{
    AppendRequest, AppendResponse, GetDelResponse, GetExParams, GetRangeParams,
    GetRangeResponse, IncrementRequest, IncrementResponse, MGetRequest, MGetResponse,
    MSetRequest, MSetResponse, SetRangeRequest, SetRangeResponse, SetStringRequest,
    SetStringResponse, StrLenResponse,
};
use crate::api::http::schemas::common::{KeyInfo, PaginationParams, ScanParams, TtlInfo};
use crate::domain::entities::StringValue;
use crate::domain::errors::{ErrorDetail, ErrorResponse};
use crate::infrastructure::redis::capabilities::{
    FeatureCapabilities, ModuleCapabilities, RedisCapabilities,
};
use crate::infrastructure::redis::connection::PoolStats;
use crate::shared::app_state::AppState;

/// OpenAPI documentation for the Redis Caching Service
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Redis Caching Service API",
        version = "0.1.0",
        description = "A high-performance Redis caching service with comprehensive Redis operations through a clean REST API",
        license(name = "MIT"),
        contact(name = "API Support")
    ),
    servers(
        (url = "/", description = "Local server")
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Strings", description = "Redis string operations (GET, SET, MGET, MSET, INCR, etc.)"),
        (name = "Admin", description = "Administrative endpoints (pool stats, capabilities)")
    ),
    paths(
        // Health endpoints
        crate::api::http::routes::health::health,
        crate::api::http::routes::health::readiness,
        crate::api::http::routes::health::liveness,
        // String endpoints
        crate::api::http::routes::strings::get_string,
        crate::api::http::routes::strings::set_string,
        crate::api::http::routes::strings::get_del_string,
        crate::api::http::routes::strings::mget_strings,
        crate::api::http::routes::strings::mset_strings,
        crate::api::http::routes::strings::incr_string,
        crate::api::http::routes::strings::decr_string,
        crate::api::http::routes::strings::append_string,
        crate::api::http::routes::strings::strlen_string,
        crate::api::http::routes::strings::get_range,
        crate::api::http::routes::strings::set_range,
        crate::api::http::routes::strings::get_ex_string,
    ),
    components(
        schemas(
            // Common schemas
            ErrorResponse,
            ErrorDetail,
            KeyInfo,
            PaginationParams,
            ScanParams,
            TtlInfo,
            // String schemas
            StringValue,
            SetStringRequest,
            SetStringResponse,
            GetDelResponse,
            MGetRequest,
            MGetResponse,
            MSetRequest,
            MSetResponse,
            IncrementRequest,
            IncrementResponse,
            AppendRequest,
            AppendResponse,
            StrLenResponse,
            GetRangeParams,
            GetRangeResponse,
            SetRangeRequest,
            SetRangeResponse,
            GetExParams,
            // Admin schemas
            PoolStats,
            RedisCapabilities,
            ModuleCapabilities,
            FeatureCapabilities,
        )
    )
)]
pub struct ApiDoc;

/// Create OpenAPI routes with Swagger UI
pub fn openapi_routes() -> Router<AppState> {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
}
