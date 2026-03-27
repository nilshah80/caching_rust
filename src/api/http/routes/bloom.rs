//! Bloom Filter Routes
//!
//! HTTP routes for Bloom filter and Cuckoo filter operations.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use validator::Validate;

use crate::api::http::schemas::bloom::{
    BloomAddRequest, BloomAddResponse, BloomCardResponse, BloomExistsRequest, BloomExistsResponse,
    BloomInfoResponse, BloomInsertRequest, BloomInsertResponse, BloomLoadChunkRequest,
    BloomLoadChunkResponse, BloomReserveRequest, BloomReserveResponse, BloomScanDumpParams,
    BloomScanDumpResponse, CuckooAddRequest, CuckooAddResponse, CuckooCountRequest,
    CuckooCountResponse, CuckooDelRequest, CuckooDelResponse, CuckooExistsRequest,
    CuckooExistsResponse, CuckooInfoResponse, CuckooInsertRequest, CuckooInsertResponse,
    CuckooLoadChunkRequest, CuckooLoadChunkResponse, CuckooReserveRequest, CuckooReserveResponse,
    CuckooScanDumpParams, CuckooScanDumpResponse,
};
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Build Bloom filter routes
pub fn bloom_routes() -> Router<AppState> {
    Router::new()
        // Bloom filter operations
        .route("/api/v1/bloom/{key}", post(bf_reserve))
        .route("/api/v1/bloom/{key}", get(bf_info))
        .route("/api/v1/bloom/{key}/add", post(bf_add))
        .route("/api/v1/bloom/{key}/exists", post(bf_exists))
        .route("/api/v1/bloom/{key}/insert", post(bf_insert))
        .route("/api/v1/bloom/{key}/card", get(bf_card))
        .route("/api/v1/bloom/{key}/scandump", get(bf_scandump))
        .route("/api/v1/bloom/{key}/loadchunk", post(bf_loadchunk))
        // Cuckoo filter operations
        .route("/api/v1/cuckoo/{key}", post(cf_reserve))
        .route("/api/v1/cuckoo/{key}", get(cf_info))
        .route("/api/v1/cuckoo/{key}/add", post(cf_add))
        .route("/api/v1/cuckoo/{key}/addnx", post(cf_addnx))
        .route("/api/v1/cuckoo/{key}/exists", post(cf_exists))
        .route("/api/v1/cuckoo/{key}/insert", post(cf_insert))
        .route("/api/v1/cuckoo/{key}/insertnx", post(cf_insertnx))
        .route("/api/v1/cuckoo/{key}/del", delete(cf_del))
        .route("/api/v1/cuckoo/{key}/count", post(cf_count))
        .route("/api/v1/cuckoo/{key}/scandump", get(cf_scandump))
        .route("/api/v1/cuckoo/{key}/loadchunk", post(cf_loadchunk))
}

// ==================== Bloom Filter Handlers ====================

/// POST /api/v1/bloom/:key
///
/// Create a new Bloom filter (BF.RESERVE)
#[utoipa::path(
    post,
    path = "/api/v1/bloom/{key}",
    params(
        ("key" = String, Path, description = "Bloom filter key")
    ),
    request_body = BloomReserveRequest,
    responses(
        (status = 200, description = "Bloom filter created", body = BloomReserveResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Bloom Filters"
)]
async fn bf_reserve(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<BloomReserveRequest>,
) -> Result<Json<ApiResponse<BloomReserveResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let options = request.into();
    let result = state.bloom_service.bf_reserve(&key, options).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/bloom/:key
