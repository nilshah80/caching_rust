//! Hash Routes
//!
//! HTTP endpoints for Redis hash operations.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
    Json, Router,
};

use crate::api::http::schemas::hashes::{
    GetMultipleFieldsRequest, HashFieldEntry, HashIncrFloatRequest, HashIncrRequest,
    HashRandomFieldResponse, HashScanResponse, RandomFieldQuery, ScanHashQuery,
    SetHashNxRequest, SetHashRequest,
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
        .route("/api/v1/hashes/{key}/fields/{field}/incr-float", patch(hincr_by_float))
        .route("/api/v1/hashes/{key}/fields/{field}/length", get(hstr_len))
        .route("/api/v1/hashes/{key}/random", get(hrand_field))
        .route("/api/v1/hashes/{key}/scan", get(hscan))
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
    let result = state.hash_service.hset_nx(&key, &req.field, &req.value).await?;
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
    let result = state.hash_service.hincr_by_float(&key, &field, req.delta).await?;
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
    let result = state.hash_service.hrand_field(&key, query.count, query.with_values).await?;
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
    Ok(Json(ApiResponse::success(HashScanResponse { cursor, entries })))
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
    use crate::test_support::test_state_with_hash_repo;
    use axum::extract::{Path, Query, State};
    use axum::Json;

    #[tokio::test]
    async fn test_hash_routes_basic() {
        let (state, hash_repo) = test_state_with_hash_repo();
        hash_repo.insert("hash1", "field1", "1");
        hash_repo.insert("hash1", "field2", "2");
        let state = State(state);

        let response = hget(state.clone(), Path(("hash1".to_string(), "field1".to_string())))
            .await
            .unwrap();
        let value = response.0.data.expect("data");
        assert_eq!(value.as_deref(), Some("1"));

        let response = hget(state.clone(), Path(("hash1".to_string(), "missing".to_string())))
            .await
            .unwrap();
        let value = response.0.data.expect("data");
        assert!(value.is_none());

        let response = hset(
            state.clone(),
            Path("hash1".to_string()),
            Json(SetHashRequest {
                items: [("field3".to_string(), "3".to_string())].into_iter().collect(),
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

        let response = hexists(state.clone(), Path(("hash1".to_string(), "field1".to_string())))
            .await
            .unwrap();
        assert!(response.0.data.expect("data"));

        let response = hkeys(state.clone(), Path("hash1".to_string()))
            .await
            .unwrap();
        assert!(response.0.data.expect("data").contains(&"field1".to_string()));

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
            Query(RandomFieldQuery { count: None, with_values: false }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data").fields.unwrap().len(), 1);

        let response = hrand_field(
            state.clone(),
            Path("hash2".to_string()),
            Query(RandomFieldQuery { count: Some(1), with_values: true }),
        )
        .await
        .unwrap();
        let entries = response.0.data.expect("data").entries.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].field.is_empty());

        let err = hrand_field(
            state.clone(),
            Path("hash2".to_string()),
            Query(RandomFieldQuery { count: None, with_values: true }),
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
}
