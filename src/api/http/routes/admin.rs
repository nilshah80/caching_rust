//! Admin Routes
//!
//! Administrative endpoints for server operations, pool stats, and capabilities.
//! These endpoints require API key authentication and use the AdminService.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::http::middleware::admin_auth::ADMIN_API_KEY_HEADER;
use crate::domain::entities::{
    AclLogEntry, BgRewriteAofResult, BgSaveResult, ClientInfo as DomainClientInfo,
    FlushResult, LatencyEvent, MemoryStats, MemoryUsage, ServerInfo, ServerTime,
    SlowlogEntry,
};
use crate::domain::errors::CacheError;
use crate::infrastructure::redis::capabilities::RedisCapabilities;
use crate::infrastructure::redis::connection::PoolStats;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

// ============================================================================
// Request/Response Schemas
// ============================================================================

/// Database size response
#[derive(Debug, Serialize, ToSchema)]
pub struct DbSizeResponse {
    pub keys: i64,
}

/// Last save response
#[derive(Debug, Serialize, ToSchema)]
pub struct LastSaveResponse {
    pub timestamp: i64,
}

/// Memory usage request
#[derive(Debug, Deserialize, ToSchema)]
pub struct MemoryUsageRequest {
    pub key: String,
    #[serde(default = "default_samples")]
    pub samples: u32,
}

fn default_samples() -> u32 {
    5
}

/// Memory doctor response
#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryDoctorResponse {
    pub report: String,
}

/// Memory purge response
#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryPurgeResponse {
    pub success: bool,
}

/// Flush database request
#[derive(Debug, Deserialize, ToSchema)]
pub struct FlushDbRequest {
    #[serde(default)]
    pub async_mode: bool,
}

/// Copy key request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CopyKeyRequest {
    pub source: String,
    pub destination: String,
    pub db: Option<u8>,
    #[serde(default)]
    pub replace: bool,
}

/// Copy key response
#[derive(Debug, Serialize, ToSchema)]
pub struct CopyKeyResponse {
    pub copied: bool,
}

/// Move key request
#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveKeyRequest {
    pub key: String,
    pub db: u8,
}

/// Move key response
#[derive(Debug, Serialize, ToSchema)]
pub struct MoveKeyResponse {
    pub moved: bool,
}

/// Swap databases request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SwapDbRequest {
    pub db1: u8,
    pub db2: u8,
}

/// Swap databases response
#[derive(Debug, Serialize, ToSchema)]
pub struct SwapDbResponse {
    pub swapped: bool,
}

/// Config get request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfigGetRequest {
    pub pattern: String,
}

/// Config get response
#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigGetResponse {
    pub config: std::collections::HashMap<String, String>,
}

/// Config set request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfigSetRequest {
    pub parameter: String,
    pub value: String,
}

/// Config set response
#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigSetResponse {
    pub success: bool,
}

/// Config rewrite response
#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigRewriteResponse {
    pub success: bool,
}

/// Config resetstat response
#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigResetStatResponse {
    pub success: bool,
}

/// Save response
#[derive(Debug, Serialize, ToSchema)]
pub struct SaveResponse {
    pub success: bool,
    pub mode: String,
}

/// Client list response (uses domain entity)
#[derive(Debug, Serialize, ToSchema)]
pub struct ClientListResponse {
    pub clients: Vec<DomainClientInfo>,
}

/// Client kill request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ClientKillRequest {
    pub id: Option<i64>,
    pub addr: Option<String>,
    pub client_type: Option<String>,
}

/// Client kill response
#[derive(Debug, Serialize, ToSchema)]
pub struct ClientKillResponse {
    pub killed: i64,
}

/// Client pause request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ClientPauseRequest {
    pub timeout_ms: u64,
    #[serde(default = "default_pause_mode")]
    pub mode: String,
}

fn default_pause_mode() -> String {
    "write".to_string()
}

/// Client pause response
#[derive(Debug, Serialize, ToSchema)]
pub struct ClientPauseResponse {
    pub success: bool,
}

/// Client unpause response
#[derive(Debug, Serialize, ToSchema)]
pub struct ClientUnpauseResponse {
    pub success: bool,
}

/// Client setname request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ClientSetNameRequest {
    pub name: String,
}

/// Client setname response
#[derive(Debug, Serialize, ToSchema)]
pub struct ClientSetNameResponse {
    pub success: bool,
}

/// Client getname response
#[derive(Debug, Serialize, ToSchema)]
pub struct ClientGetNameResponse {
    pub name: Option<String>,
}

