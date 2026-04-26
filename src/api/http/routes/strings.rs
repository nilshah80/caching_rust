//! String Routes
//!
//! HTTP endpoints for Redis string operations.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post, put},
};

use crate::api::http::schemas::strings::{
    AppendRequest, AppendResponse, DelExRequest, DelExResponse, DigestResponse, GetDelResponse,
    GetExParams, GetRangeParams, GetRangeResponse, IncrementRequest, IncrementResponse, LcsRequest,
    LcsResponse, MGetRequest, MGetResponse, MSetExRequest, MSetExResponse, MSetRequest,
    MSetResponse, SetRangeRequest, SetRangeResponse, SetStringRequest, SetStringResponse,
    StrLenResponse,
};
use crate::domain::entities::ExpiryMode;
use crate::domain::repositories::{DelExCondition, LcsOptions, MSetExExistence, MSetExOptions};
use validator::Validate;

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
        // MSETEX (Redis 8.4+) — atomic multi-key SET with shared TTL
        .route("/api/v1/strings/msetex", post(msetex_strings))
        // DELEX / DIGEST (Redis 8.4+) — conditional delete + value digest
        .route("/api/v1/strings/{key}/delex", post(delex_string))
        .route("/api/v1/strings/{key}/digest", get(digest_string))
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
        // LCS
        .route("/api/v1/strings/lcs", post(lcs))
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
    state.string_service.get(&key).await?.map_or_else(
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
        (status = 400, description = "Invalid request - value size exceeds limit")
    ),
    tag = "Strings"
)]
async fn set_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SetStringRequest>,
) -> Result<Json<ApiResponse<SetStringResponse>>, CacheError> {
    // Validate value size to prevent OOM
    let max_value_size = state.config.server.max_value_size_bytes;
    if request.value.len() > max_value_size {
        return Err(CacheError::InvalidInput(format!(
            "Value size ({} bytes) exceeds maximum allowed size of {} bytes",
            request.value.len(),
            max_value_size
        )));
    }

    let result = state
        .string_service
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
    let value = state.string_service.get_del(&key).await?;

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
        (status = 400, description = "Invalid request - batch size exceeds limit")
    ),
    tag = "Strings"
)]
async fn mget_strings(
    State(state): State<AppState>,
    Json(request): Json<MGetRequest>,
) -> Result<Json<ApiResponse<MGetResponse>>, CacheError> {
    let max_batch_size = state.config.server.max_batch_size;
    let total_requested = request.keys.len();

    // Validate batch size to prevent OOM
    if total_requested > max_batch_size {
        return Err(CacheError::InvalidInput(format!(
            "Batch size {} exceeds maximum allowed size of {}",
            total_requested, max_batch_size
        )));
    }

    let result = state.string_service.mget(request.keys).await?;

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
        (status = 400, description = "Invalid request - batch size or value size exceeds limit")
    ),
    tag = "Strings"
)]
async fn mset_strings(
    State(state): State<AppState>,
    Json(request): Json<MSetRequest>,
) -> Result<Json<ApiResponse<MSetResponse>>, CacheError> {
    let max_batch_size = state.config.server.max_batch_size;
    let max_value_size = state.config.server.max_value_size_bytes;
    let batch_size = request.pairs.len();

    // Validate batch size to prevent OOM
    if batch_size > max_batch_size {
        return Err(CacheError::InvalidInput(format!(
            "Batch size {} exceeds maximum allowed size of {}",
            batch_size, max_batch_size
        )));
    }

    // Validate individual value sizes
    for (key, value) in &request.pairs {
        if value.len() > max_value_size {
            return Err(CacheError::InvalidInput(format!(
                "Value for key '{}' ({} bytes) exceeds maximum allowed size of {} bytes",
                key,
                value.len(),
                max_value_size
            )));
        }
    }

    let pairs: Vec<(String, String)> = request.pairs.into_iter().collect();
    let keys: Vec<String> = pairs.iter().map(|(k, _)| k.clone()).collect();

    if request.nx {
        let success = state.string_service.mset_nx(pairs).await?;
        Ok(Json(ApiResponse::new(MSetResponse {
            count: if success { keys.len() } else { 0 },
            keys: if success { keys } else { vec![] },
            success,
        })))
    } else {
        let count = state.string_service.mset(pairs).await?;
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
    let new_value = if request.float {
        #[allow(clippy::cast_precision_loss)] // Integer to float conversion is intentional
        let delta = request.float_delta.unwrap_or(request.delta as f64);
        let result = state.string_service.incr_by_float(&key, delta).await?;
        result.to_string()
    } else {
        let result = state.string_service.incr_by(&key, request.delta).await?;
        result.to_string()
    };

    Ok(Json(ApiResponse::new(IncrementResponse { key, new_value })))
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
    let new_value = state.string_service.decr_by(&key, request.delta).await?;

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
        (status = 200, description = "Value appended", body = AppendResponse),
        (status = 400, description = "Invalid request - resulting value size exceeds limit")
    ),
    tag = "Strings"
)]
async fn append_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<AppendRequest>,
) -> Result<Json<ApiResponse<AppendResponse>>, CacheError> {
    let max_value_size = state.config.server.max_value_size_bytes;

    // Check current string length and validate resulting size won't exceed limit.
    // Note: This check is non-atomic; concurrent writes could exceed the limit.
    // Accepted risk: the race window is minimal and overage is bounded by max_body_size.
    // Strict enforcement would require Lua scripts, adding latency and complexity.
    let current_len = state.string_service.str_len(&key).await?;
    let new_len = current_len as usize + request.value.len();
    if new_len > max_value_size {
        return Err(CacheError::InvalidInput(format!(
            "Resulting value size ({} bytes) would exceed maximum allowed size of {} bytes",
            new_len, max_value_size
        )));
    }

    let result = state.string_service.append(&key, &request.value).await?;

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
    let length = state.string_service.str_len(&key).await?;

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
    let result = state
        .string_service
        .get_range(&key, params.start, params.end)
        .await?;

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
        (status = 400, description = "Invalid offset or resulting value size exceeds limit")
    ),
    tag = "Strings"
)]
async fn set_range(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SetRangeRequest>,
) -> Result<Json<ApiResponse<SetRangeResponse>>, CacheError> {
    // Validate offset is non-negative
    if request.offset < 0 {
        return Err(CacheError::InvalidInput(
            "Offset must be non-negative".to_string(),
        ));
    }

    let max_value_size = state.config.server.max_value_size_bytes;

    // SETRANGE can expand the string: new_len = max(current_len, offset + value.len()).
    // Note: This check is non-atomic; concurrent writes could exceed the limit.
    // Accepted risk: the race window is minimal and overage is bounded by max_body_size.
    // Strict enforcement would require Lua scripts, adding latency and complexity.
    let current_len = state.string_service.str_len(&key).await?;
    let end_position = request.offset as usize + request.value.len();
    let new_len = std::cmp::max(current_len as usize, end_position);
    if new_len > max_value_size {
        return Err(CacheError::InvalidInput(format!(
            "Resulting value size ({} bytes) would exceed maximum allowed size of {} bytes",
            new_len, max_value_size
        )));
    }

    let result = state
        .string_service
        .set_range(&key, request.offset, &request.value)
        .await?;

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
    let value = state
        .string_service
        .get_ex(&key, params.ttl_seconds, params.ttl_ms, params.persist)
        .await?;

    Ok(Json(ApiResponse::new(value)))
}

