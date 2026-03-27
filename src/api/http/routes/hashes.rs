//! Hash Routes
//!
//! HTTP endpoints for Redis hash operations.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
};

use validator::Validate;

use crate::api::http::schemas::hashes::{
    GetMultipleFieldsRequest, HExpireAtRequest, HExpireFieldResult, HExpireRequest,
    HExpireResponse, HFieldsRequest, HGetDelRequest, HGetDelResponse, HGetExRequest,
    HGetExResponse, HPExpireAtRequest, HPExpireRequest, HSetExRequest, HSetExResponse,
    HashFieldEntry, HashIncrFloatRequest, HashIncrRequest, HashRandomFieldResponse,
    HashScanResponse, RandomFieldQuery, ScanHashQuery, SetHashNxRequest, SetHashRequest,
};
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create hash routes
pub fn hash_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/hashes/{key}", get(hgetall).put(hset))
        .route("/api/v1/hashes/{key}/fields/{field}", get(hget))
        .route("/api/v1/hashes/{key}/set-nx", post(hset_nx))
        .route("/api/v1/hashes/{key}/fields/get", post(hmget))
        .route("/api/v1/hashes/{key}/fields", delete(hdel))
        .route("/api/v1/hashes/{key}/fields/{field}/exists", get(hexists))
        .route("/api/v1/hashes/{key}/keys", get(hkeys))
        .route("/api/v1/hashes/{key}/values", get(hvals))
        .route("/api/v1/hashes/{key}/length", get(hlen))
        .route("/api/v1/hashes/{key}/fields/{field}/incr", patch(hincr_by))
        .route(
            "/api/v1/hashes/{key}/fields/{field}/incr-float",
            patch(hincr_by_float),
        )
        .route("/api/v1/hashes/{key}/fields/{field}/length", get(hstr_len))
        .route("/api/v1/hashes/{key}/random", get(hrand_field))
        .route("/api/v1/hashes/{key}/scan", get(hscan))
        // Hash field expiration routes (Redis 7.4+)
        .route("/api/v1/hashes/{key}/fields/expire", post(hexpire))
        .route("/api/v1/hashes/{key}/fields/pexpire", post(hpexpire))
        .route("/api/v1/hashes/{key}/fields/expireat", post(hexpire_at))
        .route("/api/v1/hashes/{key}/fields/pexpireat", post(hpexpire_at))
        .route("/api/v1/hashes/{key}/fields/expiretime", post(hexpire_time))
        .route(
            "/api/v1/hashes/{key}/fields/pexpiretime",
            post(hpexpire_time),
        )
        .route("/api/v1/hashes/{key}/fields/ttl", post(httl))
        .route("/api/v1/hashes/{key}/fields/pttl", post(hpttl))
        .route("/api/v1/hashes/{key}/fields/persist", post(hpersist))
        // Redis 8.0+ hash commands
        .route("/api/v1/hashes/{key}/getex", post(hgetex))
        .route("/api/v1/hashes/{key}/setex", post(hsetex))
        .route("/api/v1/hashes/{key}/getdel", post(hgetdel))
}