/// Client ID response
#[derive(Debug, Serialize, ToSchema)]
pub struct ClientIdResponse {
    pub id: i64,
}

/// Slowlog get request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SlowlogGetRequest {
    #[serde(default = "default_slowlog_count")]
    pub count: i64,
}

fn default_slowlog_count() -> i64 {
    10
}

/// Slowlog get response
#[derive(Debug, Serialize, ToSchema)]
pub struct SlowlogGetResponse {
    pub entries: Vec<SlowlogEntry>,
}

/// Slowlog len response
#[derive(Debug, Serialize, ToSchema)]
pub struct SlowlogLenResponse {
    pub length: i64,
}

/// Slowlog reset response
#[derive(Debug, Serialize, ToSchema)]
pub struct SlowlogResetResponse {
    pub success: bool,
}

/// Latency latest response
#[derive(Debug, Serialize, ToSchema)]
pub struct LatencyLatestResponse {
    pub events: Vec<LatencyEvent>,
}

/// Latency history request
#[derive(Debug, Deserialize, ToSchema)]
pub struct LatencyHistoryRequest {
    pub event: String,
}

/// Latency history response
#[derive(Debug, Serialize, ToSchema)]
pub struct LatencyHistoryResponse {
    pub samples: Vec<LatencyEvent>,
}

/// Latency doctor response
#[derive(Debug, Serialize, ToSchema)]
pub struct LatencyDoctorResponse {
    pub report: String,
}

/// Latency reset request
#[derive(Debug, Deserialize, ToSchema)]
pub struct LatencyResetRequest {
    #[serde(default)]
    pub events: Vec<String>,
}

/// Latency reset response
#[derive(Debug, Serialize, ToSchema)]
pub struct LatencyResetResponse {
    pub success: bool,
}

/// ACL list response
#[derive(Debug, Serialize, ToSchema)]
pub struct AclListResponse {
    pub rules: Vec<String>,
}

/// ACL users response
#[derive(Debug, Serialize, ToSchema)]
pub struct AclUsersResponse {
    pub users: Vec<String>,
}

/// ACL whoami response
#[derive(Debug, Serialize, ToSchema)]
pub struct AclWhoamiResponse {
    pub username: String,
}

/// ACL cat request
#[derive(Debug, Deserialize, ToSchema)]
pub struct AclCatRequest {
    pub category: Option<String>,
}

/// ACL cat response
#[derive(Debug, Serialize, ToSchema)]
pub struct AclCatResponse {
    pub items: Vec<String>,
}

/// ACL genpass request
#[derive(Debug, Deserialize, ToSchema)]
pub struct AclGenPassRequest {
    #[serde(default = "default_genpass_bits")]
    pub bits: u32,
}

fn default_genpass_bits() -> u32 {
    256
}

/// ACL genpass response
#[derive(Debug, Serialize, ToSchema)]
pub struct AclGenPassResponse {
    pub password: String,
}

/// ACL log request
#[derive(Debug, Deserialize, ToSchema)]
pub struct AclLogRequest {
    pub count: Option<i64>,
    #[serde(default)]
    pub reset: bool,
}

/// ACL log response
#[derive(Debug, Serialize, ToSchema)]
pub struct AclLogResponse {
    pub entries: Vec<AclLogEntry>,
}

// ============================================================================
// Router
// ============================================================================