/// POST /api/v1/strings/msetex
///
/// Atomically set multiple string keys with an optional shared TTL (Redis 8.4+).
///
/// Returns `success: false` (HTTP 200) when an `nx` or `xx` precondition
/// caused Redis to skip the entire batch. At most one of the expiry options
/// (`ttl_seconds`, `ttl_ms`, `expire_at_seconds`, `expire_at_ms`, `keep_ttl`)
/// may be supplied; omitting all of them sets the keys without expiration.
#[utoipa::path(
    post,
    path = "/api/v1/strings/msetex",
    request_body = MSetExRequest,
    responses(
        (status = 200, description = "Batch evaluated; check success/count", body = MSetExResponse),
        (status = 400, description = "Invalid request — conflicting flags or batch limits exceeded"),
        (status = 501, description = "MSETEX requires Redis 8.4+")
    ),
    tag = "Strings"
)]
async fn msetex_strings(
    State(state): State<AppState>,
    Json(request): Json<MSetExRequest>,
) -> Result<Json<ApiResponse<MSetExResponse>>, CacheError> {
    if !state.capabilities.features.string_8_4_commands {
        return Err(CacheError::ModuleNotAvailable(
            "MSETEX requires Redis 8.4+".to_string(),
        ));
    }
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    if request.nx && request.xx {
        return Err(CacheError::InvalidInput(
            "nx and xx are mutually exclusive".to_string(),
        ));
    }

    let max_batch_size = state.config.server.max_batch_size;
    let max_value_size = state.config.server.max_value_size_bytes;
    let batch_size = request.pairs.len();
    if batch_size > max_batch_size {
        return Err(CacheError::InvalidInput(format!(
            "Batch size {} exceeds maximum allowed size of {}",
            batch_size, max_batch_size
        )));
    }
    for (key, value) in &request.pairs {
        if value.len() > max_value_size {
            return Err(CacheError::InvalidInput(format!(
                "Value for key '{}' ({} bytes) exceeds maximum allowed size of {} bytes",
                key,
                value.len(),
                max_value_size
            )));
        }
    }

    // At most one expiry source is permitted; KEEPTTL excludes the others.
    let expiry_sources = [
        request.ttl_seconds.is_some(),
        request.ttl_ms.is_some(),
        request.expire_at_seconds.is_some(),
        request.expire_at_ms.is_some(),
    ];
    let expiry_count = expiry_sources.iter().filter(|set| **set).count();
    if expiry_count > 1 {
        return Err(CacheError::InvalidInput(
            "Only one of ttl_seconds, ttl_ms, expire_at_seconds, expire_at_ms may be set"
                .to_string(),
        ));
    }
    if request.keep_ttl && expiry_count > 0 {
        return Err(CacheError::InvalidInput(
            "keep_ttl cannot be combined with an explicit expiry".to_string(),
        ));
    }

    let (expiry_mode, expiry_value) = if let Some(v) = request.ttl_ms {
        (Some(ExpiryMode::Px), Some(v))
    } else if let Some(v) = request.ttl_seconds {
        (Some(ExpiryMode::Ex), Some(v))
    } else if let Some(v) = request.expire_at_ms {
        (Some(ExpiryMode::PxAt), Some(v))
    } else if let Some(v) = request.expire_at_seconds {
        (Some(ExpiryMode::ExAt), Some(v))
    } else {
        (None, None)
    };

    let existence = if request.nx {
        Some(MSetExExistence::Nx)
    } else if request.xx {
        Some(MSetExExistence::Xx)
    } else {
        None
    };

    let options = MSetExOptions {
        existence,
        expiry_mode,
        expiry_value,
        keep_ttl: request.keep_ttl,
    };

    let pairs: Vec<(String, String)> = request.pairs.into_iter().collect();
    let keys: Vec<String> = pairs.iter().map(|(k, _)| k.clone()).collect();

    let success = state.string_service.msetex(pairs, options).await?;

    Ok(Json(ApiResponse::new(MSetExResponse {
        success,
        count: if success { keys.len() } else { 0 },
        keys: if success { keys } else { vec![] },
    })))
}