///
/// Get information about a Bloom filter (BF.INFO)
#[utoipa::path(
    get,
    path = "/api/v1/bloom/{key}",
    params(
        ("key" = String, Path, description = "Bloom filter key")
    ),
    responses(
        (status = 200, description = "Bloom filter info", body = BloomInfoResponse),
        (status = 404, description = "Filter not found"),
        (status = 500, description = "Server error")
    ),
    tag = "Bloom Filters"
)]
async fn bf_info(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<BloomInfoResponse>>, CacheError> {
    let result = state.bloom_service.bf_info(&key).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/bloom/:key/add
///
/// Add items to a Bloom filter (BF.ADD/BF.MADD)
#[utoipa::path(
    post,
    path = "/api/v1/bloom/{key}/add",
    params(
        ("key" = String, Path, description = "Bloom filter key")
    ),
    request_body = BloomAddRequest,
    responses(
        (status = 200, description = "Items added", body = BloomAddResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Bloom Filters"
)]
async fn bf_add(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<BloomAddRequest>,
) -> Result<Json<ApiResponse<BloomAddResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = if request.items.len() == 1 {
        state.bloom_service.bf_add(&key, &request.items[0]).await?
    } else {
        state.bloom_service.bf_madd(&key, request.items).await?
    };

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/bloom/:key/exists
///
/// Check if items exist in a Bloom filter (BF.EXISTS/BF.MEXISTS)
#[utoipa::path(
    post,
    path = "/api/v1/bloom/{key}/exists",
    params(
        ("key" = String, Path, description = "Bloom filter key")
    ),
    request_body = BloomExistsRequest,
    responses(
        (status = 200, description = "Existence check results", body = BloomExistsResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Bloom Filters"
)]
async fn bf_exists(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<BloomExistsRequest>,
) -> Result<Json<ApiResponse<BloomExistsResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = if request.items.len() == 1 {
        state
            .bloom_service
            .bf_exists(&key, &request.items[0])
            .await?
    } else {
        state.bloom_service.bf_mexists(&key, request.items).await?
    };

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/bloom/:key/insert
///
/// Insert items with options (BF.INSERT)
#[utoipa::path(
    post,
    path = "/api/v1/bloom/{key}/insert",
    params(
        ("key" = String, Path, description = "Bloom filter key")
    ),
    request_body = BloomInsertRequest,
    responses(
        (status = 200, description = "Items inserted", body = BloomInsertResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Bloom Filters"
)]
async fn bf_insert(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<BloomInsertRequest>,
) -> Result<Json<ApiResponse<BloomInsertResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let items = request.items.clone();
    let options = request.into();
    let result = state.bloom_service.bf_insert(&key, options, items).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/bloom/:key/card
///
/// Get estimated cardinality of a Bloom filter (BF.CARD)
#[utoipa::path(
    get,
    path = "/api/v1/bloom/{key}/card",
    params(
        ("key" = String, Path, description = "Bloom filter key")
    ),
    responses(
        (status = 200, description = "Cardinality estimate", body = BloomCardResponse),
        (status = 404, description = "Filter not found"),
        (status = 500, description = "Server error")
    ),
    tag = "Bloom Filters"
)]
async fn bf_card(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<BloomCardResponse>>, CacheError> {
    let result = state.bloom_service.bf_card(&key).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/bloom/:key/scandump
///
/// Begin incremental save of a Bloom filter (BF.SCANDUMP)
#[utoipa::path(
    get,
    path = "/api/v1/bloom/{key}/scandump",
    params(
        ("key" = String, Path, description = "Bloom filter key"),
        ("iterator" = Option<u64>, Query, description = "Iterator position (start with 0)")
    ),
    responses(
        (status = 200, description = "Scan dump chunk", body = BloomScanDumpResponse),
        (status = 404, description = "Filter not found"),
        (status = 500, description = "Server error")
    ),
    tag = "Bloom Filters"
)]
async fn bf_scandump(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<BloomScanDumpParams>,
) -> Result<Json<ApiResponse<BloomScanDumpResponse>>, CacheError> {
    let result = state
        .bloom_service
        .bf_scandump(&key, params.iterator)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/bloom/:key/loadchunk
///
/// Restore a Bloom filter from a dump (BF.LOADCHUNK)
#[utoipa::path(
    post,
    path = "/api/v1/bloom/{key}/loadchunk",
    params(
        ("key" = String, Path, description = "Bloom filter key")
    ),
    request_body = BloomLoadChunkRequest,
    responses(
        (status = 200, description = "Chunk loaded", body = BloomLoadChunkResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Bloom Filters"
)]
async fn bf_loadchunk(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<BloomLoadChunkRequest>,
) -> Result<Json<ApiResponse<BloomLoadChunkResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let data = BASE64
        .decode(&request.data)
        .map_err(|e| CacheError::InvalidInput(format!("Invalid base64 data: {}", e)))?;

    let future = state
        .bloom_service
        .bf_loadchunk(&key, request.iterator, &data);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

// ==================== Cuckoo Filter Handlers ====================

/// POST /api/v1/cuckoo/:key
///
/// Create a new Cuckoo filter (CF.RESERVE)
#[utoipa::path(
    post,
    path = "/api/v1/cuckoo/{key}",
    params(
        ("key" = String, Path, description = "Cuckoo filter key")
    ),
    request_body = CuckooReserveRequest,
    responses(
        (status = 200, description = "Cuckoo filter created", body = CuckooReserveResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Cuckoo Filters"
)]
async fn cf_reserve(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CuckooReserveRequest>,
) -> Result<Json<ApiResponse<CuckooReserveResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let options = request.into();
    let result = state.bloom_service.cf_reserve(&key, options).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/cuckoo/:key
///
/// Get information about a Cuckoo filter (CF.INFO)
#[utoipa::path(
    get,
    path = "/api/v1/cuckoo/{key}",
    params(
        ("key" = String, Path, description = "Cuckoo filter key")
    ),
    responses(
        (status = 200, description = "Cuckoo filter info", body = CuckooInfoResponse),
        (status = 404, description = "Filter not found"),
        (status = 500, description = "Server error")
    ),
    tag = "Cuckoo Filters"
)]
async fn cf_info(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<CuckooInfoResponse>>, CacheError> {
    let result = state.bloom_service.cf_info(&key).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/cuckoo/:key/add
///
/// Add an item to a Cuckoo filter (CF.ADD)
#[utoipa::path(
    post,
    path = "/api/v1/cuckoo/{key}/add",
    params(
        ("key" = String, Path, description = "Cuckoo filter key")
    ),
    request_body = CuckooAddRequest,
    responses(
        (status = 200, description = "Item added", body = CuckooAddResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Cuckoo Filters"
)]
async fn cf_add(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CuckooAddRequest>,
) -> Result<Json<ApiResponse<CuckooAddResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.bloom_service.cf_add(&key, &request.item).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/cuckoo/:key/addnx
///
/// Add an item only if it doesn't exist (CF.ADDNX)
#[utoipa::path(
    post,
    path = "/api/v1/cuckoo/{key}/addnx",
    params(
        ("key" = String, Path, description = "Cuckoo filter key")
    ),
    request_body = CuckooAddRequest,
    responses(
        (status = 200, description = "Item added if not exists", body = CuckooAddResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Cuckoo Filters"
)]
async fn cf_addnx(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CuckooAddRequest>,
) -> Result<Json<ApiResponse<CuckooAddResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.bloom_service.cf_addnx(&key, &request.item).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/cuckoo/:key/exists
///
/// Check if items exist in a Cuckoo filter (CF.EXISTS/CF.MEXISTS)
#[utoipa::path(
    post,
    path = "/api/v1/cuckoo/{key}/exists",
    params(
        ("key" = String, Path, description = "Cuckoo filter key")
    ),
    request_body = CuckooExistsRequest,
    responses(
        (status = 200, description = "Existence check results", body = CuckooExistsResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Cuckoo Filters"
)]
async fn cf_exists(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CuckooExistsRequest>,
) -> Result<Json<ApiResponse<CuckooExistsResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = if request.items.len() == 1 {
        state
            .bloom_service
            .cf_exists(&key, &request.items[0])
            .await?
    } else {
        state.bloom_service.cf_mexists(&key, request.items).await?
    };

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/cuckoo/:key/insert
///
/// Insert items with options (CF.INSERT)
#[utoipa::path(
    post,
    path = "/api/v1/cuckoo/{key}/insert",
    params(
        ("key" = String, Path, description = "Cuckoo filter key")
    ),
    request_body = CuckooInsertRequest,
    responses(
        (status = 200, description = "Items inserted", body = CuckooInsertResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Cuckoo Filters"
)]
async fn cf_insert(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CuckooInsertRequest>,
) -> Result<Json<ApiResponse<CuckooInsertResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let items = request.items.clone();
    let options = request.into();
    let result = state.bloom_service.cf_insert(&key, options, items).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/cuckoo/:key/insertnx
///
/// Insert items only if they don't exist (CF.INSERTNX)
#[utoipa::path(
    post,
    path = "/api/v1/cuckoo/{key}/insertnx",
    params(
        ("key" = String, Path, description = "Cuckoo filter key")
    ),
    request_body = CuckooInsertRequest,
    responses(
        (status = 200, description = "Items inserted if not exist", body = CuckooInsertResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Cuckoo Filters"
)]
async fn cf_insertnx(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CuckooInsertRequest>,
) -> Result<Json<ApiResponse<CuckooInsertResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let items = request.items.clone();
    let options = request.into();
    let result = state
        .bloom_service
        .cf_insertnx(&key, options, items)
        .await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// DELETE /api/v1/cuckoo/:key/del
///
/// Delete an item from a Cuckoo filter (CF.DEL)
#[utoipa::path(
    delete,
    path = "/api/v1/cuckoo/{key}/del",
    params(
        ("key" = String, Path, description = "Cuckoo filter key")
    ),
    request_body = CuckooDelRequest,
    responses(
        (status = 200, description = "Item deleted", body = CuckooDelResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Cuckoo Filters"
)]
async fn cf_del(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CuckooDelRequest>,
) -> Result<Json<ApiResponse<CuckooDelResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.bloom_service.cf_del(&key, &request.item).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/cuckoo/:key/count
///
/// Count occurrences of an item (CF.COUNT)
#[utoipa::path(
    post,
    path = "/api/v1/cuckoo/{key}/count",
    params(
        ("key" = String, Path, description = "Cuckoo filter key")
    ),
    request_body = CuckooCountRequest,
    responses(
        (status = 200, description = "Item count", body = CuckooCountResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Cuckoo Filters"
)]
async fn cf_count(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CuckooCountRequest>,
) -> Result<Json<ApiResponse<CuckooCountResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.bloom_service.cf_count(&key, &request.item).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/cuckoo/:key/scandump
///
/// Begin incremental save of a Cuckoo filter (CF.SCANDUMP)
#[utoipa::path(
    get,
    path = "/api/v1/cuckoo/{key}/scandump",
    params(
        ("key" = String, Path, description = "Cuckoo filter key"),
        ("iterator" = Option<u64>, Query, description = "Iterator position (start with 0)")
    ),
    responses(
        (status = 200, description = "Scan dump chunk", body = CuckooScanDumpResponse),
        (status = 404, description = "Filter not found"),
        (status = 500, description = "Server error")
    ),
    tag = "Cuckoo Filters"
)]
async fn cf_scandump(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<CuckooScanDumpParams>,
) -> Result<Json<ApiResponse<CuckooScanDumpResponse>>, CacheError> {
    let result = state
        .bloom_service
        .cf_scandump(&key, params.iterator)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/cuckoo/:key/loadchunk
///
/// Restore a Cuckoo filter from a dump (CF.LOADCHUNK)
#[utoipa::path(
    post,
    path = "/api/v1/cuckoo/{key}/loadchunk",
    params(
        ("key" = String, Path, description = "Cuckoo filter key")
    ),
    request_body = CuckooLoadChunkRequest,
    responses(
        (status = 200, description = "Chunk loaded", body = CuckooLoadChunkResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Cuckoo Filters"
)]
async fn cf_loadchunk(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CuckooLoadChunkRequest>,
) -> Result<Json<ApiResponse<CuckooLoadChunkResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let data = BASE64
        .decode(&request.data)
        .map_err(|e| CacheError::InvalidInput(format!("Invalid base64 data: {}", e)))?;

    let future = state
        .bloom_service
        .cf_loadchunk(&key, request.iterator, &data);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state_with_bloom_repo;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_bloom_routes() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test BF.RESERVE
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"error_rate":0.01,"capacity":1000}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test BF.INFO
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/bloom/myfilter")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test BF.ADD
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter/add")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["item1","item2"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test BF.EXISTS
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter/exists")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["item1"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test BF.MEXISTS branch with multiple items
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter/exists")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["item1","item2"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test BF.INSERT
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter/insert")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["item3"],"capacity":1000}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test BF.CARD
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/bloom/myfilter/card")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cuckoo_routes() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test CF.RESERVE
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"capacity":1000}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test CF.INFO
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/cuckoo/myfilter")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test CF.ADD
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter/add")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"item":"item1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test CF.ADDNX
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter/addnx")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"item":"item2"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test CF.EXISTS
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter/exists")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["item1","item2"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test CF.EXISTS single-item branch
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter/exists")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["item1"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test CF.INSERT
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter/insert")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["item3","item4"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test CF.DEL
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/cuckoo/myfilter/del")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"item":"item1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test CF.COUNT
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter/count")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"item":"item2"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bf_scandump_route() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test BF.SCANDUMP
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/bloom/myfilter/scandump?iterator=0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bf_loadchunk_route() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test BF.LOADCHUNK with valid base64
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter/loadchunk")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"iterator":0,"data":"SGVsbG8gV29ybGQ="}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bf_loadchunk_invalid_base64() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test BF.LOADCHUNK with invalid base64
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter/loadchunk")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"iterator":0,"data":"!!!invalid base64!!!"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_cf_scandump_route() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test CF.SCANDUMP
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/cuckoo/myfilter/scandump?iterator=0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cf_loadchunk_route() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test CF.LOADCHUNK with valid base64
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter/loadchunk")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"iterator":0,"data":"SGVsbG8gV29ybGQ="}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cf_loadchunk_invalid_base64() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test CF.LOADCHUNK with invalid base64
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter/loadchunk")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"iterator":0,"data":"!!!invalid base64!!!"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_cf_insertnx_route() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test CF.INSERTNX
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter/insertnx")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["item1","item2"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bf_insert_validation_errors() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test BF.INSERT with NONSCALING + EXPANSION (should fail)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter/insert")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"items":["item1"],"nonscaling":true,"expansion":2}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test BF.INSERT with zero capacity (should fail)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter/insert")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["item1"],"capacity":0}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test BF.INSERT with invalid error rate (should fail)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter/insert")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["item1"],"error_rate":1.5}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_bf_reserve_validation_errors() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test BF.RESERVE with NONSCALING + EXPANSION (should fail)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"error_rate":0.01,"capacity":1000,"nonscaling":true,"expansion":2}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_cf_reserve_validation_errors() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test CF.RESERVE with zero capacity (should fail)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"capacity":0}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test CF.RESERVE with zero bucket_size (should fail)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"capacity":1000,"bucket_size":0}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_cf_insert_validation_errors() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test CF.INSERT with zero capacity (should fail)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cuckoo/myfilter/insert")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["item1"],"capacity":0}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_empty_items_validation() {
        let (state, _) = test_state_with_bloom_repo();
        let app = bloom_routes().with_state(state);

        // Test BF.ADD with empty items array (should fail)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter/add")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test BF.ADD with empty string in items (should fail)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bloom/myfilter/add")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":[""]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