/// Create admin routes (protected by API key)
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        // Public admin routes (no auth required)
        .route("/api/v1/admin/pool/stats", get(get_pool_stats))
        .route("/api/v1/admin/capabilities", get(get_capabilities))
        // Server info (protected)
        .route("/api/v1/admin/server/info", get(get_server_info))
        .route("/api/v1/admin/server/time", get(get_server_time))
        .route("/api/v1/admin/server/dbsize", get(get_db_size))
        .route("/api/v1/admin/server/lastsave", get(get_lastsave))
        // Memory operations (protected)
        .route("/api/v1/admin/server/memory/stats", get(get_memory_stats))
        .route("/api/v1/admin/server/memory/usage", post(get_memory_usage))
        .route("/api/v1/admin/server/memory/doctor", get(memory_doctor))
        .route("/api/v1/admin/server/memory/purge", post(memory_purge))
        // Database operations (protected)
        .route("/api/v1/admin/db/flush", delete(flush_db))
        .route("/api/v1/admin/db/flushall", delete(flush_all))
        .route("/api/v1/admin/db/copy", post(copy_key))
        .route("/api/v1/admin/db/move", post(move_key))
        .route("/api/v1/admin/db/swapdb", post(swap_db))
        // Configuration operations (protected)
        .route("/api/v1/admin/config/get", post(config_get))
        .route("/api/v1/admin/config/set", post(config_set))
        .route("/api/v1/admin/config/rewrite", post(config_rewrite))
        .route("/api/v1/admin/config/resetstat", post(config_resetstat))
        // Persistence operations (protected)
        .route("/api/v1/admin/persistence/save", post(save))
        .route("/api/v1/admin/persistence/bgsave", post(bgsave))
        .route("/api/v1/admin/persistence/bgrewriteaof", post(bgrewriteaof))
        // Client operations (protected)
        .route("/api/v1/admin/client/list", get(client_list))
        .route("/api/v1/admin/client/kill", post(client_kill))
        .route("/api/v1/admin/client/pause", post(client_pause))
        .route("/api/v1/admin/client/unpause", post(client_unpause))
        .route("/api/v1/admin/client/setname", post(client_setname))
        .route("/api/v1/admin/client/getname", get(client_getname))
        .route("/api/v1/admin/client/id", get(client_id))
        // Slowlog operations (protected)
        .route("/api/v1/admin/slowlog/get", post(slowlog_get))
        .route("/api/v1/admin/slowlog/len", get(slowlog_len))
        .route("/api/v1/admin/slowlog/reset", post(slowlog_reset))
        // Latency operations (protected)
        .route("/api/v1/admin/latency/latest", get(latency_latest))
        .route("/api/v1/admin/latency/history", post(latency_history))
        .route("/api/v1/admin/latency/doctor", get(latency_doctor))
        .route("/api/v1/admin/latency/reset", post(latency_reset))
        // ACL operations (protected)
        .route("/api/v1/admin/acl/list", get(acl_list))
        .route("/api/v1/admin/acl/users", get(acl_users))
        .route("/api/v1/admin/acl/whoami", get(acl_whoami))
        .route("/api/v1/admin/acl/cat", post(acl_cat))
        .route("/api/v1/admin/acl/genpass", post(acl_genpass))
        .route("/api/v1/admin/acl/log", post(acl_log))
}

// ============================================================================
// Auth Helper
// ============================================================================

/// Verify admin API key from headers
fn verify_admin_key(headers: &HeaderMap, state: &AppState) -> Result<(), StatusCode> {
    let api_key = headers
        .get(ADMIN_API_KEY_HEADER)
        .and_then(|v| v.to_str().ok());

    match api_key {
        Some(key) if key == state.config.admin.api_key => Ok(()),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Convert CacheError to StatusCode
fn to_status_code(err: CacheError) -> StatusCode {
    match err {
        CacheError::KeyNotFound(_) => StatusCode::NOT_FOUND,
        CacheError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        CacheError::Unauthorized => StatusCode::UNAUTHORIZED,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// ============================================================================
// Public Endpoints (No Auth)
// ============================================================================

/// GET /api/v1/admin/pool/stats
#[utoipa::path(
    get,
    path = "/api/v1/admin/pool/stats",
    responses(
        (status = 200, description = "Pool statistics", body = PoolStats)
    ),
    tag = "Admin"
)]
pub async fn get_pool_stats(
    State(state): State<AppState>,
) -> ApiResponse<PoolStats> {
    ApiResponse::success(state.pool.get_stats())
}

/// GET /api/v1/admin/capabilities
#[utoipa::path(
    get,
    path = "/api/v1/admin/capabilities",
    responses(
        (status = 200, description = "Redis capabilities", body = RedisCapabilities)
    ),
    tag = "Admin"
)]
pub async fn get_capabilities(
    State(state): State<AppState>,
) -> ApiResponse<RedisCapabilities> {
    ApiResponse::success((*state.capabilities).clone())
}

// ============================================================================
// Server Operations
// ============================================================================

/// GET /api/v1/admin/server/info
#[utoipa::path(
    get,
    path = "/api/v1/admin/server/info",
    responses(
        (status = 200, description = "Server information", body = ServerInfo),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn get_server_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<ServerInfo>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.get_server_info().await
        .map(ApiResponse::success)
        .map_err(to_status_code)
}

