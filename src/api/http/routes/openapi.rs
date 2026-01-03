//! OpenAPI Documentation
//!
//! Swagger UI and OpenAPI specification endpoints.

use axum::Router;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::api::http::routes::admin::{
    // Database operations
    CopyKeyRequest, CopyKeyResponse, MoveKeyRequest, MoveKeyResponse,
    SwapDbRequest, SwapDbResponse, DbSizeResponse, FlushDbRequest,
    // Server info
    LastSaveResponse,
    // Memory operations
    MemoryUsageRequest, MemoryDoctorResponse, MemoryPurgeResponse,
    // Config operations
    ConfigGetRequest, ConfigGetResponse, ConfigSetRequest, ConfigSetResponse,
    ConfigRewriteResponse, ConfigResetStatResponse,
    // Persistence operations
    SaveResponse,
    // Client operations
    ClientListResponse, ClientKillRequest, ClientKillResponse,
    ClientPauseRequest, ClientPauseResponse, ClientUnpauseResponse,
    ClientSetNameRequest, ClientSetNameResponse, ClientGetNameResponse, ClientIdResponse,
    // Monitoring operations
    SlowlogGetRequest, SlowlogGetResponse, SlowlogLenResponse, SlowlogResetResponse,
    LatencyLatestResponse, LatencyHistoryRequest, LatencyHistoryResponse,
    LatencyDoctorResponse, LatencyResetRequest, LatencyResetResponse,
    // ACL operations
    AclListResponse, AclUsersResponse, AclWhoamiResponse,
    AclCatRequest, AclCatResponse, AclGenPassRequest, AclGenPassResponse,
    AclLogRequest, AclLogResponse,
};
// Domain entities used directly in API responses
use crate::domain::entities::{
    ServerInfo, ServerTime, MemoryStats, MemoryUsage,
    ClientInfo, SlowlogEntry, LatencyEvent, AclLogEntry,
    FlushResult, BgSaveResult, BgRewriteAofResult,
};
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
        (name = "Admin", description = "Administrative endpoints (pool stats, capabilities, server info, database ops, config, persistence, client management, monitoring, ACL)")
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
        // Admin - Public endpoints
        crate::api::http::routes::admin::get_pool_stats,
        crate::api::http::routes::admin::get_capabilities,
        // Admin - Server info
        crate::api::http::routes::admin::get_server_info,
        crate::api::http::routes::admin::get_server_time,
        crate::api::http::routes::admin::get_db_size,
        crate::api::http::routes::admin::get_lastsave,
        // Admin - Memory
        crate::api::http::routes::admin::get_memory_stats,
        crate::api::http::routes::admin::get_memory_usage,
        crate::api::http::routes::admin::memory_doctor,
        crate::api::http::routes::admin::memory_purge,
        // Admin - Database operations
        crate::api::http::routes::admin::flush_db,
        crate::api::http::routes::admin::flush_all,
        crate::api::http::routes::admin::copy_key,
        crate::api::http::routes::admin::move_key,
        crate::api::http::routes::admin::swap_db,
        // Admin - Config
        crate::api::http::routes::admin::config_get,
        crate::api::http::routes::admin::config_set,
        crate::api::http::routes::admin::config_rewrite,
        crate::api::http::routes::admin::config_resetstat,
        // Admin - Persistence
        crate::api::http::routes::admin::save,
        crate::api::http::routes::admin::bgsave,
        crate::api::http::routes::admin::bgrewriteaof,
        // Admin - Client operations
        crate::api::http::routes::admin::client_list,
        crate::api::http::routes::admin::client_kill,
        crate::api::http::routes::admin::client_pause,
        crate::api::http::routes::admin::client_unpause,
        crate::api::http::routes::admin::client_setname,
        crate::api::http::routes::admin::client_getname,
        crate::api::http::routes::admin::client_id,
        // Admin - Slowlog
        crate::api::http::routes::admin::slowlog_get,
        crate::api::http::routes::admin::slowlog_len,
        crate::api::http::routes::admin::slowlog_reset,
        // Admin - Latency
        crate::api::http::routes::admin::latency_latest,
        crate::api::http::routes::admin::latency_history,
        crate::api::http::routes::admin::latency_doctor,
        crate::api::http::routes::admin::latency_reset,
        // Admin - ACL
        crate::api::http::routes::admin::acl_list,
        crate::api::http::routes::admin::acl_users,
        crate::api::http::routes::admin::acl_whoami,
        crate::api::http::routes::admin::acl_cat,
        crate::api::http::routes::admin::acl_genpass,
        crate::api::http::routes::admin::acl_log,
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
            // Admin - Pool & Capabilities
            PoolStats,
            RedisCapabilities,
            ModuleCapabilities,
            FeatureCapabilities,
            // Admin - Server info (domain entities)
            ServerInfo,
            ServerTime,
            DbSizeResponse,
            LastSaveResponse,
            // Admin - Memory (domain entities + request schemas)
            MemoryStats,
            MemoryUsageRequest,
            MemoryUsage,
            MemoryDoctorResponse,
            MemoryPurgeResponse,
            // Admin - Database operations (domain entity + request/response schemas)
            FlushDbRequest,
            FlushResult,
            CopyKeyRequest,
            CopyKeyResponse,
            MoveKeyRequest,
            MoveKeyResponse,
            SwapDbRequest,
            SwapDbResponse,
            // Admin - Config
            ConfigGetRequest,
            ConfigGetResponse,
            ConfigSetRequest,
            ConfigSetResponse,
            ConfigRewriteResponse,
            ConfigResetStatResponse,
            // Admin - Persistence (domain entities + response schema)
            SaveResponse,
            BgSaveResult,
            BgRewriteAofResult,
            // Admin - Client (domain entity for ClientInfo)
            ClientInfo,
            ClientListResponse,
            ClientKillRequest,
            ClientKillResponse,
            ClientPauseRequest,
            ClientPauseResponse,
            ClientUnpauseResponse,
            ClientSetNameRequest,
            ClientSetNameResponse,
            ClientGetNameResponse,
            ClientIdResponse,
            // Admin - Slowlog
            SlowlogEntry,
            SlowlogGetRequest,
            SlowlogGetResponse,
            SlowlogLenResponse,
            SlowlogResetResponse,
            // Admin - Latency
            LatencyEvent,
            LatencyLatestResponse,
            LatencyHistoryRequest,
            LatencyHistoryResponse,
            LatencyDoctorResponse,
            LatencyResetRequest,
            LatencyResetResponse,
            // Admin - ACL
            AclListResponse,
            AclUsersResponse,
            AclWhoamiResponse,
            AclCatRequest,
            AclCatResponse,
            AclGenPassRequest,
            AclGenPassResponse,
            AclLogRequest,
            AclLogEntry,
            AclLogResponse,
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Security scheme modifier for OpenAPI
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Admin-Api-Key"))),
            );
        }
    }
}

/// Create OpenAPI routes with Swagger UI
pub fn openapi_routes() -> Router<AppState> {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_security_scheme() {
        let spec = ApiDoc::openapi();
        let components = spec.components.expect("components");
        assert!(components.security_schemes.contains_key("api_key"));
    }
}