/// POST /api/v1/strings/:key/delex
///
/// Conditionally delete a key based on its value or XXH3 digest (Redis 8.4+).
/// At most one of `if_eq`, `if_ne`, `if_deq`, `if_dne` may be supplied. Returns
/// `deleted: false` (HTTP 200) when the condition fails or the key is absent.
#[utoipa::path(
    post,
    path = "/api/v1/strings/{key}/delex",
    params(
        ("key" = String, Path, description = "The key to conditionally delete")
    ),
    request_body = DelExRequest,
    responses(
        (status = 200, description = "Condition evaluated", body = DelExResponse),
        (status = 400, description = "Invalid request — multiple conditions supplied"),
        (status = 501, description = "DELEX requires Redis 8.4+")
    ),
    tag = "Strings"
)]
async fn delex_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<DelExRequest>,
) -> Result<Json<ApiResponse<DelExResponse>>, CacheError> {
    if !state.capabilities.features.string_8_4_commands {
        return Err(CacheError::ModuleNotAvailable(
            "DELEX requires Redis 8.4+".to_string(),
        ));
    }

    let mut condition: Option<DelExCondition> = None;
    let mut count = 0;
    if let Some(v) = request.if_eq {
        condition = Some(DelExCondition::IfEq(v));
        count += 1;
    }
    if let Some(v) = request.if_ne {
        condition = Some(DelExCondition::IfNe(v));
        count += 1;
    }
    if let Some(v) = request.if_deq {
        condition = Some(DelExCondition::IfDeq(v));
        count += 1;
    }
    if let Some(v) = request.if_dne {
        condition = Some(DelExCondition::IfDne(v));
        count += 1;
    }
    if count > 1 {
        return Err(CacheError::InvalidInput(
            "At most one of if_eq, if_ne, if_deq, if_dne may be supplied".to_string(),
        ));
    }

    let deleted = state.string_service.delex(&key, condition).await?;

    Ok(Json(ApiResponse::new(DelExResponse { key, deleted })))
}

