//! String Routes
//!
//! HTTP endpoints for Redis string operations.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, patch, post, put},
    Json, Router,
};

use crate::api::http::schemas::strings::{
    AppendRequest, AppendResponse, GetDelResponse, GetExParams, GetRangeParams,
    GetRangeResponse, IncrementRequest, IncrementResponse, MGetRequest, MGetResponse,
    MSetRequest, MSetResponse, SetRangeRequest, SetRangeResponse, SetStringRequest,
    SetStringResponse, StrLenResponse,
};
use crate::application::services::StringService;
use crate::domain::entities::StringValue;
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create string routes
pub fn string_routes() -> Router<AppState> {
    Router::new()
        // Single key operations
        .route("/api/v1/strings/{key}", get(get_string))
        .route("/api/v1/strings/{key}", put(set_string))
        .route("/api/v1/strings/{key}", delete(get_del_string))
        // Multi-key operations
        .route("/api/v1/strings/mget", post(mget_strings))
        .route("/api/v1/strings/mset", post(mset_strings))
        // Increment/Decrement
        .route("/api/v1/strings/{key}/incr", patch(incr_string))
        .route("/api/v1/strings/{key}/decr", patch(decr_string))
        // String manipulation
        .route("/api/v1/strings/{key}/append", patch(append_string))
        .route("/api/v1/strings/{key}/length", get(strlen_string))
        // Range operations
        .route("/api/v1/strings/{key}/range", get(get_range))
        .route("/api/v1/strings/{key}/range", patch(set_range))
        // GETEX
        .route("/api/v1/strings/{key}/getex", get(get_ex_string))
}

/// GET /api/v1/strings/:key
///
/// Get the value of a string key.
#[utoipa::path(
    get,
    path = "/api/v1/strings/{key}",
    params(
        ("key" = String, Path, description = "The key to retrieve")
    ),
    responses(
        (status = 200, description = "Key found", body = StringValue),
        (status = 404, description = "Key not found")
    ),
    tag = "Strings"
)]
async fn get_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<StringValue>>, CacheError> {
    let service = StringService::new(state.pool.clone());

    service
        .get(&key)
        .await?
        .map_or_else(
            || Err(CacheError::KeyNotFound(key)),
            |value| Ok(Json(ApiResponse::new(value))),
        )
}

/// PUT /api/v1/strings/:key
///
/// Set the value of a string key with options.
#[utoipa::path(
    put,
    path = "/api/v1/strings/{key}",
    params(
        ("key" = String, Path, description = "The key to set")
    ),
    request_body = SetStringRequest,
    responses(
        (status = 200, description = "Key set successfully", body = SetStringResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Strings"
)]
async fn set_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SetStringRequest>,
) -> Result<Json<ApiResponse<SetStringResponse>>, CacheError> {
    let service = StringService::new(state.pool.clone());

    let result = service
        .set(
            &key,
            &request.value,
            request.ttl_seconds,
            request.ttl_ms,
            request.nx,
            request.xx,
            request.get,
            request.keep_ttl,
        )
        .await?;

    Ok(Json(ApiResponse::new(SetStringResponse {
        key: result.key,
        success: result.success,
        previous_value: result.previous_value,
    })))
}

/// DELETE /api/v1/strings/:key
///
/// Get the value and delete the key (GETDEL).
#[utoipa::path(
    delete,
    path = "/api/v1/strings/{key}",
    params(
        ("key" = String, Path, description = "The key to delete")
    ),
    responses(
        (status = 200, description = "Key deleted", body = GetDelResponse),
        (status = 404, description = "Key not found")
    ),
    tag = "Strings"
)]
async fn get_del_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<GetDelResponse>>, CacheError> {
    let service = StringService::new(state.pool.clone());

    let value = service.get_del(&key).await?;

    Ok(Json(ApiResponse::new(GetDelResponse {
        key: key.clone(),
        existed: value.is_some(),
        value,
    })))
}