/// GET /api/v1/admin/server/time
#[utoipa::path(
    get,
    path = "/api/v1/admin/server/time",
    responses(
        (status = 200, description = "Server time", body = ServerTime),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn get_server_time(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<ServerTime>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.get_server_time().await
        .map(ApiResponse::success)
        .map_err(to_status_code)
}

/// GET /api/v1/admin/server/dbsize
#[utoipa::path(
    get,
    path = "/api/v1/admin/server/dbsize",
    responses(
        (status = 200, description = "Database size", body = DbSizeResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn get_db_size(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<DbSizeResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.get_db_size().await
        .map(|keys| ApiResponse::success(DbSizeResponse { keys }))
        .map_err(to_status_code)
}

/// GET /api/v1/admin/server/lastsave
#[utoipa::path(
    get,
    path = "/api/v1/admin/server/lastsave",
    responses(
        (status = 200, description = "Last save timestamp", body = LastSaveResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn get_lastsave(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<LastSaveResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.get_last_save().await
        .map(|timestamp| ApiResponse::success(LastSaveResponse { timestamp }))
        .map_err(to_status_code)
}

// ============================================================================
// Memory Operations
// ============================================================================

/// GET /api/v1/admin/server/memory/stats
#[utoipa::path(
    get,
    path = "/api/v1/admin/server/memory/stats",
    responses(
        (status = 200, description = "Memory statistics", body = MemoryStats),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn get_memory_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<MemoryStats>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.get_memory_stats().await
        .map(ApiResponse::success)
        .map_err(to_status_code)
}

/// POST /api/v1/admin/server/memory/usage
#[utoipa::path(
    post,
    path = "/api/v1/admin/server/memory/usage",
    request_body = MemoryUsageRequest,
    responses(
        (status = 200, description = "Memory usage", body = MemoryUsage),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn get_memory_usage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<MemoryUsageRequest>,
) -> Result<ApiResponse<MemoryUsage>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.get_memory_usage(&request.key, Some(request.samples)).await
        .map(ApiResponse::success)
        .map_err(to_status_code)
}

/// GET /api/v1/admin/server/memory/doctor
#[utoipa::path(
    get,
    path = "/api/v1/admin/server/memory/doctor",
    responses(
        (status = 200, description = "Memory doctor report", body = MemoryDoctorResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn memory_doctor(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<MemoryDoctorResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.memory_doctor().await
        .map(|report| ApiResponse::success(MemoryDoctorResponse { report }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/server/memory/purge
#[utoipa::path(
    post,
    path = "/api/v1/admin/server/memory/purge",
    responses(
        (status = 200, description = "Memory purged", body = MemoryPurgeResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn memory_purge(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<MemoryPurgeResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.memory_purge().await
        .map(|_| ApiResponse::success(MemoryPurgeResponse { success: true }))
        .map_err(to_status_code)
}

// ============================================================================
// Database Operations
// ============================================================================

/// DELETE /api/v1/admin/db/flush
#[utoipa::path(
    delete,
    path = "/api/v1/admin/db/flush",
    request_body = FlushDbRequest,
    responses(
        (status = 200, description = "Database flushed", body = FlushResult),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn flush_db(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<FlushDbRequest>,
) -> Result<ApiResponse<FlushResult>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.flush_db(request.async_mode).await
        .map(ApiResponse::success)
        .map_err(to_status_code)
}

/// DELETE /api/v1/admin/db/flushall
#[utoipa::path(
    delete,
    path = "/api/v1/admin/db/flushall",
    request_body = FlushDbRequest,
    responses(
        (status = 200, description = "All databases flushed", body = FlushResult),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn flush_all(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<FlushDbRequest>,
) -> Result<ApiResponse<FlushResult>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.flush_all(request.async_mode).await
        .map(ApiResponse::success)
        .map_err(to_status_code)
}

/// POST /api/v1/admin/db/copy
#[utoipa::path(
    post,
    path = "/api/v1/admin/db/copy",
    request_body = CopyKeyRequest,
    responses(
        (status = 200, description = "Key copied", body = CopyKeyResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn copy_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CopyKeyRequest>,
) -> Result<ApiResponse<CopyKeyResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.copy_key(request.source, request.destination, request.db, request.replace).await
        .map(|copied| ApiResponse::success(CopyKeyResponse { copied }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/db/move
#[utoipa::path(
    post,
    path = "/api/v1/admin/db/move",
    request_body = MoveKeyRequest,
    responses(
        (status = 200, description = "Key moved", body = MoveKeyResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn move_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<MoveKeyRequest>,
) -> Result<ApiResponse<MoveKeyResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.move_key(request.key, request.db).await
        .map(|moved| ApiResponse::success(MoveKeyResponse { moved }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/db/swapdb
#[utoipa::path(
    post,
    path = "/api/v1/admin/db/swapdb",
    request_body = SwapDbRequest,
    responses(
        (status = 200, description = "Databases swapped", body = SwapDbResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn swap_db(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SwapDbRequest>,
) -> Result<ApiResponse<SwapDbResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.swap_db(request.db1, request.db2).await
        .map(|_| ApiResponse::success(SwapDbResponse { swapped: true }))
        .map_err(to_status_code)
}

// ============================================================================
// Configuration Operations
// ============================================================================

/// POST /api/v1/admin/config/get
#[utoipa::path(
    post,
    path = "/api/v1/admin/config/get",
    request_body = ConfigGetRequest,
    responses(
        (status = 200, description = "Configuration", body = ConfigGetResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn config_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ConfigGetRequest>,
) -> Result<ApiResponse<ConfigGetResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.config_get(&request.pattern).await
        .map(|config| ApiResponse::success(ConfigGetResponse { config }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/config/set
#[utoipa::path(
    post,
    path = "/api/v1/admin/config/set",
    request_body = ConfigSetRequest,
    responses(
        (status = 200, description = "Configuration set", body = ConfigSetResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn config_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ConfigSetRequest>,
) -> Result<ApiResponse<ConfigSetResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.config_set(&request.parameter, &request.value).await
        .map(|_| ApiResponse::success(ConfigSetResponse { success: true }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/config/rewrite
#[utoipa::path(
    post,
    path = "/api/v1/admin/config/rewrite",
    responses(
        (status = 200, description = "Configuration rewritten", body = ConfigRewriteResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn config_rewrite(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<ConfigRewriteResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.config_rewrite().await
        .map(|_| ApiResponse::success(ConfigRewriteResponse { success: true }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/config/resetstat
#[utoipa::path(
    post,
    path = "/api/v1/admin/config/resetstat",
    responses(
        (status = 200, description = "Statistics reset", body = ConfigResetStatResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn config_resetstat(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<ConfigResetStatResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.config_resetstat().await
        .map(|_| ApiResponse::success(ConfigResetStatResponse { success: true }))
        .map_err(to_status_code)
}

// ============================================================================
// Persistence Operations
// ============================================================================

/// POST /api/v1/admin/persistence/save
#[utoipa::path(
    post,
    path = "/api/v1/admin/persistence/save",
    responses(
        (status = 200, description = "Save completed", body = SaveResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn save(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<SaveResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.save().await
        .map(|_| ApiResponse::success(SaveResponse { success: true, mode: "sync".to_string() }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/persistence/bgsave
#[utoipa::path(
    post,
    path = "/api/v1/admin/persistence/bgsave",
    responses(
        (status = 200, description = "Background save started", body = BgSaveResult),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn bgsave(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<BgSaveResult>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.bgsave().await
        .map(ApiResponse::success)
        .map_err(to_status_code)
}

/// POST /api/v1/admin/persistence/bgrewriteaof
#[utoipa::path(
    post,
    path = "/api/v1/admin/persistence/bgrewriteaof",
    responses(
        (status = 200, description = "AOF rewrite started", body = BgRewriteAofResult),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn bgrewriteaof(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<BgRewriteAofResult>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.bgrewriteaof().await
        .map(ApiResponse::success)
        .map_err(to_status_code)
}

// ============================================================================
// Client Operations
// ============================================================================

/// GET /api/v1/admin/client/list
#[utoipa::path(
    get,
    path = "/api/v1/admin/client/list",
    responses(
        (status = 200, description = "Client list", body = ClientListResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn client_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<ClientListResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.client_list().await
        .map(|clients| ApiResponse::success(ClientListResponse { clients }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/client/kill
#[utoipa::path(
    post,
    path = "/api/v1/admin/client/kill",
    request_body = ClientKillRequest,
    responses(
        (status = 200, description = "Clients killed", body = ClientKillResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn client_kill(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ClientKillRequest>,
) -> Result<ApiResponse<ClientKillResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.client_kill(request.id, request.addr, request.client_type).await
        .map(|killed| ApiResponse::success(ClientKillResponse { killed }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/client/pause
#[utoipa::path(
    post,
    path = "/api/v1/admin/client/pause",
    request_body = ClientPauseRequest,
    responses(
        (status = 200, description = "Clients paused", body = ClientPauseResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn client_pause(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ClientPauseRequest>,
) -> Result<ApiResponse<ClientPauseResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.client_pause(request.timeout_ms, Some(request.mode)).await
        .map(|_| ApiResponse::success(ClientPauseResponse { success: true }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/client/unpause
#[utoipa::path(
    post,
    path = "/api/v1/admin/client/unpause",
    responses(
        (status = 200, description = "Clients unpaused", body = ClientUnpauseResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn client_unpause(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<ClientUnpauseResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.client_unpause().await
        .map(|_| ApiResponse::success(ClientUnpauseResponse { success: true }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/client/setname
#[utoipa::path(
    post,
    path = "/api/v1/admin/client/setname",
    request_body = ClientSetNameRequest,
    responses(
        (status = 200, description = "Client name set", body = ClientSetNameResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn client_setname(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ClientSetNameRequest>,
) -> Result<ApiResponse<ClientSetNameResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.client_setname(&request.name).await
        .map(|_| ApiResponse::success(ClientSetNameResponse { success: true }))
        .map_err(to_status_code)
}

/// GET /api/v1/admin/client/getname
#[utoipa::path(
    get,
    path = "/api/v1/admin/client/getname",
    responses(
        (status = 200, description = "Client name", body = ClientGetNameResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn client_getname(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<ClientGetNameResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.client_getname().await
        .map(|name| ApiResponse::success(ClientGetNameResponse { name }))
        .map_err(to_status_code)
}

/// GET /api/v1/admin/client/id
#[utoipa::path(
    get,
    path = "/api/v1/admin/client/id",
    responses(
        (status = 200, description = "Client ID", body = ClientIdResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn client_id(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<ClientIdResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.client_id().await
        .map(|id| ApiResponse::success(ClientIdResponse { id }))
        .map_err(to_status_code)
}

// ============================================================================
// Slowlog Operations
// ============================================================================

/// POST /api/v1/admin/slowlog/get
#[utoipa::path(
    post,
    path = "/api/v1/admin/slowlog/get",
    request_body = SlowlogGetRequest,
    responses(
        (status = 200, description = "Slowlog entries", body = SlowlogGetResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn slowlog_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SlowlogGetRequest>,
) -> Result<ApiResponse<SlowlogGetResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.slowlog_get(Some(request.count)).await
        .map(|entries| ApiResponse::success(SlowlogGetResponse { entries }))
        .map_err(to_status_code)
}

/// GET /api/v1/admin/slowlog/len
#[utoipa::path(
    get,
    path = "/api/v1/admin/slowlog/len",
    responses(
        (status = 200, description = "Slowlog length", body = SlowlogLenResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn slowlog_len(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<SlowlogLenResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.slowlog_len().await
        .map(|length| ApiResponse::success(SlowlogLenResponse { length }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/slowlog/reset
#[utoipa::path(
    post,
    path = "/api/v1/admin/slowlog/reset",
    responses(
        (status = 200, description = "Slowlog reset", body = SlowlogResetResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn slowlog_reset(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<SlowlogResetResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.slowlog_reset().await
        .map(|_| ApiResponse::success(SlowlogResetResponse { success: true }))
        .map_err(to_status_code)
}

// ============================================================================
// Latency Operations
// ============================================================================

/// GET /api/v1/admin/latency/latest
#[utoipa::path(
    get,
    path = "/api/v1/admin/latency/latest",
    responses(
        (status = 200, description = "Latest latency events", body = LatencyLatestResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn latency_latest(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<LatencyLatestResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.latency_latest().await
        .map(|events| ApiResponse::success(LatencyLatestResponse { events }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/latency/history
#[utoipa::path(
    post,
    path = "/api/v1/admin/latency/history",
    request_body = LatencyHistoryRequest,
    responses(
        (status = 200, description = "Latency history", body = LatencyHistoryResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn latency_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<LatencyHistoryRequest>,
) -> Result<ApiResponse<LatencyHistoryResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.latency_history(&request.event).await
        .map(|samples| ApiResponse::success(LatencyHistoryResponse { samples }))
        .map_err(to_status_code)
}

/// GET /api/v1/admin/latency/doctor
#[utoipa::path(
    get,
    path = "/api/v1/admin/latency/doctor",
    responses(
        (status = 200, description = "Latency doctor report", body = LatencyDoctorResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn latency_doctor(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<LatencyDoctorResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.latency_doctor().await
        .map(|report| ApiResponse::success(LatencyDoctorResponse { report }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/latency/reset
#[utoipa::path(
    post,
    path = "/api/v1/admin/latency/reset",
    request_body = LatencyResetRequest,
    responses(
        (status = 200, description = "Latency reset", body = LatencyResetResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn latency_reset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<LatencyResetRequest>,
) -> Result<ApiResponse<LatencyResetResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.latency_reset(request.events).await
        .map(|_| ApiResponse::success(LatencyResetResponse { success: true }))
        .map_err(to_status_code)
}

// ============================================================================
// ACL Operations
// ============================================================================

/// GET /api/v1/admin/acl/list
#[utoipa::path(
    get,
    path = "/api/v1/admin/acl/list",
    responses(
        (status = 200, description = "ACL rules", body = AclListResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn acl_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<AclListResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.acl_list().await
        .map(|rules| ApiResponse::success(AclListResponse { rules }))
        .map_err(to_status_code)
}

/// GET /api/v1/admin/acl/users
#[utoipa::path(
    get,
    path = "/api/v1/admin/acl/users",
    responses(
        (status = 200, description = "ACL users", body = AclUsersResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn acl_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<AclUsersResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.acl_users().await
        .map(|users| ApiResponse::success(AclUsersResponse { users }))
        .map_err(to_status_code)
}

/// GET /api/v1/admin/acl/whoami
#[utoipa::path(
    get,
    path = "/api/v1/admin/acl/whoami",
    responses(
        (status = 200, description = "Current user", body = AclWhoamiResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn acl_whoami(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResponse<AclWhoamiResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.acl_whoami().await
        .map(|username| ApiResponse::success(AclWhoamiResponse { username }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/acl/cat
#[utoipa::path(
    post,
    path = "/api/v1/admin/acl/cat",
    request_body = AclCatRequest,
    responses(
        (status = 200, description = "ACL categories or commands", body = AclCatResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn acl_cat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AclCatRequest>,
) -> Result<ApiResponse<AclCatResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.acl_cat(request.category.as_deref()).await
        .map(|items| ApiResponse::success(AclCatResponse { items }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/acl/genpass
#[utoipa::path(
    post,
    path = "/api/v1/admin/acl/genpass",
    request_body = AclGenPassRequest,
    responses(
        (status = 200, description = "Generated password", body = AclGenPassResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn acl_genpass(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AclGenPassRequest>,
) -> Result<ApiResponse<AclGenPassResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.acl_genpass(Some(request.bits)).await
        .map(|password| ApiResponse::success(AclGenPassResponse { password }))
        .map_err(to_status_code)
}

/// POST /api/v1/admin/acl/log
#[utoipa::path(
    post,
    path = "/api/v1/admin/acl/log",
    request_body = AclLogRequest,
    responses(
        (status = 200, description = "ACL log entries", body = AclLogResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = [])),
    tag = "Admin"
)]
pub async fn acl_log(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AclLogRequest>,
) -> Result<ApiResponse<AclLogResponse>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    state.admin_service.acl_log(request.count, request.reset).await
        .map(|entries| ApiResponse::success(AclLogResponse { entries }))
        .map_err(to_status_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state;
    use axum::extract::State;
    use axum::http::{HeaderMap, HeaderValue};
    use axum::Json;

    fn auth_headers(key: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ADMIN_API_KEY_HEADER, HeaderValue::from_str(key).expect("header"));
        headers
    }

    #[tokio::test]
    async fn test_admin_helpers() {
        assert_eq!(default_samples(), 5);
        assert_eq!(default_pause_mode(), "write");
        assert_eq!(default_slowlog_count(), 10);
        assert_eq!(default_genpass_bits(), 256);

        let (state, _, _) = test_state();
        let ok = verify_admin_key(&auth_headers(&state.config.admin.api_key), &state);
        assert!(ok.is_ok());
        let bad = verify_admin_key(&HeaderMap::new(), &state);
        assert!(bad.is_err());

        assert_eq!(to_status_code(CacheError::InvalidInput("bad".to_string())), StatusCode::BAD_REQUEST);
        assert_eq!(to_status_code(CacheError::KeyNotFound("k".to_string())), StatusCode::NOT_FOUND);
        assert_eq!(to_status_code(CacheError::Unauthorized), StatusCode::UNAUTHORIZED);
        assert_eq!(to_status_code(CacheError::Timeout), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_admin_handlers() {
        let (state, _, _) = test_state();
        let auth = auth_headers(&state.config.admin.api_key);
        let state = State(state);

        let stats = get_pool_stats(state.clone()).await;
        assert!(stats.success);

        let caps = get_capabilities(state.clone()).await;
        assert!(caps.success);

        let _ = get_server_info(state.clone(), auth.clone()).await.unwrap();
        let _ = get_server_time(state.clone(), auth.clone()).await.unwrap();
        let _ = get_db_size(state.clone(), auth.clone()).await.unwrap();
        let _ = get_lastsave(state.clone(), auth.clone()).await.unwrap();

        let _ = get_memory_stats(state.clone(), auth.clone()).await.unwrap();
        let _ = get_memory_usage(
            state.clone(),
            auth.clone(),
            Json(MemoryUsageRequest { key: "k".to_string(), samples: 1 }),
        )
        .await
        .unwrap();
        let _ = memory_doctor(state.clone(), auth.clone()).await.unwrap();
        let _ = memory_purge(state.clone(), auth.clone()).await.unwrap();

        let _ = flush_db(state.clone(), auth.clone(), Json(FlushDbRequest { async_mode: false }))
            .await
            .unwrap();
        let _ = flush_all(state.clone(), auth.clone(), Json(FlushDbRequest { async_mode: true }))
            .await
            .unwrap();
        let _ = copy_key(
            state.clone(),
            auth.clone(),
            Json(CopyKeyRequest {
                source: "a".to_string(),
                destination: "b".to_string(),
                db: None,
                replace: false,
            }),
        )
        .await
        .unwrap();
        let _ = move_key(
            state.clone(),
            auth.clone(),
            Json(MoveKeyRequest { key: "a".to_string(), db: 1 }),
        )
        .await
        .unwrap();
        let _ = swap_db(
            state.clone(),
            auth.clone(),
            Json(SwapDbRequest { db1: 0, db2: 1 }),
        )
        .await
        .unwrap();

        let _ = config_get(
            state.clone(),
            auth.clone(),
            Json(ConfigGetRequest { pattern: "*".to_string() }),
        )
        .await
        .unwrap();
        let _ = config_set(
            state.clone(),
            auth.clone(),
            Json(ConfigSetRequest { parameter: "maxmemory".to_string(), value: "1".to_string() }),
        )
        .await
        .unwrap();
        let _ = config_rewrite(state.clone(), auth.clone()).await.unwrap();
        let _ = config_resetstat(state.clone(), auth.clone()).await.unwrap();

        let _ = save(state.clone(), auth.clone()).await.unwrap();
        let _ = bgsave(state.clone(), auth.clone()).await.unwrap();
        let _ = bgrewriteaof(state.clone(), auth.clone()).await.unwrap();

        let _ = client_list(state.clone(), auth.clone()).await.unwrap();
        let _ = client_kill(
            state.clone(),
            auth.clone(),
            Json(ClientKillRequest { id: None, addr: None, client_type: None }),
        )
        .await
        .unwrap();
        let _ = client_pause(
            state.clone(),
            auth.clone(),
            Json(ClientPauseRequest { timeout_ms: 1, mode: "write".to_string() }),
        )
        .await
        .unwrap();
        let _ = client_unpause(state.clone(), auth.clone()).await.unwrap();
        let _ = client_setname(
            state.clone(),
            auth.clone(),
            Json(ClientSetNameRequest { name: "client".to_string() }),
        )
        .await
        .unwrap();
        let _ = client_getname(state.clone(), auth.clone()).await.unwrap();
        let _ = client_id(state.clone(), auth.clone()).await.unwrap();

        let _ = slowlog_get(
            state.clone(),
            auth.clone(),
            Json(SlowlogGetRequest { count: 1 }),
        )
        .await
        .unwrap();
        let _ = slowlog_len(state.clone(), auth.clone()).await.unwrap();
        let _ = slowlog_reset(state.clone(), auth.clone()).await.unwrap();

        let _ = latency_latest(state.clone(), auth.clone()).await.unwrap();
        let _ = latency_history(
            state.clone(),
            auth.clone(),
            Json(LatencyHistoryRequest { event: "command".to_string() }),
        )
        .await
        .unwrap();
        let _ = latency_doctor(state.clone(), auth.clone()).await.unwrap();
        let _ = latency_reset(
            state.clone(),
            auth.clone(),
            Json(LatencyResetRequest { events: vec!["command".to_string()] }),
        )
        .await
        .unwrap();

        let _ = acl_list(state.clone(), auth.clone()).await.unwrap();
        let _ = acl_users(state.clone(), auth.clone()).await.unwrap();
        let _ = acl_whoami(state.clone(), auth.clone()).await.unwrap();
        let _ = acl_cat(
            state.clone(),
            auth.clone(),
            Json(AclCatRequest { category: Some("string".to_string()) }),
        )
        .await
        .unwrap();
        let _ = acl_genpass(
            state.clone(),
            auth.clone(),
            Json(AclGenPassRequest { bits: 64 }),
        )
        .await
        .unwrap();
        let _ = acl_log(
            state,
            auth,
            Json(AclLogRequest { count: Some(1), reset: false }),
        )
        .await
        .unwrap();
    }
}