/// GET /api/v1/hashes/{key}/fields/{field}
///
/// Get the value of a hash field (HGET).
#[utoipa::path(
    get,
    path = "/api/v1/hashes/{key}/fields/{field}",
    params(
        ("key" = String, Path, description = "The hash key"),
        ("field" = String, Path, description = "The field name")
    ),
    responses(
        (status = 200, description = "Field value retrieved", body = Option<String>),
        (status = 404, description = "Key or field not found")
    ),
    tag = "Hashes"
)]
pub async fn hget(
    State(state): State<AppState>,
    Path((key, field)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Option<String>>>, CacheError> {
    let value = state.hash_service.hget(&key, &field).await?;
    Ok(Json(ApiResponse::success(value)))
}

/// PUT /api/v1/hashes/{key}
///
/// Set multiple fields in a hash (HSET).
#[utoipa::path(
    put,
    path = "/api/v1/hashes/{key}",
    params(
        ("key" = String, Path, description = "The hash key")
    ),
    request_body = SetHashRequest,
    responses(
        (status = 200, description = "Fields set successfully", body = i64),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hset(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<SetHashRequest>,
) -> Result<Json<ApiResponse<i64>>, CacheError> {
    let pairs: Vec<(String, String)> = req.items.into_iter().collect();
    let count = state.hash_service.hset(&key, pairs).await?;
    Ok(Json(ApiResponse::success(count)))
}

/// POST /api/v1/hashes/{key}/set-nx
///
/// Set a field only if it doesn't exist (HSETNX).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/set-nx",
    params(
        ("key" = String, Path, description = "The hash key")
    ),
    request_body = SetHashNxRequest,
    responses(
        (status = 200, description = "Field set if new", body = bool),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hset_nx(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<SetHashNxRequest>,
) -> Result<Json<ApiResponse<bool>>, CacheError> {
    let result = state
        .hash_service
        .hset_nx(&key, &req.field, &req.value)
        .await?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/hashes/{key}
///
/// Get all fields and values in a hash (HGETALL).
#[utoipa::path(
    get,
    path = "/api/v1/hashes/{key}",
    params(
        ("key" = String, Path, description = "The hash key")
    ),
    responses(
        (status = 200, description = "All fields and values", body = std::collections::HashMap<String, String>),
        (status = 404, description = "Key not found")
    ),
    tag = "Hashes"
)]
pub async fn hgetall(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<std::collections::HashMap<String, String>>>, CacheError> {
    let result = state.hash_service.hgetall(&key).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// POST /api/v1/hashes/{key}/fields/get
///
/// Get multiple field values (HMGET).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/fields/get",
    params(
        ("key" = String, Path, description = "The hash key")
    ),
    request_body = GetMultipleFieldsRequest,
    responses(
        (status = 200, description = "Field values retrieved", body = Vec<Option<String>>),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hmget(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<GetMultipleFieldsRequest>,
) -> Result<Json<ApiResponse<Vec<Option<String>>>>, CacheError> {
    let result = state.hash_service.hmget(&key, req.fields).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// DELETE /api/v1/hashes/{key}/fields
///
/// Delete one or more fields from a hash (HDEL).
#[utoipa::path(
    delete,
    path = "/api/v1/hashes/{key}/fields",
    params(
        ("key" = String, Path, description = "The hash key")
    ),
    request_body = GetMultipleFieldsRequest,
    responses(
        (status = 200, description = "Number of fields deleted", body = i64),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hdel(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<GetMultipleFieldsRequest>,
) -> Result<Json<ApiResponse<i64>>, CacheError> {
    let count = state.hash_service.hdel(&key, req.fields).await?;
    Ok(Json(ApiResponse::success(count)))
}

/// GET /api/v1/hashes/{key}/fields/{field}/exists
///
/// Check if a field exists in a hash (HEXISTS).
#[utoipa::path(
    get,
    path = "/api/v1/hashes/{key}/fields/{field}/exists",
    params(
        ("key" = String, Path, description = "The hash key"),
        ("field" = String, Path, description = "The field name")
    ),
    responses(
        (status = 200, description = "Field existence check", body = bool)
    ),
    tag = "Hashes"
)]
pub async fn hexists(
    State(state): State<AppState>,
    Path((key, field)): Path<(String, String)>,
) -> Result<Json<ApiResponse<bool>>, CacheError> {
    let exists = state.hash_service.hexists(&key, &field).await?;
    Ok(Json(ApiResponse::success(exists)))
}

/// GET /api/v1/hashes/{key}/keys
///
/// Get all field names in a hash (HKEYS).
#[utoipa::path(
    get,
    path = "/api/v1/hashes/{key}/keys",
    params(
        ("key" = String, Path, description = "The hash key")
    ),
    responses(
        (status = 200, description = "List of field names", body = Vec<String>)
    ),
    tag = "Hashes"
)]
pub async fn hkeys(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<Vec<String>>>, CacheError> {
    let result = state.hash_service.hkeys(&key).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/hashes/{key}/values
///
/// Get all values in a hash (HVALS).
#[utoipa::path(
    get,
    path = "/api/v1/hashes/{key}/values",
    params(
        ("key" = String, Path, description = "The hash key")
    ),
    responses(
        (status = 200, description = "List of values", body = Vec<String>)
    ),
    tag = "Hashes"
)]
pub async fn hvals(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<Vec<String>>>, CacheError> {
    let result = state.hash_service.hvals(&key).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/hashes/{key}/length
///
/// Get the number of fields in a hash (HLEN).
#[utoipa::path(
    get,
    path = "/api/v1/hashes/{key}/length",
    params(
        ("key" = String, Path, description = "The hash key")
    ),
    responses(
        (status = 200, description = "Number of fields", body = i64)
    ),
    tag = "Hashes"
)]
pub async fn hlen(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<i64>>, CacheError> {
    let result = state.hash_service.hlen(&key).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// PATCH /api/v1/hashes/{key}/fields/{field}/incr
///
/// Increment a hash field by integer value (HINCRBY).
#[utoipa::path(
    patch,
    path = "/api/v1/hashes/{key}/fields/{field}/incr",
    params(
        ("key" = String, Path, description = "The hash key"),
        ("field" = String, Path, description = "The field to increment")
    ),
    request_body = HashIncrRequest,
    responses(
        (status = 200, description = "New value after increment", body = i64),
        (status = 400, description = "Field value is not an integer")
    ),
    tag = "Hashes"
)]
pub async fn hincr_by(
    State(state): State<AppState>,
    Path((key, field)): Path<(String, String)>,
    Json(req): Json<HashIncrRequest>,
) -> Result<Json<ApiResponse<i64>>, CacheError> {
    let result = state.hash_service.hincr_by(&key, &field, req.delta).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// PATCH /api/v1/hashes/{key}/fields/{field}/incr-float
///
/// Increment a hash field by float value (HINCRBYFLOAT).
#[utoipa::path(
    patch,
    path = "/api/v1/hashes/{key}/fields/{field}/incr-float",
    params(
        ("key" = String, Path, description = "The hash key"),
        ("field" = String, Path, description = "The field to increment")
    ),
    request_body = HashIncrFloatRequest,
    responses(
        (status = 200, description = "New value after increment", body = f64),
        (status = 400, description = "Field value is not a number")
    ),
    tag = "Hashes"
)]
pub async fn hincr_by_float(
    State(state): State<AppState>,
    Path((key, field)): Path<(String, String)>,
    Json(req): Json<HashIncrFloatRequest>,
) -> Result<Json<ApiResponse<f64>>, CacheError> {
    let result = state
        .hash_service
        .hincr_by_float(&key, &field, req.delta)
        .await?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/hashes/{key}/fields/{field}/length
///
/// Get the length of a hash field value (HSTRLEN).
#[utoipa::path(
    get,
    path = "/api/v1/hashes/{key}/fields/{field}/length",
    params(
        ("key" = String, Path, description = "The hash key"),
        ("field" = String, Path, description = "The field name")
    ),
    responses(
        (status = 200, description = "Length of field value", body = i64)
    ),
    tag = "Hashes"
)]
pub async fn hstr_len(
    State(state): State<AppState>,
    Path((key, field)): Path<(String, String)>,
) -> Result<Json<ApiResponse<i64>>, CacheError> {
    let result = state.hash_service.hstr_len(&key, &field).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/hashes/{key}/random
///
/// Get random field(s) from a hash (HRANDFIELD).
#[utoipa::path(
    get,
    path = "/api/v1/hashes/{key}/random",
    params(
        ("key" = String, Path, description = "The hash key"),
        ("count" = Option<i64>, Query, description = "Number of fields to return"),
        ("with_values" = bool, Query, description = "Include values (requires count)")
    ),
    responses(
        (status = 200, description = "Random field(s)", body = HashRandomFieldResponse),
        (status = 400, description = "Invalid request (with_values requires count)")
    ),
    tag = "Hashes"
)]
pub async fn hrand_field(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<RandomFieldQuery>,
) -> Result<Json<ApiResponse<HashRandomFieldResponse>>, CacheError> {
    let result = state
        .hash_service
        .hrand_field(&key, query.count, query.with_values)
        .await?;
    if query.with_values {
        let entries = to_entries(result);
        Ok(Json(ApiResponse::success(HashRandomFieldResponse {
            fields: None,
            entries: Some(entries),
        })))
    } else {
        Ok(Json(ApiResponse::success(HashRandomFieldResponse {
            fields: Some(result),
            entries: None,
        })))
    }
}

/// GET /api/v1/hashes/{key}/scan
///
/// Incrementally iterate over hash fields (HSCAN).
#[utoipa::path(
    get,
    path = "/api/v1/hashes/{key}/scan",
    params(
        ("key" = String, Path, description = "The hash key"),
        ("cursor" = u64, Query, description = "Cursor position (0 to start)"),
        ("pattern" = Option<String>, Query, description = "Pattern to match fields"),
        ("count" = Option<u64>, Query, description = "Hint for number of items to return")
    ),
    responses(
        (status = 200, description = "Scan result with cursor and entries", body = HashScanResponse)
    ),
    tag = "Hashes"
)]
pub async fn hscan(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<ScanHashQuery>,
) -> Result<Json<ApiResponse<HashScanResponse>>, CacheError> {
    let (cursor, items) = state
        .hash_service
        .hscan(&key, query.cursor, query.pattern, query.count)
        .await?;
    let entries = to_entries(items);
    Ok(Json(ApiResponse::success(HashScanResponse {
        cursor,
        entries,
    })))
}