/// POST /api/v1/strings/mget
///
/// Get multiple string values at once.
#[utoipa::path(
    post,
    path = "/api/v1/strings/mget",
    request_body = MGetRequest,
    responses(
        (status = 200, description = "Values retrieved", body = MGetResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Strings"
)]
async fn mget_strings(
    State(state): State<AppState>,
    Json(request): Json<MGetRequest>,
) -> Result<Json<ApiResponse<MGetResponse>>, CacheError> {
    let service = StringService::new(state.pool.clone());
    let total_requested = request.keys.len();

    let result = service.mget(request.keys).await?;

    Ok(Json(ApiResponse::new(MGetResponse {
        found_count: result.found.len(),
        found: result.found,
        missing: result.missing,
        total_requested,
    })))
}

/// POST /api/v1/strings/mset
///
/// Set multiple string values at once.
#[utoipa::path(
    post,
    path = "/api/v1/strings/mset",
    request_body = MSetRequest,
    responses(
        (status = 200, description = "Values set", body = MSetResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Strings"
)]
async fn mset_strings(
    State(state): State<AppState>,
    Json(request): Json<MSetRequest>,
) -> Result<Json<ApiResponse<MSetResponse>>, CacheError> {
    let service = StringService::new(state.pool.clone());

    let pairs: Vec<(String, String)> = request.pairs.into_iter().collect();
    let keys: Vec<String> = pairs.iter().map(|(k, _)| k.clone()).collect();

    if request.nx {
        let success = service.mset_nx(pairs).await?;
        Ok(Json(ApiResponse::new(MSetResponse {
            count: if success { keys.len() } else { 0 },
            keys: if success { keys } else { vec![] },
            success,
        })))
    } else {
        let count = service.mset(pairs).await?;
        Ok(Json(ApiResponse::new(MSetResponse {
            count,
            keys,
            success: true,
        })))
    }
}

/// PATCH /api/v1/strings/:key/incr
///
/// Increment a numeric string value.
#[utoipa::path(
    patch,
    path = "/api/v1/strings/{key}/incr",
    params(
        ("key" = String, Path, description = "The key to increment")
    ),
    request_body = IncrementRequest,
    responses(
        (status = 200, description = "Value incremented", body = IncrementResponse),
        (status = 400, description = "Value is not a number")
    ),
    tag = "Strings"
)]
async fn incr_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<IncrementRequest>,
) -> Result<Json<ApiResponse<IncrementResponse>>, CacheError> {
    let service = StringService::new(state.pool.clone());

    let new_value = if request.float {
        #[allow(clippy::cast_precision_loss)] // Integer to float conversion is intentional
        let delta = request.float_delta.unwrap_or(request.delta as f64);
        let result = service.incr_by_float(&key, delta).await?;
        result.to_string()
    } else {
        let result = service.incr_by(&key, request.delta).await?;
        result.to_string()
    };

    Ok(Json(ApiResponse::new(IncrementResponse {
        key,
        new_value,
    })))
}

/// PATCH /api/v1/strings/:key/decr
///
/// Decrement a numeric string value.
#[utoipa::path(
    patch,
    path = "/api/v1/strings/{key}/decr",
    params(
        ("key" = String, Path, description = "The key to decrement")
    ),
    request_body = IncrementRequest,
    responses(
        (status = 200, description = "Value decremented", body = IncrementResponse),
        (status = 400, description = "Value is not a number")
    ),
    tag = "Strings"
)]
async fn decr_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<IncrementRequest>,
) -> Result<Json<ApiResponse<IncrementResponse>>, CacheError> {
    let service = StringService::new(state.pool.clone());

    let new_value = service.decr_by(&key, request.delta).await?;

    Ok(Json(ApiResponse::new(IncrementResponse {
        key,
        new_value: new_value.to_string(),
    })))
}

