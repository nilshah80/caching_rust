//! JSON Routes
//!
//! HTTP endpoints for RedisJSON operations.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use validator::Validate;

use crate::api::http::schemas::json::{
    JsonArrAppendRequest, JsonArrAppendResponse, JsonArrIndexRequest, JsonArrIndexResponse,
    JsonArrInsertRequest, JsonArrInsertResponse, JsonArrLenParams, JsonArrLenResponse,
    JsonArrPopRequest, JsonArrPopResponse, JsonArrTrimRequest, JsonArrTrimResponse,
    JsonClearParams, JsonClearResponse, JsonDebugMemoryParams, JsonDebugMemoryResponse,
    JsonDelParams, JsonDelResponse, JsonGetParams, JsonGetResponse, JsonMGetRequest,
    JsonMGetItem, JsonMGetResponse, JsonMSetRequest, JsonNumIncrByRequest, JsonNumMultByRequest,
    JsonNumResponse, JsonObjKeysParams, JsonObjKeysResponse, JsonObjLenParams, JsonObjLenResponse,
    JsonRespParams, JsonRespResponse, JsonSetRequest, JsonSetResponse, JsonStrAppendRequest,
    JsonStrAppendResponse, JsonStrLenParams, JsonStrLenResponse, JsonToggleParams,
    JsonToggleResponse, JsonTypeParams, JsonTypeResponse,
};
use crate::domain::entities::JsonMSetItem;
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create JSON routes
pub fn json_routes() -> Router<AppState> {
    Router::new()
        // Core operations
        .route("/api/v1/json/{key}", put(json_set))
        .route("/api/v1/json/{key}", get(json_get))
        .route("/api/v1/json/{key}", delete(json_del))
        .route("/api/v1/json/mget", post(json_mget))
        .route("/api/v1/json/mset", post(json_mset))
        .route("/api/v1/json/{key}/type", get(json_type))
        // String operations
        .route("/api/v1/json/{key}/strlen", get(json_str_len))
        .route("/api/v1/json/{key}/strappend", patch(json_str_append))
        // Numeric operations
        .route("/api/v1/json/{key}/numincrby", patch(json_num_incr_by))
        .route("/api/v1/json/{key}/nummultby", patch(json_num_mult_by))
        .route("/api/v1/json/{key}/toggle", patch(json_toggle))
        .route("/api/v1/json/{key}/clear", post(json_clear))
        // Array operations
        .route("/api/v1/json/{key}/arrlen", get(json_arr_len))
        .route("/api/v1/json/{key}/arrappend", post(json_arr_append))
        .route("/api/v1/json/{key}/arrindex", post(json_arr_index))
        .route("/api/v1/json/{key}/arrinsert", post(json_arr_insert))
        .route("/api/v1/json/{key}/arrpop", delete(json_arr_pop))
        .route("/api/v1/json/{key}/arrtrim", post(json_arr_trim))
        // Object operations
        .route("/api/v1/json/{key}/objlen", get(json_obj_len))
        .route("/api/v1/json/{key}/objkeys", get(json_obj_keys))
        // Debug operations
        .route("/api/v1/json/{key}/debug/memory", get(json_debug_memory))
        .route("/api/v1/json/{key}/resp", get(json_resp))
}

// ==================== Core Operations ====================