/// GET /api/v1/strings/:key/digest
///
/// Compute the XXH3 hash digest of a string value (Redis 8.4+). Returns HTTP 200
/// with `exists: false` and the `digest` field omitted from the JSON body when
/// the key is absent.
#[utoipa::path(
    get,
    path = "/api/v1/strings/{key}/digest",
    params(
        ("key" = String, Path, description = "The key to digest")
    ),
    responses(
        (status = 200, description = "Digest result", body = DigestResponse),
        (status = 501, description = "DIGEST requires Redis 8.4+")
    ),
    tag = "Strings"
)]
async fn digest_string(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<DigestResponse>>, CacheError> {
    if !state.capabilities.features.string_8_4_commands {
        return Err(CacheError::ModuleNotAvailable(
            "DIGEST requires Redis 8.4+".to_string(),
        ));
    }

    let digest = state.string_service.digest(&key).await?;

    Ok(Json(ApiResponse::new(DigestResponse {
        key,
        exists: digest.is_some(),
        digest,
    })))
}

/// POST /api/v1/strings/lcs
///
/// Compute the Longest Common Subsequence of two string keys (Redis 7.0+).
#[utoipa::path(
    post,
    path = "/api/v1/strings/lcs",
    request_body = LcsRequest,
    responses(
        (status = 200, description = "LCS computed", body = LcsResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Strings"
)]
pub async fn lcs(
    State(state): State<AppState>,
    Json(request): Json<LcsRequest>,
) -> Result<Json<ApiResponse<LcsResponse>>, CacheError> {
    if !state.capabilities.features.lcs {
        return Err(CacheError::ModuleNotAvailable(
            "LCS command requires Redis 7.0+".to_string(),
        ));
    }
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let options = LcsOptions {
        len: request.len,
        idx: request.idx,
        min_match_len: request.min_match_len,
        with_match_len: request.with_match_len,
    };

    let result = state
        .string_service
        .lcs(&request.key1, &request.key2, options)
        .await?;

    Ok(Json(ApiResponse::success(LcsResponse::from(result))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::config::Settings;
    use crate::test_support::{test_state, test_state_with_config};
    use axum::Json;
    use axum::extract::{Path, Query, State};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_string_handlers() {
        let (state, string_repo, _, _) = test_state();
        let state = State(state);

        // Missing key -> KeyNotFound
        let missing = get_string(state.clone(), Path("missing".to_string())).await;
        assert!(matches!(missing, Err(CacheError::KeyNotFound(_))));

        string_repo.insert("key", "value");

        let found = get_string(state.clone(), Path("key".to_string()))
            .await
            .unwrap();
        assert_eq!(found.0.data.as_ref().unwrap().value, "value");

        let set_req = SetStringRequest {
            value: "new".to_string(),
            ttl_seconds: None,
            ttl_ms: None,
            nx: false,
            xx: false,
            get: false,
            keep_ttl: false,
        };
        let set_resp = set_string(state.clone(), Path("key".to_string()), Json(set_req))
            .await
            .unwrap();
        assert!(set_resp.0.data.unwrap().success);

        let del_resp = get_del_string(state.clone(), Path("key".to_string()))
            .await
            .unwrap();
        assert!(del_resp.0.data.unwrap().existed);

        string_repo.insert("k1", "v1");
        let mget_req = MGetRequest {
            keys: vec!["k1".to_string(), "k2".to_string()],
        };
        let mget_resp = mget_strings(state.clone(), Json(mget_req)).await.unwrap();
        assert_eq!(mget_resp.0.data.unwrap().found_count, 1);

        let mut pairs = HashMap::new();
        pairs.insert("a".to_string(), "1".to_string());
        let mset_req = MSetRequest { pairs, nx: false };
        let mset_resp = mset_strings(state.clone(), Json(mset_req)).await.unwrap();
        assert!(mset_resp.0.data.unwrap().success);

        let mut nx_pairs = HashMap::new();
        nx_pairs.insert("a".to_string(), "2".to_string());
        let mset_nx_req = MSetRequest {
            pairs: nx_pairs,
            nx: true,
        };
        let mset_nx_resp = mset_strings(state.clone(), Json(mset_nx_req))
            .await
            .unwrap();
        assert!(!mset_nx_resp.0.data.unwrap().success);

        let incr_req = IncrementRequest {
            delta: 2,
            float: false,
            float_delta: None,
        };
        let incr_resp = incr_string(state.clone(), Path("counter".to_string()), Json(incr_req))
            .await
            .unwrap();
        assert_eq!(incr_resp.0.data.unwrap().new_value, "2");

        let incr_float = IncrementRequest {
            delta: 1,
            float: true,
            float_delta: Some(1.5),
        };
        let incr_float_resp = incr_string(state.clone(), Path("f".to_string()), Json(incr_float))
            .await
            .unwrap();
        assert_eq!(incr_float_resp.0.data.unwrap().new_value, "1.5");

        let decr_req = IncrementRequest {
            delta: 1,
            float: false,
            float_delta: None,
        };
        let decr_resp = decr_string(state.clone(), Path("counter".to_string()), Json(decr_req))
            .await
            .unwrap();
        assert_eq!(decr_resp.0.data.unwrap().new_value, "1");

        let append_req = AppendRequest {
            value: "x".to_string(),
        };
        let append_resp = append_string(state.clone(), Path("k1".to_string()), Json(append_req))
            .await
            .unwrap();
        assert!(append_resp.0.data.unwrap().new_length > 0);

        let len_resp = strlen_string(state.clone(), Path("k1".to_string()))
            .await
            .unwrap();
        assert!(len_resp.0.data.unwrap().length > 0);

        let range_params = GetRangeParams { start: 0, end: 1 };
        let range_resp = get_range(state.clone(), Path("k1".to_string()), Query(range_params))
            .await
            .unwrap();
        assert_eq!(range_resp.0.data.unwrap().start, 0);

        let set_range_req = SetRangeRequest {
            offset: 0,
            value: "zz".to_string(),
        };
        let set_range_resp = set_range(state.clone(), Path("k1".to_string()), Json(set_range_req))
            .await
            .unwrap();
        assert!(set_range_resp.0.data.unwrap().new_length >= 2);

        let get_ex_params = GetExParams {
            ttl_seconds: None,
            ttl_ms: None,
            persist: false,
        };
        let get_ex_resp = get_ex_string(state, Path("k1".to_string()), Query(get_ex_params))
            .await
            .unwrap();
        assert!(get_ex_resp.0.data.is_some());
    }

    #[tokio::test]
    async fn test_set_value_size_limit() {
        // Create config with small max_value_size for testing
        let mut config = Settings::default();
        config.server.max_value_size_bytes = 10; // Only 10 bytes allowed
        let (state, _, _, _) = test_state_with_config(config);
        let state = State(state);

        // Value within limit should succeed
        let small_req = SetStringRequest {
            value: "small".to_string(), // 5 bytes
            ttl_seconds: None,
            ttl_ms: None,
            nx: false,
            xx: false,
            get: false,
            keep_ttl: false,
        };
        let result = set_string(state.clone(), Path("key".to_string()), Json(small_req)).await;
        assert!(result.is_ok());

        // Value exceeding limit should fail with InvalidInput
        let large_req = SetStringRequest {
            value: "this is way too large".to_string(), // > 10 bytes
            ttl_seconds: None,
            ttl_ms: None,
            nx: false,
            xx: false,
            get: false,
            keep_ttl: false,
        };
        let result = set_string(state.clone(), Path("key2".to_string()), Json(large_req)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_mset_value_size_limit() {
        let mut config = Settings::default();
        config.server.max_value_size_bytes = 10;
        let (state, _, _, _) = test_state_with_config(config);
        let state = State(state);

        // One value exceeds limit
        let mut pairs = HashMap::new();
        pairs.insert("a".to_string(), "small".to_string()); // OK
        pairs.insert("b".to_string(), "this is too large for limit".to_string()); // Exceeds
        let req = MSetRequest { pairs, nx: false };
        let result = mset_strings(state.clone(), Json(req)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_mset_batch_size_limit() {
        let mut config = Settings::default();
        config.server.max_batch_size = 1;
        let (state, _, _, _) = test_state_with_config(config);
        let state = State(state);

        let mut pairs = HashMap::new();
        pairs.insert("a".to_string(), "1".to_string());
        pairs.insert("b".to_string(), "2".to_string());
        let req = MSetRequest { pairs, nx: false };
        let result = mset_strings(state.clone(), Json(req)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_mget_batch_size_limit() {
        let mut config = Settings::default();
        config.server.max_batch_size = 2; // Only 2 keys allowed
        let (state, _, _, _) = test_state_with_config(config);
        let state = State(state);

        // Batch within limit should succeed
        let small_req = MGetRequest {
            keys: vec!["a".to_string(), "b".to_string()],
        };
        let result = mget_strings(state.clone(), Json(small_req)).await;
        assert!(result.is_ok());

        // Batch exceeding limit should fail
        let large_req = MGetRequest {
            keys: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };
        let result = mget_strings(state.clone(), Json(large_req)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_append_size_limit() {
        let mut config = Settings::default();
        config.server.max_value_size_bytes = 10;
        let (state, string_repo, _, _) = test_state_with_config(config);
        let state = State(state);

        // Set initial value (5 bytes)
        string_repo.insert("key", "hello");

        // Append within limit should succeed (5 + 3 = 8 < 10)
        let small_append = AppendRequest {
            value: "abc".to_string(),
        };
        let result =
            append_string(state.clone(), Path("key".to_string()), Json(small_append)).await;
        assert!(result.is_ok());

        // Append that would exceed limit should fail (8 + 5 = 13 > 10)
        let large_append = AppendRequest {
            value: "world".to_string(),
        };
        let result =
            append_string(state.clone(), Path("key".to_string()), Json(large_append)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_set_range_size_limit() {
        let mut config = Settings::default();
        config.server.max_value_size_bytes = 10;
        let (state, string_repo, _, _) = test_state_with_config(config);
        let state = State(state);

        // Set initial value (5 bytes)
        string_repo.insert("key", "hello");

        // SETRANGE within current length should succeed
        let in_range = SetRangeRequest {
            offset: 0,
            value: "hi".to_string(),
        };
        let result = set_range(state.clone(), Path("key".to_string()), Json(in_range)).await;
        assert!(result.is_ok());

        // SETRANGE that would expand beyond limit should fail
        // offset 8 + value "xyz" (3 bytes) = 11 bytes > 10 limit
        let expand = SetRangeRequest {
            offset: 8,
            value: "xyz".to_string(),
        };
        let result = set_range(state.clone(), Path("key".to_string()), Json(expand)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_set_range_negative_offset() {
        let (state, string_repo, _, _) = test_state();
        let state = State(state);

        string_repo.insert("key", "hello");

        // Negative offset should fail with InvalidInput
        let neg_offset = SetRangeRequest {
            offset: -1,
            value: "x".to_string(),
        };
        let result = set_range(state.clone(), Path("key".to_string()), Json(neg_offset)).await;
        assert!(
            matches!(result, Err(CacheError::InvalidInput(ref msg)) if msg.contains("non-negative"))
        );
    }

    #[tokio::test]
    async fn test_lcs_string_result() {
        let (state, string_repo, _, _) = test_state();
        let state = State(state);

        string_repo.insert("k1", "ohmytext");
        string_repo.insert("k2", "mynewtext");

        let req = LcsRequest {
            key1: "k1".to_string(),
            key2: "k2".to_string(),
            len: false,
            idx: false,
            min_match_len: None,
            with_match_len: false,
        };
        let resp = lcs(state, Json(req)).await.unwrap();
        let data = resp.0.data.unwrap();
        match data {
            LcsResponse::String { lcs: s } => assert_eq!(s, "mytext"),
            _ => panic!("Expected String variant"),
        }
    }

    #[tokio::test]
    async fn test_lcs_length_result() {
        let (state, string_repo, _, _) = test_state();
        let state = State(state);

        string_repo.insert("k1", "ohmytext");
        string_repo.insert("k2", "mynewtext");

        let req = LcsRequest {
            key1: "k1".to_string(),
            key2: "k2".to_string(),
            len: true,
            idx: false,
            min_match_len: None,
            with_match_len: false,
        };
        let resp = lcs(state, Json(req)).await.unwrap();
        let data = resp.0.data.unwrap();
        match data {
            LcsResponse::Length { length } => assert_eq!(length, 6),
            _ => panic!("Expected Length variant"),
        }
    }

    #[tokio::test]
    async fn test_lcs_matches_result() {
        let (state, string_repo, _, _) = test_state();
        let state = State(state);

        string_repo.insert("k1", "ohmytext");
        string_repo.insert("k2", "mynewtext");

        let req = LcsRequest {
            key1: "k1".to_string(),
            key2: "k2".to_string(),
            len: false,
            idx: true,
            min_match_len: None,
            with_match_len: true,
        };
        let resp = lcs(state, Json(req)).await.unwrap();
        let data = resp.0.data.unwrap();
        match data {
            LcsResponse::Matches { matches, len } => {
                assert_eq!(len, 6);
                assert!(!matches.is_empty());
                // Verify match_len is present since with_match_len was true
                assert!(matches.iter().all(|m| m.match_len.is_some()));
            }
            _ => panic!("Expected Matches variant"),
        }
    }

    #[tokio::test]
    async fn test_lcs_validation_rejects_empty_keys() {
        let (state, _, _, _) = test_state();
        let state = State(state);

        let req = LcsRequest {
            key1: "".to_string(),
            key2: "b".to_string(),
            len: false,
            idx: false,
            min_match_len: None,
            with_match_len: false,
        };
        let result = lcs(state.clone(), Json(req)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));

        let req2 = LcsRequest {
            key1: "a".to_string(),
            key2: "".to_string(),
            len: false,
            idx: false,
            min_match_len: None,
            with_match_len: false,
        };
        let result2 = lcs(state, Json(req2)).await;
        assert!(matches!(result2, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_lcs_returns_501_when_feature_disabled() {
        let (mut app_state, _, _, _) = test_state();
        // Create capabilities with lcs disabled
        let mut caps = (*app_state.capabilities).clone();
        caps.features.lcs = false;
        app_state.capabilities = std::sync::Arc::new(caps);
        let state = State(app_state);

        let req = LcsRequest {
            key1: "k1".into(),
            key2: "k2".into(),
            len: false,
            idx: false,
            min_match_len: None,
            with_match_len: false,
        };
        let result = lcs(state, Json(req)).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    fn enable_string_8_4(app_state: &mut crate::shared::app_state::AppState) {
        let mut caps = (*app_state.capabilities).clone();
        caps.features.string_8_4_commands = true;
        app_state.capabilities = std::sync::Arc::new(caps);
    }

    #[tokio::test]
    async fn test_msetex_handler_success_path() {
        let (mut app_state, string_repo, _, _) = test_state();
        enable_string_8_4(&mut app_state);
        let state = State(app_state);

        let mut pairs = HashMap::new();
        pairs.insert("k1".to_string(), "v1".to_string());
        pairs.insert("k2".to_string(), "v2".to_string());
        let req = MSetExRequest {
            pairs,
            nx: false,
            xx: false,
            ttl_seconds: Some(60),
            ttl_ms: None,
            expire_at_seconds: None,
            expire_at_ms: None,
            keep_ttl: false,
        };

        let resp = msetex_strings(state, Json(req)).await.unwrap();
        let body = resp.0.data.expect("body");
        assert!(body.success);
        assert_eq!(body.count, 2);
        use crate::domain::repositories::StringRepository;
        let stored = StringRepository::get(string_repo.as_ref(), "k1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.value, "v1");
    }

    #[tokio::test]
    async fn test_msetex_returns_501_when_capability_missing() {
        // Default test_state() leaves string_8_4_commands disabled.
        let (state, _, _, _) = test_state();
        let state = State(state);
        let mut pairs = HashMap::new();
        pairs.insert("k1".to_string(), "v1".to_string());
        let req = MSetExRequest {
            pairs,
            nx: false,
            xx: false,
            ttl_seconds: Some(10),
            ttl_ms: None,
            expire_at_seconds: None,
            expire_at_ms: None,
            keep_ttl: false,
        };
        let result = msetex_strings(state, Json(req)).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_msetex_rejects_nx_and_xx_together() {
        let (mut app_state, _, _, _) = test_state();
        enable_string_8_4(&mut app_state);
        let state = State(app_state);

        let mut pairs = HashMap::new();
        pairs.insert("k".to_string(), "v".to_string());
        let req = MSetExRequest {
            pairs,
            nx: true,
            xx: true,
            ttl_seconds: Some(10),
            ttl_ms: None,
            expire_at_seconds: None,
            expire_at_ms: None,
            keep_ttl: false,
        };
        let result = msetex_strings(state, Json(req)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_msetex_rejects_multiple_expiry_sources() {
        let (mut app_state, _, _, _) = test_state();
        enable_string_8_4(&mut app_state);
        let state = State(app_state);

        let mut pairs = HashMap::new();
        pairs.insert("k".to_string(), "v".to_string());
        let req = MSetExRequest {
            pairs,
            nx: false,
            xx: false,
            ttl_seconds: Some(10),
            ttl_ms: Some(5000),
            expire_at_seconds: None,
            expire_at_ms: None,
            keep_ttl: false,
        };
        let result = msetex_strings(state, Json(req)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_msetex_rejects_zero_ttl_seconds() {
        let (mut app_state, _, _, _) = test_state();
        enable_string_8_4(&mut app_state);
        let state = State(app_state);

        let mut pairs = HashMap::new();
        pairs.insert("k".to_string(), "v".to_string());
        let req = MSetExRequest {
            pairs,
            nx: false,
            xx: false,
            ttl_seconds: Some(0),
            ttl_ms: None,
            expire_at_seconds: None,
            expire_at_ms: None,
            keep_ttl: false,
        };
        let result = msetex_strings(state, Json(req)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_msetex_allows_nx_with_keep_ttl_through_handler() {
        // Redis allows NX KEEPTTL together; ensure the handler does not 400.
        let (mut app_state, _, _, _) = test_state();
        enable_string_8_4(&mut app_state);
        let state = State(app_state);

        let mut pairs = HashMap::new();
        pairs.insert("k".to_string(), "v".to_string());
        let req = MSetExRequest {
            pairs,
            nx: true,
            xx: false,
            ttl_seconds: None,
            ttl_ms: None,
            expire_at_seconds: None,
            expire_at_ms: None,
            keep_ttl: true,
        };
        let resp = msetex_strings(state, Json(req)).await.unwrap();
        // Mock writes through and reports success.
        assert!(resp.0.data.unwrap().success);
    }

    #[tokio::test]
    async fn test_msetex_rejects_keep_ttl_with_explicit_expiry() {
        let (mut app_state, _, _, _) = test_state();
        enable_string_8_4(&mut app_state);
        let state = State(app_state);

        let mut pairs = HashMap::new();
        pairs.insert("k".to_string(), "v".to_string());
        let req = MSetExRequest {
            pairs,
            nx: false,
            xx: false,
            ttl_seconds: Some(10),
            ttl_ms: None,
            expire_at_seconds: None,
            expire_at_ms: None,
            keep_ttl: true,
        };
        let result = msetex_strings(state, Json(req)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_delex_handler_unconditional_delete() {
        let (mut app_state, string_repo, _, _) = test_state();
        enable_string_8_4(&mut app_state);
        string_repo.insert("victim", "v");
        let state = State(app_state);

        let resp = delex_string(
            state.clone(),
            Path("victim".to_string()),
            Json(DelExRequest::default()),
        )
        .await
        .unwrap();
        assert!(resp.0.data.unwrap().deleted);

        let resp = delex_string(
            state,
            Path("victim".to_string()),
            Json(DelExRequest::default()),
        )
        .await
        .unwrap();
        assert!(!resp.0.data.unwrap().deleted);
    }

    #[tokio::test]
    async fn test_delex_handler_if_eq_no_match() {
        let (mut app_state, string_repo, _, _) = test_state();
        enable_string_8_4(&mut app_state);
        string_repo.insert("k", "current");
        let state = State(app_state);

        let req = DelExRequest {
            if_eq: Some("other".to_string()),
            ..Default::default()
        };
        let resp = delex_string(state, Path("k".to_string()), Json(req))
            .await
            .unwrap();
        assert!(!resp.0.data.unwrap().deleted);
    }

    #[tokio::test]
    async fn test_delex_handler_rejects_multiple_conditions() {
        let (mut app_state, _, _, _) = test_state();
        enable_string_8_4(&mut app_state);
        let state = State(app_state);

        let req = DelExRequest {
            if_eq: Some("a".to_string()),
            if_ne: Some("b".to_string()),
            ..Default::default()
        };
        let result = delex_string(state, Path("k".to_string()), Json(req)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_delex_returns_501_when_capability_missing() {
        let (state, _, _, _) = test_state();
        let state = State(state);
        let result =
            delex_string(state, Path("k".to_string()), Json(DelExRequest::default())).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_digest_handler_returns_value_and_missing() {
        let (mut app_state, string_repo, _, _) = test_state();
        enable_string_8_4(&mut app_state);
        string_repo.insert("present", "payload");
        let state = State(app_state);

        let resp = digest_string(state.clone(), Path("present".to_string()))
            .await
            .unwrap();
        let body = resp.0.data.expect("body");
        assert!(body.exists);
        assert!(body.digest.is_some());

        let resp = digest_string(state, Path("absent".to_string()))
            .await
            .unwrap();
        let body = resp.0.data.expect("body");
        assert!(!body.exists);
        assert!(body.digest.is_none());
    }

    #[tokio::test]
    async fn test_digest_returns_501_when_capability_missing() {
        let (state, _, _, _) = test_state();
        let state = State(state);
        let result = digest_string(state, Path("k".to_string())).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }
}