/// PATCH /api/v1/strings/:key/append
///
/// Append a value to an existing string.
#[utoipa::path(
    patch,
    path = "/api/v1/strings/{key}/append",
    params(
        ("key" = String, Path, description = "The key to append to")
    ),
    request_body = AppendRequest,
    responses(
        (status = 200, description = "Value appended", body = AppendResponse)
    ),
    tag = "Strings"
)]
async fn append_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<AppendRequest>,
) -> Result<Json<ApiResponse<AppendResponse>>, CacheError> {
    let service = StringService::new(state.pool.clone());

    let result = service.append(&key, &request.value).await?;

    Ok(Json(ApiResponse::new(AppendResponse {
        key: result.key,
        new_length: result.new_length,
    })))
}

/// GET /api/v1/strings/:key/length
///
/// Get the length of a string value.
#[utoipa::path(
    get,
    path = "/api/v1/strings/{key}/length",
    params(
        ("key" = String, Path, description = "The key to get length of")
    ),
    responses(
        (status = 200, description = "Length retrieved", body = StrLenResponse)
    ),
    tag = "Strings"
)]
async fn strlen_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<StrLenResponse>>, CacheError> {
    let service = StringService::new(state.pool.clone());

    let length = service.str_len(&key).await?;

    Ok(Json(ApiResponse::new(StrLenResponse { key, length })))
}

/// GET /api/v1/strings/:key/range
///
/// Get a substring of a string value.
#[utoipa::path(
    get,
    path = "/api/v1/strings/{key}/range",
    params(
        ("key" = String, Path, description = "The key"),
        ("start" = i64, Query, description = "Start index"),
        ("end" = i64, Query, description = "End index")
    ),
    responses(
        (status = 200, description = "Range retrieved", body = GetRangeResponse)
    ),
    tag = "Strings"
)]
async fn get_range(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<GetRangeParams>,
) -> Result<Json<ApiResponse<GetRangeResponse>>, CacheError> {
    let service = StringService::new(state.pool.clone());

    let result = service.get_range(&key, params.start, params.end).await?;

    Ok(Json(ApiResponse::new(GetRangeResponse {
        key: result.key,
        value: result.value,
        start: result.start,
        end: result.end,
    })))
}

/// PATCH /api/v1/strings/:key/range
///
/// Overwrite part of a string at a specific offset.
#[utoipa::path(
    patch,
    path = "/api/v1/strings/{key}/range",
    params(
        ("key" = String, Path, description = "The key")
    ),
    request_body = SetRangeRequest,
    responses(
        (status = 200, description = "Range set", body = SetRangeResponse),
        (status = 400, description = "Invalid offset")
    ),
    tag = "Strings"
)]
async fn set_range(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SetRangeRequest>,
) -> Result<Json<ApiResponse<SetRangeResponse>>, CacheError> {
    let service = StringService::new(state.pool.clone());

    let result = service.set_range(&key, request.offset, &request.value).await?;

    Ok(Json(ApiResponse::new(SetRangeResponse {
        key: result.key,
        new_length: result.new_length,
    })))
}

/// GET /api/v1/strings/:key/getex
///
/// Get a value and optionally update its expiration.
#[utoipa::path(
    get,
    path = "/api/v1/strings/{key}/getex",
    params(
        ("key" = String, Path, description = "The key"),
        ("ttl_seconds" = Option<u64>, Query, description = "New TTL in seconds"),
        ("ttl_ms" = Option<u64>, Query, description = "New TTL in milliseconds"),
        ("persist" = Option<bool>, Query, description = "Remove TTL")
    ),
    responses(
        (status = 200, description = "Value retrieved"),
        (status = 404, description = "Key not found")
    ),
    tag = "Strings"
)]
async fn get_ex_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<GetExParams>,
) -> Result<Json<ApiResponse<Option<String>>>, CacheError> {
    let service = StringService::new(state.pool.clone());

    let value = service
        .get_ex(&key, params.ttl_seconds, params.ttl_ms, params.persist)
        .await?;

    Ok(Json(ApiResponse::new(value)))
}