/// Check that hash field expiration is supported (Redis 7.4+)
fn require_hash_field_expiration(state: &AppState) -> Result<(), CacheError> {
    if !state.capabilities.features.hash_field_expiration {
        return Err(CacheError::ModuleNotAvailable(
            "Hash field expiration requires Redis 7.4+".to_string(),
        ));
    }
    Ok(())
}

/// Helper to zip field names with result codes into HExpireFieldResult.
fn zip_field_results(fields: &[String], results: &[i64]) -> Vec<HExpireFieldResult> {
    fields
        .iter()
        .zip(results.iter())
        .map(|(field, &result)| HExpireFieldResult {
            field: field.clone(),
            result,
        })
        .collect()
}

/// POST /api/v1/hashes/{key}/fields/expire
///
/// Set expiration (in seconds) on hash fields (HEXPIRE, Redis 7.4+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/fields/expire",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HExpireRequest,
    responses(
        (status = 200, description = "Expiration set results", body = HExpireResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hexpire(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HExpireRequest>,
) -> Result<Json<ApiResponse<HExpireResponse>>, CacheError> {
    require_hash_field_expiration(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let condition = req.condition.map(Into::into);
    let results = state
        .hash_service
        .hexpire(&key, req.seconds, req.fields.clone(), condition)
        .await?;
    Ok(Json(ApiResponse::success(HExpireResponse {
        results: zip_field_results(&req.fields, &results),
    })))
}

/// POST /api/v1/hashes/{key}/fields/pexpire
///
/// Set expiration (in milliseconds) on hash fields (HPEXPIRE, Redis 7.4+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/fields/pexpire",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HPExpireRequest,
    responses(
        (status = 200, description = "Expiration set results", body = HExpireResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hpexpire(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HPExpireRequest>,
) -> Result<Json<ApiResponse<HExpireResponse>>, CacheError> {
    require_hash_field_expiration(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let condition = req.condition.map(Into::into);
    let results = state
        .hash_service
        .hpexpire(&key, req.milliseconds, req.fields.clone(), condition)
        .await?;
    Ok(Json(ApiResponse::success(HExpireResponse {
        results: zip_field_results(&req.fields, &results),
    })))
}

/// POST /api/v1/hashes/{key}/fields/expireat
///
/// Set expiration as unix timestamp (seconds) on hash fields (HEXPIREAT, Redis 7.4+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/fields/expireat",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HExpireAtRequest,
    responses(
        (status = 200, description = "Expiration set results", body = HExpireResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hexpire_at(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HExpireAtRequest>,
) -> Result<Json<ApiResponse<HExpireResponse>>, CacheError> {
    require_hash_field_expiration(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let condition = req.condition.map(Into::into);
    let results = state
        .hash_service
        .hexpire_at(&key, req.unix_time, req.fields.clone(), condition)
        .await?;
    Ok(Json(ApiResponse::success(HExpireResponse {
        results: zip_field_results(&req.fields, &results),
    })))
}

/// POST /api/v1/hashes/{key}/fields/pexpireat
///
/// Set expiration as unix timestamp (milliseconds) on hash fields (HPEXPIREAT, Redis 7.4+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/fields/pexpireat",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HPExpireAtRequest,
    responses(
        (status = 200, description = "Expiration set results", body = HExpireResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hpexpire_at(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HPExpireAtRequest>,
) -> Result<Json<ApiResponse<HExpireResponse>>, CacheError> {
    require_hash_field_expiration(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let condition = req.condition.map(Into::into);
    let results = state
        .hash_service
        .hpexpire_at(&key, req.unix_time_ms, req.fields.clone(), condition)
        .await?;
    Ok(Json(ApiResponse::success(HExpireResponse {
        results: zip_field_results(&req.fields, &results),
    })))
}

/// POST /api/v1/hashes/{key}/fields/expiretime
///
/// Get expiration unix timestamp (seconds) of hash fields (HEXPIRETIME, Redis 7.4+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/fields/expiretime",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HFieldsRequest,
    responses(
        (status = 200, description = "Expiration time results", body = HExpireResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hexpire_time(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HFieldsRequest>,
) -> Result<Json<ApiResponse<HExpireResponse>>, CacheError> {
    require_hash_field_expiration(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let results = state
        .hash_service
        .hexpire_time(&key, req.fields.clone())
        .await?;
    Ok(Json(ApiResponse::success(HExpireResponse {
        results: zip_field_results(&req.fields, &results),
    })))
}

/// POST /api/v1/hashes/{key}/fields/pexpiretime
///
/// Get expiration unix timestamp (milliseconds) of hash fields (HPEXPIRETIME, Redis 7.4+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/fields/pexpiretime",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HFieldsRequest,
    responses(
        (status = 200, description = "Expiration time results", body = HExpireResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hpexpire_time(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HFieldsRequest>,
) -> Result<Json<ApiResponse<HExpireResponse>>, CacheError> {
    require_hash_field_expiration(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let results = state
        .hash_service
        .hpexpire_time(&key, req.fields.clone())
        .await?;
    Ok(Json(ApiResponse::success(HExpireResponse {
        results: zip_field_results(&req.fields, &results),
    })))
}

/// POST /api/v1/hashes/{key}/fields/ttl
///
/// Get TTL (seconds) of hash fields (HTTL, Redis 7.4+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/fields/ttl",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HFieldsRequest,
    responses(
        (status = 200, description = "TTL results", body = HExpireResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn httl(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HFieldsRequest>,
) -> Result<Json<ApiResponse<HExpireResponse>>, CacheError> {
    require_hash_field_expiration(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let results = state.hash_service.httl(&key, req.fields.clone()).await?;
    Ok(Json(ApiResponse::success(HExpireResponse {
        results: zip_field_results(&req.fields, &results),
    })))
}

/// POST /api/v1/hashes/{key}/fields/pttl
///
/// Get TTL (milliseconds) of hash fields (HPTTL, Redis 7.4+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/fields/pttl",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HFieldsRequest,
    responses(
        (status = 200, description = "TTL results", body = HExpireResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hpttl(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HFieldsRequest>,
) -> Result<Json<ApiResponse<HExpireResponse>>, CacheError> {
    require_hash_field_expiration(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let results = state.hash_service.hpttl(&key, req.fields.clone()).await?;
    Ok(Json(ApiResponse::success(HExpireResponse {
        results: zip_field_results(&req.fields, &results),
    })))
}

/// POST /api/v1/hashes/{key}/fields/persist
///
/// Remove expiration from hash fields (HPERSIST, Redis 7.4+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/fields/persist",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HFieldsRequest,
    responses(
        (status = 200, description = "Persist results", body = HExpireResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Hashes"
)]
pub async fn hpersist(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HFieldsRequest>,
) -> Result<Json<ApiResponse<HExpireResponse>>, CacheError> {
    require_hash_field_expiration(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let results = state
        .hash_service
        .hpersist(&key, req.fields.clone())
        .await?;
    Ok(Json(ApiResponse::success(HExpireResponse {
        results: zip_field_results(&req.fields, &results),
    })))
}

/// Check that Redis 8.0+ hash commands are supported.
fn require_redis8_hash(state: &AppState) -> Result<(), CacheError> {
    if !state.capabilities.features.hash_8_commands {
        return Err(CacheError::ModuleNotAvailable(
            "HGETEX/HSETEX/HGETDEL require Redis 8.0+".to_string(),
        ));
    }
    Ok(())
}

/// POST /api/v1/hashes/{key}/getex
///
/// Get field values and optionally set/remove their expiration atomically (HGETEX, Redis 8.0+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/getex",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HGetExRequest,
    responses(
        (status = 200, description = "Field values with optional expiration change", body = HGetExResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "Redis 8.0+ required")
    ),
    tag = "Hashes"
)]
pub async fn hgetex(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HGetExRequest>,
) -> Result<Json<ApiResponse<HGetExResponse>>, CacheError> {
    require_redis8_hash(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let expiration = req.expiration.map(Into::into);
    let values = state
        .hash_service
        .hgetex(&key, req.fields, expiration)
        .await?;
    Ok(Json(ApiResponse::success(HGetExResponse { values })))
}

/// POST /api/v1/hashes/{key}/setex
///
/// Set fields with optional condition and expiration (HSETEX, Redis 8.0+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/setex",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HSetExRequest,
    responses(
        (status = 200, description = "Number of fields set", body = HSetExResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "Redis 8.0+ required")
    ),
    tag = "Hashes"
)]
pub async fn hsetex(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HSetExRequest>,
) -> Result<Json<ApiResponse<HSetExResponse>>, CacheError> {
    require_redis8_hash(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let condition = req.condition.map(Into::into);
    let expiration = req.expiration.map(Into::into);
    let field_values: Vec<(String, String)> = req.fields.into_iter().collect();
    let count = state
        .hash_service
        .hsetex(&key, field_values, condition, expiration)
        .await?;
    Ok(Json(ApiResponse::success(HSetExResponse { count })))
}

/// POST /api/v1/hashes/{key}/getdel
///
/// Get field values and delete them atomically (HGETDEL, Redis 8.0+).
#[utoipa::path(
    post,
    path = "/api/v1/hashes/{key}/getdel",
    params(("key" = String, Path, description = "The hash key")),
    request_body = HGetDelRequest,
    responses(
        (status = 200, description = "Field values before deletion", body = HGetDelResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "Redis 8.0+ required")
    ),
    tag = "Hashes"
)]
pub async fn hgetdel(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<HGetDelRequest>,
) -> Result<Json<ApiResponse<HGetDelResponse>>, CacheError> {
    require_redis8_hash(&state)?;
    req.validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let values = state.hash_service.hgetdel(&key, req.fields).await?;
    Ok(Json(ApiResponse::success(HGetDelResponse { values })))
}

fn to_entries(items: Vec<String>) -> Vec<HashFieldEntry> {
    let mut entries = Vec::new();
    let mut iter = items.into_iter();
    while let (Some(field), Some(value)) = (iter.next(), iter.next()) {
        entries.push(HashFieldEntry { field, value });
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http::schemas::hashes::{
        ExpireConditionSchema, HExpireAtRequest, HExpireRequest, HFieldsRequest, HPExpireAtRequest,
        HPExpireRequest,
    };
    use crate::test_support::test_state_with_hash_repo;
    use axum::Json;
    use axum::extract::{Path, Query, State};

    #[tokio::test]
    async fn test_hash_routes_basic() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("hash1", "field1", "1");
        hash_repo.insert("hash1", "field2", "2");
        let state = State(state);

        let response = hget(
            state.clone(),
            Path(("hash1".to_string(), "field1".to_string())),
        )
        .await
        .unwrap();
        let value = response.0.data.expect("data");
        assert_eq!(value.as_deref(), Some("1"));

        let response = hget(
            state.clone(),
            Path(("hash1".to_string(), "missing".to_string())),
        )
        .await
        .unwrap();
        let value = response.0.data.expect("data");
        assert!(value.is_none());

        let response = hset(
            state.clone(),
            Path("hash1".to_string()),
            Json(SetHashRequest {
                items: [("field3".to_string(), "3".to_string())]
                    .into_iter()
                    .collect(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data"), 1);

        let response = hset_nx(
            state.clone(),
            Path("hash1".to_string()),
            Json(SetHashNxRequest {
                field: "field3".to_string(),
                value: "4".to_string(),
            }),
        )
        .await
        .unwrap();
        assert!(!response.0.data.expect("data"));

        let response = hgetall(state.clone(), Path("hash1".to_string()))
            .await
            .unwrap();
        assert_eq!(response.0.data.expect("data").len(), 3);

        let response = hmget(
            state.clone(),
            Path("hash1".to_string()),
            Json(GetMultipleFieldsRequest {
                fields: vec!["field1".to_string(), "missing".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data").len(), 2);

        let response = hdel(
            state.clone(),
            Path("hash1".to_string()),
            Json(GetMultipleFieldsRequest {
                fields: vec!["field2".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data"), 1);

        let response = hexists(
            state.clone(),
            Path(("hash1".to_string(), "field1".to_string())),
        )
        .await
        .unwrap();
        assert!(response.0.data.expect("data"));

        let response = hkeys(state.clone(), Path("hash1".to_string()))
            .await
            .unwrap();
        assert!(
            response
                .0
                .data
                .expect("data")
                .contains(&"field1".to_string())
        );

        let response = hvals(state.clone(), Path("hash1".to_string()))
            .await
            .unwrap();
        assert!(response.0.data.expect("data").contains(&"1".to_string()));

        let response = hlen(state.clone(), Path("hash1".to_string()))
            .await
            .unwrap();
        assert_eq!(response.0.data.expect("data"), 2);

        let response = hincr_by(
            state.clone(),
            Path(("hash1".to_string(), "counter".to_string())),
            Json(HashIncrRequest { delta: 5 }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data"), 5);

        let response = hincr_by_float(
            state.clone(),
            Path(("hash1".to_string(), "float".to_string())),
            Json(HashIncrFloatRequest { delta: 1.5 }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data"), 1.5);

        let response = hstr_len(state, Path(("hash1".to_string(), "field1".to_string())))
            .await
            .unwrap();
        assert_eq!(response.0.data.expect("data"), 1);
    }

    #[tokio::test]
    async fn test_hash_routes_random_and_scan() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("hash2", "alpha", "one");
        hash_repo.insert("hash2", "beta", "two");
        let state = State(state);

        let response = hrand_field(
            state.clone(),
            Path("hash2".to_string()),
            Query(RandomFieldQuery {
                count: None,
                with_values: false,
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data").fields.unwrap().len(), 1);

        let response = hrand_field(
            state.clone(),
            Path("hash2".to_string()),
            Query(RandomFieldQuery {
                count: Some(1),
                with_values: true,
            }),
        )
        .await
        .unwrap();
        let entries = response.0.data.expect("data").entries.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].field.is_empty());

        let err = hrand_field(
            state.clone(),
            Path("hash2".to_string()),
            Query(RandomFieldQuery {
                count: None,
                with_values: true,
            }),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let response = hscan(
            state,
            Path("hash2".to_string()),
            Query(ScanHashQuery {
                cursor: 0,
                pattern: Some("a".to_string()),
                count: Some(10),
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.cursor, 0);
        assert!(!data.entries.is_empty());
    }

    #[tokio::test]
    async fn test_hexpire_route_success() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        hash_repo.insert("h1", "f2", "v2");
        let state = State(state);

        let response = hexpire(
            state.clone(),
            Path("h1".to_string()),
            Json(HExpireRequest {
                fields: vec!["f1".to_string(), "f2".to_string()],
                seconds: 60,
                condition: None,
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.results.len(), 2);
        assert_eq!(data.results[0].field, "f1");
        assert_eq!(data.results[0].result, 1);
        assert_eq!(data.results[1].field, "f2");
        assert_eq!(data.results[1].result, 1);
    }

    #[tokio::test]
    async fn test_hexpire_route_with_condition() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        let state = State(state);

        let response = hexpire(
            state,
            Path("h1".to_string()),
            Json(HExpireRequest {
                fields: vec!["f1".to_string()],
                seconds: 60,
                condition: Some(ExpireConditionSchema::Nx),
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.results[0].result, 1);
    }

    #[tokio::test]
    async fn test_hpexpire_route_success() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        let state = State(state);

        let response = hpexpire(
            state,
            Path("h1".to_string()),
            Json(HPExpireRequest {
                fields: vec!["f1".to_string()],
                milliseconds: 60000,
                condition: None,
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.results.len(), 1);
        assert_eq!(data.results[0].result, 1);
    }

    #[tokio::test]
    async fn test_hexpire_at_route_success() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        let state = State(state);

        let response = hexpire_at(
            state,
            Path("h1".to_string()),
            Json(HExpireAtRequest {
                fields: vec!["f1".to_string()],
                unix_time: 1700000000,
                condition: None,
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.results[0].result, 1);
    }

    #[tokio::test]
    async fn test_hpexpire_at_route_success() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        let state = State(state);

        let response = hpexpire_at(
            state,
            Path("h1".to_string()),
            Json(HPExpireAtRequest {
                fields: vec!["f1".to_string()],
                unix_time_ms: 1700000000000,
                condition: None,
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.results[0].result, 1);
    }

    #[tokio::test]
    async fn test_hexpire_time_route_success() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        let state = State(state);

        let response = hexpire_time(
            state,
            Path("h1".to_string()),
            Json(HFieldsRequest {
                fields: vec!["f1".to_string()],
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.results[0].result, -1);
    }

    #[tokio::test]
    async fn test_hpexpire_time_route_success() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        let state = State(state);

        let response = hpexpire_time(
            state,
            Path("h1".to_string()),
            Json(HFieldsRequest {
                fields: vec!["f1".to_string()],
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.results[0].result, -1);
    }

    #[tokio::test]
    async fn test_httl_route_success() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        let state = State(state);

        let response = httl(
            state,
            Path("h1".to_string()),
            Json(HFieldsRequest {
                fields: vec!["f1".to_string()],
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.results[0].field, "f1");
        assert_eq!(data.results[0].result, -1);
    }

    #[tokio::test]
    async fn test_hpttl_route_success() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        let state = State(state);

        let response = hpttl(
            state,
            Path("h1".to_string()),
            Json(HFieldsRequest {
                fields: vec!["f1".to_string()],
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.results[0].result, -1);
    }

    #[tokio::test]
    async fn test_hpersist_route_success() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        let state = State(state);

        let response = hpersist(
            state,
            Path("h1".to_string()),
            Json(HFieldsRequest {
                fields: vec!["f1".to_string()],
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.results[0].result, 1);
    }

    #[tokio::test]
    async fn test_zip_field_results_helper() {
        let fields = vec!["f1".to_string(), "f2".to_string()];
        let results = vec![1, -1];
        let zipped = zip_field_results(&fields, &results);
        assert_eq!(zipped.len(), 2);
        assert_eq!(zipped[0].field, "f1");
        assert_eq!(zipped[0].result, 1);
        assert_eq!(zipped[1].field, "f2");
        assert_eq!(zipped[1].result, -1);
    }

    #[tokio::test]
    async fn test_hexpire_returns_501_when_feature_disabled() {
        let (mut app_state, _) = test_state_with_hash_repo();
        let mut caps = (*app_state.capabilities).clone();
        caps.features.hash_field_expiration = false;
        app_state.capabilities = std::sync::Arc::new(caps);
        let state = State(app_state);

        let req = HExpireRequest {
            fields: vec!["f1".into()],
            seconds: 60,
            condition: None,
        };
        let result = hexpire(state, Path("mykey".into()), Json(req)).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_httl_returns_501_when_feature_disabled() {
        let (mut app_state, _) = test_state_with_hash_repo();
        let mut caps = (*app_state.capabilities).clone();
        caps.features.hash_field_expiration = false;
        app_state.capabilities = std::sync::Arc::new(caps);
        let state = State(app_state);

        let req = HFieldsRequest {
            fields: vec!["f1".into()],
        };
        let result = httl(state, Path("mykey".into()), Json(req)).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    // --- Redis 8.0+ route tests ---

    #[tokio::test]
    async fn test_hgetex_route_success() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        hash_repo.insert("h1", "f2", "v2");
        let state = State(state);

        let response = hgetex(
            state,
            Path("h1".to_string()),
            Json(crate::api::http::schemas::hashes::HGetExRequest {
                fields: vec!["f1".to_string(), "f2".to_string(), "missing".to_string()],
                expiration: None,
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.values.len(), 3);
        assert_eq!(data.values[0].as_deref(), Some("v1"));
        assert_eq!(data.values[1].as_deref(), Some("v2"));
        assert!(data.values[2].is_none());
    }

    #[tokio::test]
    async fn test_hgetex_route_with_expiration() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        let state = State(state);

        let response = hgetex(
            state,
            Path("h1".to_string()),
            Json(crate::api::http::schemas::hashes::HGetExRequest {
                fields: vec!["f1".to_string()],
                expiration: Some(crate::api::http::schemas::hashes::HGetExExpirationSchema::Ex(60)),
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.values[0].as_deref(), Some("v1"));
    }

    #[tokio::test]
    async fn test_hgetex_returns_501_when_feature_disabled() {
        let (mut app_state, _) = test_state_with_hash_repo();
        let mut caps = (*app_state.capabilities).clone();
        caps.features.hash_8_commands = false;
        app_state.capabilities = std::sync::Arc::new(caps);
        let state = State(app_state);

        let req = crate::api::http::schemas::hashes::HGetExRequest {
            fields: vec!["f1".into()],
            expiration: None,
        };
        let result = hgetex(state, Path("mykey".into()), Json(req)).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_hsetex_route_success() {
        let (state, _hash_repo) = test_state_with_hash_repo();
        let state = State(state);

        let mut fields = std::collections::HashMap::new();
        fields.insert("f1".to_string(), "v1".to_string());
        fields.insert("f2".to_string(), "v2".to_string());

        let response = hsetex(
            state,
            Path("h1".to_string()),
            Json(crate::api::http::schemas::hashes::HSetExRequest {
                fields,
                condition: None,
                expiration: None,
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.count, 2);
    }

    #[tokio::test]
    async fn test_hsetex_route_with_condition_and_expiration() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "old");
        let state = State(state);

        let mut fields = std::collections::HashMap::new();
        fields.insert("f1".to_string(), "new".to_string());
        fields.insert("f2".to_string(), "v2".to_string());

        let response = hsetex(
            state,
            Path("h1".to_string()),
            Json(crate::api::http::schemas::hashes::HSetExRequest {
                fields,
                condition: Some(crate::api::http::schemas::hashes::HSetExConditionSchema::Fnx),
                expiration: Some(crate::api::http::schemas::hashes::HSetExExpirationSchema::Ex(60)),
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        // FNX: f1 exists so not set, f2 new so set
        assert_eq!(data.count, 1);
    }

    #[tokio::test]
    async fn test_hsetex_returns_501_when_feature_disabled() {
        let (mut app_state, _) = test_state_with_hash_repo();
        let mut caps = (*app_state.capabilities).clone();
        caps.features.hash_8_commands = false;
        app_state.capabilities = std::sync::Arc::new(caps);
        let state = State(app_state);

        let mut fields = std::collections::HashMap::new();
        fields.insert("f1".to_string(), "v1".to_string());

        let req = crate::api::http::schemas::hashes::HSetExRequest {
            fields,
            condition: None,
            expiration: None,
        };
        let result = hsetex(state, Path("mykey".into()), Json(req)).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_hgetdel_route_success() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("h1", "f1", "v1");
        hash_repo.insert("h1", "f2", "v2");
        let state = State(state);

        let response = hgetdel(
            state,
            Path("h1".to_string()),
            Json(crate::api::http::schemas::hashes::HGetDelRequest {
                fields: vec!["f1".to_string(), "missing".to_string()],
            }),
        )
        .await
        .unwrap();
        let data = response.0.data.expect("data");
        assert_eq!(data.values.len(), 2);
        assert_eq!(data.values[0].as_deref(), Some("v1"));
        assert!(data.values[1].is_none());
    }

    #[tokio::test]
    async fn test_hgetdel_returns_501_when_feature_disabled() {
        let (mut app_state, _) = test_state_with_hash_repo();
        let mut caps = (*app_state.capabilities).clone();
        caps.features.hash_8_commands = false;
        app_state.capabilities = std::sync::Arc::new(caps);
        let state = State(app_state);

        let req = crate::api::http::schemas::hashes::HGetDelRequest {
            fields: vec!["f1".into()],
        };
        let result = hgetdel(state, Path("mykey".into()), Json(req)).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }
}