/// PUT /api/v1/json/:key
///
/// Set a JSON value at a path (JSON.SET).
#[utoipa::path(
    put,
    path = "/api/v1/json/{key}",
    params(
        ("key" = String, Path, description = "The key to set")
    ),
    request_body = JsonSetRequest,
    responses(
        (status = 200, description = "JSON value set successfully", body = JsonSetResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_set(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<JsonSetRequest>,
) -> Result<Json<ApiResponse<JsonSetResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.json_service.json_set(&key, &request.path, request.value, request.nx, request.xx).await?;

    Ok(Json(ApiResponse::new(JsonSetResponse {
        key: result.key,
        path: result.path,
        success: result.success,
    })))
}

/// GET /api/v1/json/:key
///
/// Get JSON value at path (JSON.GET).
#[utoipa::path(
    get,
    path = "/api/v1/json/{key}",
    params(
        ("key" = String, Path, description = "The key to retrieve"),
        ("path" = Option<String>, Query, description = "JSONPath to retrieve (default: $)")
    ),
    responses(
        (status = 200, description = "JSON value retrieved", body = JsonGetResponse),
        (status = 404, description = "Key not found"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_get(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<JsonGetParams>,
) -> Result<Json<ApiResponse<JsonGetResponse>>, CacheError> {
    let result = state
        .json_service
        .json_get(&key, vec![params.path.clone()])
        .await?
        .ok_or_else(|| CacheError::KeyNotFound(key.clone()))?;

    Ok(Json(ApiResponse::new(JsonGetResponse {
        key: result.key,
        paths: result.paths,
        value: result.value,
    })))
}

/// DELETE /api/v1/json/:key
///
/// Delete JSON value at path (JSON.DEL).
#[utoipa::path(
    delete,
    path = "/api/v1/json/{key}",
    params(
        ("key" = String, Path, description = "The key to delete from"),
        ("path" = Option<String>, Query, description = "JSONPath to delete (default: $ for entire document)")
    ),
    responses(
        (status = 200, description = "JSON value deleted", body = JsonDelResponse),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_del(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<JsonDelParams>,
) -> Result<Json<ApiResponse<JsonDelResponse>>, CacheError> {
    let result = state.json_service.json_del(&key, &params.path).await?;

    Ok(Json(ApiResponse::new(JsonDelResponse {
        key: result.key,
        path: result.path,
        deleted_count: result.deleted_count,
    })))
}

/// POST /api/v1/json/mget
///
/// Get values from multiple keys at a path (JSON.MGET).
#[utoipa::path(
    post,
    path = "/api/v1/json/mget",
    request_body = JsonMGetRequest,
    responses(
        (status = 200, description = "JSON values retrieved", body = JsonMGetResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_mget(
    State(state): State<AppState>,
    Json(request): Json<JsonMGetRequest>,
) -> Result<Json<ApiResponse<JsonMGetResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.json_service.json_mget(request.keys.clone(), &request.path).await?;

    let results: Vec<JsonMGetItem> = result
        .results
        .into_iter()
        .map(|item| JsonMGetItem {
            key: item.key,
            value: item.value,
        })
        .collect();

    Ok(Json(ApiResponse::new(JsonMGetResponse {
        results,
        path: result.path,
    })))
}

/// POST /api/v1/json/mset
///
/// Set multiple key-path-value triplets (JSON.MSET).
#[utoipa::path(
    post,
    path = "/api/v1/json/mset",
    request_body = JsonMSetRequest,
    responses(
        (status = 200, description = "JSON values set successfully"),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_mset(
    State(state): State<AppState>,
    Json(request): Json<JsonMSetRequest>,
) -> Result<Json<ApiResponse<()>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let items: Vec<JsonMSetItem> = request
        .items
        .into_iter()
        .map(|item| JsonMSetItem {
            key: item.key,
            path: item.path,
            value: item.value,
        })
        .collect();

    let result = state.json_service.json_mset(items).await;
    result?;

    Ok(Json(ApiResponse::new(())))
}

/// GET /api/v1/json/:key/type
///
/// Get the JSON type at path (JSON.TYPE).
#[utoipa::path(
    get,
    path = "/api/v1/json/{key}/type",
    params(
        ("key" = String, Path, description = "The key"),
        ("path" = Option<String>, Query, description = "JSONPath (default: $)")
    ),
    responses(
        (status = 200, description = "JSON type retrieved", body = JsonTypeResponse),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_type(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<JsonTypeParams>,
) -> Result<Json<ApiResponse<JsonTypeResponse>>, CacheError> {
    let result = state.json_service.json_type(&key, &params.path).await?;

    Ok(Json(ApiResponse::new(JsonTypeResponse {
        key: result.key,
        path: result.path,
        types: result.types,
    })))
}

// ==================== String Operations ====================

/// GET /api/v1/json/:key/strlen
///
/// Get the length of a JSON string at path (JSON.STRLEN).
#[utoipa::path(
    get,
    path = "/api/v1/json/{key}/strlen",
    params(
        ("key" = String, Path, description = "The key"),
        ("path" = Option<String>, Query, description = "JSONPath to the string (default: $)")
    ),
    responses(
        (status = 200, description = "String length retrieved", body = JsonStrLenResponse),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_str_len(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<JsonStrLenParams>,
) -> Result<Json<ApiResponse<JsonStrLenResponse>>, CacheError> {
    let result = state.json_service.json_str_len(&key, &params.path).await?;

    Ok(Json(ApiResponse::new(JsonStrLenResponse {
        key: result.key,
        path: result.path,
        lengths: result.lengths,
    })))
}

/// PATCH /api/v1/json/:key/strappend
///
/// Append to a JSON string at path (JSON.STRAPPEND).
#[utoipa::path(
    patch,
    path = "/api/v1/json/{key}/strappend",
    params(
        ("key" = String, Path, description = "The key")
    ),
    request_body = JsonStrAppendRequest,
    responses(
        (status = 200, description = "String appended", body = JsonStrAppendResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_str_append(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<JsonStrAppendRequest>,
) -> Result<Json<ApiResponse<JsonStrAppendResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.json_service.json_str_append(&key, &request.path, &request.value).await?;

    Ok(Json(ApiResponse::new(JsonStrAppendResponse {
        key: result.key,
        path: result.path,
        new_lengths: result.new_lengths,
    })))
}

// ==================== Numeric Operations ====================

/// PATCH /api/v1/json/:key/numincrby
///
/// Increment a numeric value at path (JSON.NUMINCRBY).
#[utoipa::path(
    patch,
    path = "/api/v1/json/{key}/numincrby",
    params(
        ("key" = String, Path, description = "The key")
    ),
    request_body = JsonNumIncrByRequest,
    responses(
        (status = 200, description = "Number incremented", body = JsonNumResponse),
        (status = 400, description = "Invalid request or not a number"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_num_incr_by(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<JsonNumIncrByRequest>,
) -> Result<Json<ApiResponse<JsonNumResponse>>, CacheError> {
    let result = state
        .json_service
        .json_num_incr_by(&key, &request.path, request.value)
        .await?;

    Ok(Json(ApiResponse::new(JsonNumResponse {
        key: result.key,
        path: result.path,
        values: result.values,
    })))
}

/// PATCH /api/v1/json/:key/nummultby
///
/// Multiply a numeric value at path (JSON.NUMMULTBY).
#[utoipa::path(
    patch,
    path = "/api/v1/json/{key}/nummultby",
    params(
        ("key" = String, Path, description = "The key")
    ),
    request_body = JsonNumMultByRequest,
    responses(
        (status = 200, description = "Number multiplied", body = JsonNumResponse),
        (status = 400, description = "Invalid request or not a number"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_num_mult_by(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<JsonNumMultByRequest>,
) -> Result<Json<ApiResponse<JsonNumResponse>>, CacheError> {
    let result = state
        .json_service
        .json_num_mult_by(&key, &request.path, request.value)
        .await?;

    Ok(Json(ApiResponse::new(JsonNumResponse {
        key: result.key,
        path: result.path,
        values: result.values,
    })))
}

/// PATCH /api/v1/json/:key/toggle
///
/// Toggle a boolean value at path (JSON.TOGGLE).
#[utoipa::path(
    patch,
    path = "/api/v1/json/{key}/toggle",
    params(
        ("key" = String, Path, description = "The key"),
        ("path" = Option<String>, Query, description = "JSONPath to the boolean (default: $)")
    ),
    responses(
        (status = 200, description = "Boolean toggled", body = JsonToggleResponse),
        (status = 400, description = "Not a boolean value"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_toggle(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<JsonToggleParams>,
) -> Result<Json<ApiResponse<JsonToggleResponse>>, CacheError> {
    let result = state.json_service.json_toggle(&key, &params.path).await?;

    Ok(Json(ApiResponse::new(JsonToggleResponse {
        key: result.key,
        path: result.path,
        values: result.values,
    })))
}

/// POST /api/v1/json/:key/clear
///
/// Clear container values or set numbers to 0 (JSON.CLEAR).
#[utoipa::path(
    post,
    path = "/api/v1/json/{key}/clear",
    params(
        ("key" = String, Path, description = "The key"),
        ("path" = Option<String>, Query, description = "JSONPath to clear (default: $)")
    ),
    responses(
        (status = 200, description = "Values cleared", body = JsonClearResponse),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_clear(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<JsonClearParams>,
) -> Result<Json<ApiResponse<JsonClearResponse>>, CacheError> {
    let result = state.json_service.json_clear(&key, &params.path).await?;

    Ok(Json(ApiResponse::new(JsonClearResponse {
        key: result.key,
        path: result.path,
        cleared_count: result.cleared_count,
    })))
}

// ==================== Array Operations ====================

/// GET /api/v1/json/:key/arrlen
///
/// Get the length of a JSON array at path (JSON.ARRLEN).
#[utoipa::path(
    get,
    path = "/api/v1/json/{key}/arrlen",
    params(
        ("key" = String, Path, description = "The key"),
        ("path" = Option<String>, Query, description = "JSONPath to the array (default: $)")
    ),
    responses(
        (status = 200, description = "Array length retrieved", body = JsonArrLenResponse),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_arr_len(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<JsonArrLenParams>,
) -> Result<Json<ApiResponse<JsonArrLenResponse>>, CacheError> {
    let result = state.json_service.json_arr_len(&key, &params.path).await?;

    Ok(Json(ApiResponse::new(JsonArrLenResponse {
        key: result.key,
        path: result.path,
        lengths: result.lengths,
    })))
}

/// POST /api/v1/json/:key/arrappend
///
/// Append values to a JSON array at path (JSON.ARRAPPEND).
#[utoipa::path(
    post,
    path = "/api/v1/json/{key}/arrappend",
    params(
        ("key" = String, Path, description = "The key")
    ),
    request_body = JsonArrAppendRequest,
    responses(
        (status = 200, description = "Values appended", body = JsonArrAppendResponse),
        (status = 400, description = "Invalid request or not an array"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_arr_append(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<JsonArrAppendRequest>,
) -> Result<Json<ApiResponse<JsonArrAppendResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.json_service.json_arr_append(&key, &request.path, request.values).await?;

    Ok(Json(ApiResponse::new(JsonArrAppendResponse {
        key: result.key,
        path: result.path,
        new_lengths: result.new_lengths,
    })))
}

/// POST /api/v1/json/:key/arrindex
///
/// Find the index of an element in a JSON array (JSON.ARRINDEX).
#[utoipa::path(
    post,
    path = "/api/v1/json/{key}/arrindex",
    params(
        ("key" = String, Path, description = "The key")
    ),
    request_body = JsonArrIndexRequest,
    responses(
        (status = 200, description = "Index found", body = JsonArrIndexResponse),
        (status = 400, description = "Invalid request or not an array"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_arr_index(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<JsonArrIndexRequest>,
) -> Result<Json<ApiResponse<JsonArrIndexResponse>>, CacheError> {
    let result = state
        .json_service
        .json_arr_index(&key, &request.path, request.value, request.start, request.stop)
        .await?;

    Ok(Json(ApiResponse::new(JsonArrIndexResponse {
        key: result.key,
        path: result.path,
        indices: result.indices,
    })))
}

/// POST /api/v1/json/:key/arrinsert
///
/// Insert values at an index in a JSON array (JSON.ARRINSERT).
#[utoipa::path(
    post,
    path = "/api/v1/json/{key}/arrinsert",
    params(
        ("key" = String, Path, description = "The key")
    ),
    request_body = JsonArrInsertRequest,
    responses(
        (status = 200, description = "Values inserted", body = JsonArrInsertResponse),
        (status = 400, description = "Invalid request or not an array"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_arr_insert(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<JsonArrInsertRequest>,
) -> Result<Json<ApiResponse<JsonArrInsertResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.json_service.json_arr_insert(&key, &request.path, request.index, request.values).await?;

    Ok(Json(ApiResponse::new(JsonArrInsertResponse {
        key: result.key,
        path: result.path,
        new_lengths: result.new_lengths,
    })))
}

/// DELETE /api/v1/json/:key/arrpop
///
/// Pop an element from a JSON array (JSON.ARRPOP).
#[utoipa::path(
    delete,
    path = "/api/v1/json/{key}/arrpop",
    params(
        ("key" = String, Path, description = "The key")
    ),
    request_body = JsonArrPopRequest,
    responses(
        (status = 200, description = "Element popped", body = JsonArrPopResponse),
        (status = 400, description = "Invalid request or not an array"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_arr_pop(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<JsonArrPopRequest>,
) -> Result<Json<ApiResponse<JsonArrPopResponse>>, CacheError> {
    let result = state
        .json_service
        .json_arr_pop(&key, &request.path, request.index)
        .await?;

    Ok(Json(ApiResponse::new(JsonArrPopResponse {
        key: result.key,
        path: result.path,
        values: result.values,
    })))
}

/// POST /api/v1/json/:key/arrtrim
///
/// Trim a JSON array to a specified range (JSON.ARRTRIM).
#[utoipa::path(
    post,
    path = "/api/v1/json/{key}/arrtrim",
    params(
        ("key" = String, Path, description = "The key")
    ),
    request_body = JsonArrTrimRequest,
    responses(
        (status = 200, description = "Array trimmed", body = JsonArrTrimResponse),
        (status = 400, description = "Invalid request or not an array"),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_arr_trim(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<JsonArrTrimRequest>,
) -> Result<Json<ApiResponse<JsonArrTrimResponse>>, CacheError> {
    let result = state
        .json_service
        .json_arr_trim(&key, &request.path, request.start, request.stop)
        .await?;

    Ok(Json(ApiResponse::new(JsonArrTrimResponse {
        key: result.key,
        path: result.path,
        new_lengths: result.new_lengths,
    })))
}

// ==================== Object Operations ====================

/// GET /api/v1/json/:key/objlen
///
/// Get the number of keys in a JSON object at path (JSON.OBJLEN).
#[utoipa::path(
    get,
    path = "/api/v1/json/{key}/objlen",
    params(
        ("key" = String, Path, description = "The key"),
        ("path" = Option<String>, Query, description = "JSONPath to the object (default: $)")
    ),
    responses(
        (status = 200, description = "Object length retrieved", body = JsonObjLenResponse),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_obj_len(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<JsonObjLenParams>,
) -> Result<Json<ApiResponse<JsonObjLenResponse>>, CacheError> {
    let result = state.json_service.json_obj_len(&key, &params.path).await?;

    Ok(Json(ApiResponse::new(JsonObjLenResponse {
        key: result.key,
        path: result.path,
        lengths: result.lengths,
    })))
}

/// GET /api/v1/json/:key/objkeys
///
/// Get the keys of a JSON object at path (JSON.OBJKEYS).
#[utoipa::path(
    get,
    path = "/api/v1/json/{key}/objkeys",
    params(
        ("key" = String, Path, description = "The key"),
        ("path" = Option<String>, Query, description = "JSONPath to the object (default: $)")
    ),
    responses(
        (status = 200, description = "Object keys retrieved", body = JsonObjKeysResponse),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_obj_keys(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<JsonObjKeysParams>,
) -> Result<Json<ApiResponse<JsonObjKeysResponse>>, CacheError> {
    let result = state.json_service.json_obj_keys(&key, &params.path).await?;

    Ok(Json(ApiResponse::new(JsonObjKeysResponse {
        key: result.key,
        path: result.path,
        keys: result.keys,
    })))
}

// ==================== Debug Operations ====================

/// GET /api/v1/json/:key/debug/memory
///
/// Get the memory usage of a JSON value at path (JSON.DEBUG MEMORY).
#[utoipa::path(
    get,
    path = "/api/v1/json/{key}/debug/memory",
    params(
        ("key" = String, Path, description = "The key"),
        ("path" = Option<String>, Query, description = "JSONPath (default: $)")
    ),
    responses(
        (status = 200, description = "Memory usage retrieved", body = JsonDebugMemoryResponse),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_debug_memory(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<JsonDebugMemoryParams>,
) -> Result<Json<ApiResponse<JsonDebugMemoryResponse>>, CacheError> {
    let result = state
        .json_service
        .json_debug_memory(&key, &params.path)
        .await?;

    Ok(Json(ApiResponse::new(JsonDebugMemoryResponse {
        key: result.key,
        path: result.path,
        memory_bytes: result.memory_bytes,
    })))
}

/// GET /api/v1/json/:key/resp
///
/// Get the RESP representation of a JSON value (JSON.RESP).
#[utoipa::path(
    get,
    path = "/api/v1/json/{key}/resp",
    params(
        ("key" = String, Path, description = "The key"),
        ("path" = Option<String>, Query, description = "JSONPath (default: $)")
    ),
    responses(
        (status = 200, description = "RESP representation retrieved", body = JsonRespResponse),
        (status = 501, description = "RedisJSON module not available")
    ),
    tag = "JSON"
)]
async fn json_resp(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<JsonRespParams>,
) -> Result<Json<ApiResponse<JsonRespResponse>>, CacheError> {
    let result = state.json_service.json_resp(&key, &params.path).await?;

    Ok(Json(ApiResponse::new(JsonRespResponse {
        key: result.key,
        path: result.path,
        resp: result.resp,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::{Path, State};
    use axum::http::Request;
    use axum::Json;
    use serde_json::json;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    use crate::application::services::{
        AdminService, BitMapService, BloomService, GeoService, HashService, JsonService, KeyService, ListService, ProbabilisticService, PubSubService, SearchService, SetService,
        SortedSetService, StreamService, StringService,
    };
    use crate::api::http::schemas::json::JsonMSetItemRequest;
    use crate::domain::entities::{
        JsonArrAppendResult, JsonArrIndexResult, JsonArrInsertResult, JsonArrLenResult,
        JsonArrPopResult, JsonArrTrimResult, JsonClearResult, JsonDebugMemoryResult, JsonDelResult,
        JsonGetResult, JsonMGetResult, JsonMSetItem, JsonNumResult, JsonObjKeysResult,
        JsonObjLenResult, JsonRespResult, JsonSetOptions, JsonSetResult, JsonStrAppendResult,
        JsonStrLenResult, JsonToggleResult, JsonTypeResult,
    };
    use crate::domain::repositories::JsonRepository;
    use crate::infrastructure::config::Settings;
    use crate::infrastructure::redis::capabilities::RedisCapabilities;
    use crate::infrastructure::redis::connection::InstrumentedPool;
    use crate::test_support::{
        MockAdminRepository, MockBitMapRepository, MockBloomRepository, MockGeoRepository, MockHashRepository, MockJsonRepository, MockKeyRepository,
        MockListRepository, MockProbabilisticRepository, MockSearchRepository, MockSetRepository, MockSortedSetRepository,
        MockStreamRepository, MockStringRepository, test_state_with_json_repo,
    };

    struct SequenceJsonRepository {
        base: MockJsonRepository,
        json_get_results: Mutex<VecDeque<Result<Option<JsonGetResult>, CacheError>>>,
    }

    impl SequenceJsonRepository {
        fn new() -> Self {
            Self {
                base: MockJsonRepository::new(),
                json_get_results: Mutex::new(VecDeque::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl JsonRepository for SequenceJsonRepository {
        async fn json_set(
            &self,
            key: &str,
            path: &str,
            value: serde_json::Value,
            options: JsonSetOptions,
        ) -> Result<JsonSetResult, CacheError> {
            self.base.json_set(key, path, value, options).await
        }

        async fn json_get(
            &self,
            key: &str,
            paths: &[String],
        ) -> Result<Option<JsonGetResult>, CacheError> {
            let next = { self.json_get_results.lock().expect("json_get lock").pop_front() };
            if let Some(result) = next {
                return result;
            }
            self.base.json_get(key, paths).await
        }

        async fn json_mget(
            &self,
            keys: &[String],
            path: &str,
        ) -> Result<JsonMGetResult, CacheError> {
            self.base.json_mget(keys, path).await
        }

        async fn json_mset(&self, items: &[JsonMSetItem]) -> Result<(), CacheError> {
            self.base.json_mset(items).await
        }

        async fn json_del(&self, key: &str, path: &str) -> Result<JsonDelResult, CacheError> {
            self.base.json_del(key, path).await
        }

        async fn json_type(&self, key: &str, path: &str) -> Result<JsonTypeResult, CacheError> {
            self.base.json_type(key, path).await
        }

        async fn json_str_len(
            &self,
            key: &str,
            path: &str,
        ) -> Result<JsonStrLenResult, CacheError> {
            self.base.json_str_len(key, path).await
        }

        async fn json_str_append(
            &self,
            key: &str,
            path: &str,
            value: &str,
        ) -> Result<JsonStrAppendResult, CacheError> {
            self.base.json_str_append(key, path, value).await
        }

        async fn json_num_incr_by(
            &self,
            key: &str,
            path: &str,
            value: f64,
        ) -> Result<JsonNumResult, CacheError> {
            self.base.json_num_incr_by(key, path, value).await
        }

        async fn json_num_mult_by(
            &self,
            key: &str,
            path: &str,
            value: f64,
        ) -> Result<JsonNumResult, CacheError> {
            self.base.json_num_mult_by(key, path, value).await
        }

        async fn json_toggle(&self, key: &str, path: &str) -> Result<JsonToggleResult, CacheError> {
            self.base.json_toggle(key, path).await
        }

        async fn json_clear(&self, key: &str, path: &str) -> Result<JsonClearResult, CacheError> {
            self.base.json_clear(key, path).await
        }

        async fn json_arr_len(
            &self,
            key: &str,
            path: &str,
        ) -> Result<JsonArrLenResult, CacheError> {
            self.base.json_arr_len(key, path).await
        }

        async fn json_arr_append(
            &self,
            key: &str,
            path: &str,
            values: &[serde_json::Value],
        ) -> Result<JsonArrAppendResult, CacheError> {
            self.base.json_arr_append(key, path, values).await
        }

        async fn json_arr_index(
            &self,
            key: &str,
            path: &str,
            value: &serde_json::Value,
            start: Option<i64>,
            stop: Option<i64>,
        ) -> Result<JsonArrIndexResult, CacheError> {
            self.base.json_arr_index(key, path, value, start, stop).await
        }

        async fn json_arr_insert(
            &self,
            key: &str,
            path: &str,
            index: i64,
            values: &[serde_json::Value],
        ) -> Result<JsonArrInsertResult, CacheError> {
            self.base.json_arr_insert(key, path, index, values).await
        }

        async fn json_arr_pop(
            &self,
            key: &str,
            path: &str,
            index: Option<i64>,
        ) -> Result<JsonArrPopResult, CacheError> {
            self.base.json_arr_pop(key, path, index).await
        }

        async fn json_arr_trim(
            &self,
            key: &str,
            path: &str,
            start: i64,
            stop: i64,
        ) -> Result<JsonArrTrimResult, CacheError> {
            self.base.json_arr_trim(key, path, start, stop).await
        }

        async fn json_obj_len(
            &self,
            key: &str,
            path: &str,
        ) -> Result<JsonObjLenResult, CacheError> {
            self.base.json_obj_len(key, path).await
        }

        async fn json_obj_keys(
            &self,
            key: &str,
            path: &str,
        ) -> Result<JsonObjKeysResult, CacheError> {
            self.base.json_obj_keys(key, path).await
        }

        async fn json_debug_memory(
            &self,
            key: &str,
            path: &str,
        ) -> Result<JsonDebugMemoryResult, CacheError> {
            self.base.json_debug_memory(key, path).await
        }

        async fn json_resp(&self, key: &str, path: &str) -> Result<JsonRespResult, CacheError> {
            self.base.json_resp(key, path).await
        }
    }

    fn state_with_json_repo(repo: Arc<dyn JsonRepository>) -> AppState {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let config = Arc::new(Settings::default());
        let capabilities = Arc::new(RedisCapabilities::default_capabilities());
        let string_service =
            Arc::new(StringService::new_with_repository(Arc::new(MockStringRepository::new())));
        let hash_service =
            Arc::new(HashService::new_with_repository(Arc::new(MockHashRepository::new())));
        let list_service =
            Arc::new(ListService::new_with_repository(Arc::new(MockListRepository::new())));
        let set_service =
            Arc::new(SetService::new_with_repository(Arc::new(MockSetRepository::new())));
        let sorted_set_service = Arc::new(SortedSetService::new_with_repository(Arc::new(
            MockSortedSetRepository::new(),
        )));
        let bitmap_service =
            Arc::new(BitMapService::new_with_repository(Arc::new(MockBitMapRepository::new())));
        let key_service =
            Arc::new(KeyService::new_with_repository(Arc::new(MockKeyRepository::new())));
        let admin_service =
            Arc::new(AdminService::new_with_repository(Arc::new(MockAdminRepository::default())));
        let stream_service =
            Arc::new(StreamService::new_with_repository(Arc::new(MockStreamRepository::new())));
        let json_service = Arc::new(JsonService::new_with_repository(repo));
        let search_service =
            Arc::new(SearchService::new_with_repository(Arc::new(MockSearchRepository::new())));
        let bloom_service =
            Arc::new(BloomService::new_with_repository(Arc::new(MockBloomRepository::new())));
        let probabilistic_service =
            Arc::new(ProbabilisticService::new_with_repository(Arc::new(MockProbabilisticRepository::new())));
        let geo_service =
            Arc::new(GeoService::new_with_repository(Arc::new(MockGeoRepository::new())));
        let pubsub_manager = Arc::new(
            crate::infrastructure::redis::pubsub_manager::PubSubManager::new(&config.redis.url, config.pubsub.clone())
                .expect("Failed to create PubSubManager for tests")
        );
        let pubsub_service = Arc::new(PubSubService::new(pool.clone(), pubsub_manager));
        let sse_semaphore = Arc::new(tokio::sync::Semaphore::new(config.blocking.max_sse_connections));

        AppState::new_with_services(
            pool,
            config,
            capabilities,
            sse_semaphore,
            string_service,
            hash_service,
            list_service,
            set_service,
            sorted_set_service,
            bitmap_service,
            key_service,
            admin_service,
            stream_service,
            json_service,
            search_service,
            bloom_service,
            probabilistic_service,
            geo_service,
            pubsub_service,
        )
    }

    #[tokio::test]
    async fn test_json_routes_core_operations() {
        let (state, _) = test_state_with_json_repo();
        let app = json_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/json/key")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"value":{"name":"john"},"path":"$"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        // Test GET with single path
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/json/key?path=$.name")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/json/key?path=$.name")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/json/mget")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"keys":["k1","k2"],"path":"$"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/json/mset")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"items":[{"key":"k1","path":"$","value":{"a":1}}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/json/key/type?path=$")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_json_routes_string_numeric_array_object_debug() {
        let (state, _) = test_state_with_json_repo();
        let app = json_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/json/key/strlen?path=$")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/json/key/strappend")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"path":"$","value":"more"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/json/key/numincrby")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"path":"$","value":1}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/json/key/nummultby")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"path":"$","value":2}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/json/key/toggle?path=$")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/json/key/clear?path=$")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/json/key/arrlen?path=$")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/json/key/arrappend")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"path":"$","values":[1]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/json/key/arrindex")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"path":"$","value":1}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/json/key/arrinsert")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"path":"$","index":0,"values":[1]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/json/key/arrpop")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"path":"$","index":0}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/json/key/arrtrim")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"path":"$","start":0,"stop":1}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/json/key/objlen?path=$")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/json/key/objkeys?path=$")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/json/key/debug/memory?path=$")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/json/key/resp?path=$")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_json_get_not_found() {
        let repo = Arc::new(SequenceJsonRepository::new());
        repo.json_get_results
            .lock()
            .expect("json_get lock")
            .push_back(Ok(None));
        let state = state_with_json_repo(repo);
        let app = json_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/json/missing?path=$")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_json_handlers_direct_calls() {
        let (state, _) = test_state_with_json_repo();

        let response = json_set(
            State(state.clone()),
            Path("key".to_string()),
            Json(JsonSetRequest {
                value: json!({"a": 1}),
                path: "$".to_string(),
                nx: false,
                xx: false,
            }),
        )
        .await;
        assert!(response.is_ok());

        let response = json_mget(
            State(state.clone()),
            Json(JsonMGetRequest {
                keys: vec!["k1".to_string()],
                path: "$".to_string(),
            }),
        )
        .await;
        assert!(response.is_ok());

        let response = json_mset(
            State(state.clone()),
            Json(JsonMSetRequest {
                items: vec![JsonMSetItemRequest {
                    key: "k1".to_string(),
                    path: "$".to_string(),
                    value: json!({"a": 1}),
                }],
            }),
        )
        .await;
        assert!(response.is_ok());

        let response = json_str_append(
            State(state.clone()),
            Path("key".to_string()),
            Json(JsonStrAppendRequest {
                path: "$".to_string(),
                value: "more".to_string(),
            }),
        )
        .await;
        assert!(response.is_ok());

        let response = json_arr_append(
            State(state.clone()),
            Path("key".to_string()),
            Json(JsonArrAppendRequest {
                path: "$".to_string(),
                values: vec![json!(1)],
            }),
        )
        .await;
        assert!(response.is_ok());

        let response = json_arr_insert(
            State(state),
            Path("key".to_string()),
            Json(JsonArrInsertRequest {
                path: "$".to_string(),
                index: 0,
                values: vec![json!(1)],
            }),
        )
        .await;
        assert!(response.is_ok());
    }
}
