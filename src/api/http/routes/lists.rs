//! List Routes
//!
//! HTTP endpoints for Redis list operations.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};

use crate::api::http::schemas::lists::{
    BlockingMoveRequest, BlockingPopRequest, BlockingPopResponse,
    ListIndexQuery, ListIndexResponse, ListInsertRequest, ListInsertResponse,
    ListLengthResponse, ListMoveRequest, ListMoveResponse, ListPopRequest, ListPopResponse,
    ListPosQuery, ListPosResponse, ListPushRequest, ListPushResponse, ListRangeQuery,
    ListRemoveRequest, ListRemoveResponse, ListSetRequest, ListTrimRequest,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::LPosOptions;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create list routes
pub fn list_routes() -> Router<AppState> {
    Router::new()
        // Push operations
        .route("/api/v1/lists/{key}/lpush", post(lpush))
        .route("/api/v1/lists/{key}/rpush", post(rpush))
        .route("/api/v1/lists/{key}/lpushx", post(lpush_x))
        .route("/api/v1/lists/{key}/rpushx", post(rpush_x))
        // Pop operations
        .route("/api/v1/lists/{key}/lpop", post(lpop))
        .route("/api/v1/lists/{key}/rpop", post(rpop))
        // Range and length
        .route("/api/v1/lists/{key}/range", get(lrange))
        .route("/api/v1/lists/{key}/length", get(llen))
        // Index operations
        .route("/api/v1/lists/{key}/index", get(lindex))
        .route("/api/v1/lists/{key}/set", patch(lset))
        // Insert and remove
        .route("/api/v1/lists/{key}/insert", post(linsert))
        .route("/api/v1/lists/{key}/remove", delete(lrem))
        .route("/api/v1/lists/{key}/trim", post(ltrim))
        // Position
        .route("/api/v1/lists/{key}/pos", get(lpos))
        // Move operations
        .route("/api/v1/lists/move", post(lmove))
        .route("/api/v1/lists/rpoplpush", post(rpop_lpush))
        // Blocking operations
        .route("/api/v1/lists/blpop", post(blpop))
        .route("/api/v1/lists/brpop", post(brpop))
        .route("/api/v1/lists/blmove", post(blmove))
        .route("/api/v1/lists/brpoplpush", post(brpop_lpush))
}

// ========== Push Operations ==========

/// POST /api/v1/lists/{key}/lpush
///
/// Insert values at the head of the list (LPUSH).
#[utoipa::path(
    post,
    path = "/api/v1/lists/{key}/lpush",
    params(
        ("key" = String, Path, description = "The list key")
    ),
    request_body = ListPushRequest,
    responses(
        (status = 200, description = "Values pushed successfully", body = ListPushResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn lpush(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<ListPushRequest>,
) -> Result<Json<ApiResponse<ListPushResponse>>, CacheError> {
    let length = state.list_service.lpush(&key, req.values).await?;
    Ok(Json(ApiResponse::success(ListPushResponse { length })))
}

/// POST /api/v1/lists/{key}/rpush
///
/// Insert values at the tail of the list (RPUSH).
#[utoipa::path(
    post,
    path = "/api/v1/lists/{key}/rpush",
    params(
        ("key" = String, Path, description = "The list key")
    ),
    request_body = ListPushRequest,
    responses(
        (status = 200, description = "Values pushed successfully", body = ListPushResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn rpush(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<ListPushRequest>,
) -> Result<Json<ApiResponse<ListPushResponse>>, CacheError> {
    let length = state.list_service.rpush(&key, req.values).await?;
    Ok(Json(ApiResponse::success(ListPushResponse { length })))
}

/// POST /api/v1/lists/{key}/lpushx
///
/// Insert values at head only if list exists (LPUSHX).
#[utoipa::path(
    post,
    path = "/api/v1/lists/{key}/lpushx",
    params(
        ("key" = String, Path, description = "The list key")
    ),
    request_body = ListPushRequest,
    responses(
        (status = 200, description = "Values pushed (0 if list doesn't exist)", body = ListPushResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn lpush_x(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<ListPushRequest>,
) -> Result<Json<ApiResponse<ListPushResponse>>, CacheError> {
    let length = state.list_service.lpush_x(&key, req.values).await?;
    Ok(Json(ApiResponse::success(ListPushResponse { length })))
}

/// POST /api/v1/lists/{key}/rpushx
///
/// Insert values at tail only if list exists (RPUSHX).
#[utoipa::path(
    post,
    path = "/api/v1/lists/{key}/rpushx",
    params(
        ("key" = String, Path, description = "The list key")
    ),
    request_body = ListPushRequest,
    responses(
        (status = 200, description = "Values pushed (0 if list doesn't exist)", body = ListPushResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn rpush_x(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<ListPushRequest>,
) -> Result<Json<ApiResponse<ListPushResponse>>, CacheError> {
    let length = state.list_service.rpush_x(&key, req.values).await?;
    Ok(Json(ApiResponse::success(ListPushResponse { length })))
}

// ========== Pop Operations ==========

/// POST /api/v1/lists/{key}/lpop
///
/// Remove and return elements from the head of the list (LPOP).
#[utoipa::path(
    post,
    path = "/api/v1/lists/{key}/lpop",
    params(
        ("key" = String, Path, description = "The list key")
    ),
    request_body = ListPopRequest,
    responses(
        (status = 200, description = "Elements popped successfully", body = ListPopResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn lpop(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<ListPopRequest>,
) -> Result<Json<ApiResponse<ListPopResponse>>, CacheError> {
    let values = state.list_service.lpop(&key, req.count).await?;
    Ok(Json(ApiResponse::success(ListPopResponse { values })))
}

/// POST /api/v1/lists/{key}/rpop
///
/// Remove and return elements from the tail of the list (RPOP).
#[utoipa::path(
    post,
    path = "/api/v1/lists/{key}/rpop",
    params(
        ("key" = String, Path, description = "The list key")
    ),
    request_body = ListPopRequest,
    responses(
        (status = 200, description = "Elements popped successfully", body = ListPopResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn rpop(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<ListPopRequest>,
) -> Result<Json<ApiResponse<ListPopResponse>>, CacheError> {
    let values = state.list_service.rpop(&key, req.count).await?;
    Ok(Json(ApiResponse::success(ListPopResponse { values })))
}

// ========== Range and Length ==========

/// GET /api/v1/lists/{key}/range
///
/// Get a range of elements from the list (LRANGE).
#[utoipa::path(
    get,
    path = "/api/v1/lists/{key}/range",
    params(
        ("key" = String, Path, description = "The list key"),
        ("start" = i64, Query, description = "Start index (default: 0)"),
        ("stop" = i64, Query, description = "Stop index (default: -1 for end)")
    ),
    responses(
        (status = 200, description = "Elements in range", body = Vec<String>)
    ),
    tag = "Lists"
)]
pub async fn lrange(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<ListRangeQuery>,
) -> Result<Json<ApiResponse<Vec<String>>>, CacheError> {
    let values = state.list_service.lrange(&key, query.start, query.stop).await?;
    Ok(Json(ApiResponse::success(values)))
}

/// GET /api/v1/lists/{key}/length
///
/// Get the length of the list (LLEN).
#[utoipa::path(
    get,
    path = "/api/v1/lists/{key}/length",
    params(
        ("key" = String, Path, description = "The list key")
    ),
    responses(
        (status = 200, description = "Length of the list", body = ListLengthResponse)
    ),
    tag = "Lists"
)]
pub async fn llen(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<ListLengthResponse>>, CacheError> {
    let length = state.list_service.llen(&key).await?;
    Ok(Json(ApiResponse::success(ListLengthResponse { length })))
}

// ========== Index Operations ==========

/// GET /api/v1/lists/{key}/index
///
/// Get element at index (LINDEX).
#[utoipa::path(
    get,
    path = "/api/v1/lists/{key}/index",
    params(
        ("key" = String, Path, description = "The list key"),
        ("index" = i64, Query, description = "Index to retrieve (0-based, negative counts from end)")
    ),
    responses(
        (status = 200, description = "Element at index", body = ListIndexResponse),
        (status = 404, description = "Index out of range")
    ),
    tag = "Lists"
)]
pub async fn lindex(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<ListIndexQuery>,
) -> Result<Json<ApiResponse<ListIndexResponse>>, CacheError> {
    let value = state.list_service.lindex(&key, query.index).await?;
    Ok(Json(ApiResponse::success(ListIndexResponse { value })))
}

/// PATCH /api/v1/lists/{key}/set
///
/// Set element at index (LSET).
#[utoipa::path(
    patch,
    path = "/api/v1/lists/{key}/set",
    params(
        ("key" = String, Path, description = "The list key")
    ),
    request_body = ListSetRequest,
    responses(
        (status = 200, description = "Element set successfully"),
        (status = 400, description = "Index out of range")
    ),
    tag = "Lists"
)]
pub async fn lset(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<ListSetRequest>,
) -> Result<Json<ApiResponse<()>>, CacheError> {
    state.list_service.lset(&key, req.index, &req.value).await?;
    Ok(Json(ApiResponse::success(())))
}

// ========== Insert and Remove ==========

/// POST /api/v1/lists/{key}/insert
///
/// Insert element before or after pivot (LINSERT).
#[utoipa::path(
    post,
    path = "/api/v1/lists/{key}/insert",
    params(
        ("key" = String, Path, description = "The list key")
    ),
    request_body = ListInsertRequest,
    responses(
        (status = 200, description = "Element inserted", body = ListInsertResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn linsert(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<ListInsertRequest>,
) -> Result<Json<ApiResponse<ListInsertResponse>>, CacheError> {
    let length = state
        .list_service
        .linsert(&key, req.position.into(), &req.pivot, &req.value)
        .await?;
    Ok(Json(ApiResponse::success(ListInsertResponse { length })))
}

/// DELETE /api/v1/lists/{key}/remove
///
/// Remove elements equal to value (LREM).
#[utoipa::path(
    delete,
    path = "/api/v1/lists/{key}/remove",
    params(
        ("key" = String, Path, description = "The list key")
    ),
    request_body = ListRemoveRequest,
    responses(
        (status = 200, description = "Elements removed", body = ListRemoveResponse)
    ),
    tag = "Lists"
)]
pub async fn lrem(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<ListRemoveRequest>,
) -> Result<Json<ApiResponse<ListRemoveResponse>>, CacheError> {
    let removed = state.list_service.lrem(&key, req.count, &req.value).await?;
    Ok(Json(ApiResponse::success(ListRemoveResponse { removed })))
}

/// POST /api/v1/lists/{key}/trim
///
/// Trim list to specified range (LTRIM).
#[utoipa::path(
    post,
    path = "/api/v1/lists/{key}/trim",
    params(
        ("key" = String, Path, description = "The list key")
    ),
    request_body = ListTrimRequest,
    responses(
        (status = 200, description = "List trimmed successfully")
    ),
    tag = "Lists"
)]
pub async fn ltrim(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<ListTrimRequest>,
) -> Result<Json<ApiResponse<()>>, CacheError> {
    state.list_service.ltrim(&key, req.start, req.stop).await?;
    Ok(Json(ApiResponse::success(())))
}

// ========== Position ==========

/// GET /api/v1/lists/{key}/pos
///
/// Get index of element in list (LPOS).
#[utoipa::path(
    get,
    path = "/api/v1/lists/{key}/pos",
    params(
        ("key" = String, Path, description = "The list key"),
        ("element" = String, Query, description = "Element to find"),
        ("rank" = Option<i64>, Query, description = "Starting rank for search"),
        ("count" = Option<i64>, Query, description = "Number of indices to return"),
        ("max_len" = Option<i64>, Query, description = "Maximum comparisons")
    ),
    responses(
        (status = 200, description = "Indices found", body = ListPosResponse)
    ),
    tag = "Lists"
)]
pub async fn lpos(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<ListPosQuery>,
) -> Result<Json<ApiResponse<ListPosResponse>>, CacheError> {
    let options = LPosOptions {
        count: query.count,
        rank: query.rank,
        max_len: query.max_len,
    };
    let indices = state.list_service.lpos(&key, &query.element, options).await?;
    Ok(Json(ApiResponse::success(ListPosResponse { indices })))
}

// ========== Move Operations ==========

/// POST /api/v1/lists/move
///
/// Move element from source to destination (LMOVE).
#[utoipa::path(
    post,
    path = "/api/v1/lists/move",
    request_body = ListMoveRequest,
    responses(
        (status = 200, description = "Element moved", body = ListMoveResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn lmove(
    State(state): State<AppState>,
    Json(req): Json<ListMoveRequest>,
) -> Result<Json<ApiResponse<ListMoveResponse>>, CacheError> {
    let value = state
        .list_service
        .lmove(
            &req.source,
            &req.destination,
            req.src_direction.into(),
            req.dst_direction.into(),
        )
        .await?;
    Ok(Json(ApiResponse::success(ListMoveResponse { value })))
}

/// POST /api/v1/lists/rpoplpush
///
/// Pop from source tail and push to destination head (RPOPLPUSH, deprecated - use LMOVE).
#[utoipa::path(
    post,
    path = "/api/v1/lists/rpoplpush",
    request_body = ListMoveRequest,
    responses(
        (status = 200, description = "Element moved", body = ListMoveResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn rpop_lpush(
    State(state): State<AppState>,
    Json(req): Json<ListMoveRequest>,
) -> Result<Json<ApiResponse<ListMoveResponse>>, CacheError> {
    let value = state
        .list_service
        .rpop_lpush(&req.source, &req.destination)
        .await?;
    Ok(Json(ApiResponse::success(ListMoveResponse { value })))
}

// ========== Blocking Operations ==========

/// POST /api/v1/lists/blpop
///
/// Blocking pop from head of list(s) (BLPOP).
/// Returns 204 No Content if timeout is reached.
#[utoipa::path(
    post,
    path = "/api/v1/lists/blpop",
    request_body = BlockingPopRequest,
    responses(
        (status = 200, description = "Element popped", body = BlockingPopResponse),
        (status = 204, description = "Timeout reached, no data available"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn blpop(
    State(state): State<AppState>,
    Json(req): Json<BlockingPopRequest>,
) -> Result<impl IntoResponse, CacheError> {
    let result = state
        .list_service
        .blpop(req.keys, req.timeout_seconds)
        .await?;

    match result {
        Some(pop_result) => Ok(Json(ApiResponse::success(BlockingPopResponse {
            key: pop_result.key,
            value: pop_result.value,
        }))
        .into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

/// POST /api/v1/lists/brpop
///
/// Blocking pop from tail of list(s) (BRPOP).
/// Returns 204 No Content if timeout is reached.
#[utoipa::path(
    post,
    path = "/api/v1/lists/brpop",
    request_body = BlockingPopRequest,
    responses(
        (status = 200, description = "Element popped", body = BlockingPopResponse),
        (status = 204, description = "Timeout reached, no data available"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn brpop(
    State(state): State<AppState>,
    Json(req): Json<BlockingPopRequest>,
) -> Result<impl IntoResponse, CacheError> {
    let result = state
        .list_service
        .brpop(req.keys, req.timeout_seconds)
        .await?;

    match result {
        Some(pop_result) => Ok(Json(ApiResponse::success(BlockingPopResponse {
            key: pop_result.key,
            value: pop_result.value,
        }))
        .into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

/// POST /api/v1/lists/blmove
///
/// Blocking move from source to destination (BLMOVE).
/// Returns 204 No Content if timeout is reached.
#[utoipa::path(
    post,
    path = "/api/v1/lists/blmove",
    request_body = BlockingMoveRequest,
    responses(
        (status = 200, description = "Element moved", body = ListMoveResponse),
        (status = 204, description = "Timeout reached, no data available"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn blmove(
    State(state): State<AppState>,
    Json(req): Json<BlockingMoveRequest>,
) -> Result<impl IntoResponse, CacheError> {
    let result = state
        .list_service
        .blmove(
            &req.source,
            &req.destination,
            req.src_direction.into(),
            req.dst_direction.into(),
            req.timeout_seconds,
        )
        .await?;

    match result {
        Some(value) => Ok(Json(ApiResponse::success(ListMoveResponse { value: Some(value) })).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

/// POST /api/v1/lists/brpoplpush
///
/// Blocking pop from source tail and push to destination head (BRPOPLPUSH, deprecated - use BLMOVE).
/// Returns 204 No Content if timeout is reached.
#[utoipa::path(
    post,
    path = "/api/v1/lists/brpoplpush",
    request_body = BlockingMoveRequest,
    responses(
        (status = 200, description = "Element moved", body = ListMoveResponse),
        (status = 204, description = "Timeout reached, no data available"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Lists"
)]
pub async fn brpop_lpush(
    State(state): State<AppState>,
    Json(req): Json<BlockingMoveRequest>,
) -> Result<impl IntoResponse, CacheError> {
    let result = state
        .list_service
        .brpop_lpush(&req.source, &req.destination, req.timeout_seconds)
        .await?;

    match result {
        Some(value) => Ok(Json(ApiResponse::success(ListMoveResponse { value: Some(value) })).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http::schemas::lists::{InsertPositionParam, ListDirectionParam};
    use crate::test_support::test_state_with_list_repo;
    use axum::extract::{Path, Query, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::Json;

    #[tokio::test]
    async fn test_list_routes_push_pop() {
        let (state, _list_repo) = test_state_with_list_repo();
        let state = State(state);

        // Test LPUSH
        let response = lpush(
            state.clone(),
            Path("mylist".to_string()),
            Json(ListPushRequest {
                values: vec!["a".to_string(), "b".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data").length, 2);

        // Test RPUSH
        let response = rpush(
            state.clone(),
            Path("mylist".to_string()),
            Json(ListPushRequest {
                values: vec!["c".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data").length, 3);

        // Test LLEN
        let response = llen(state.clone(), Path("mylist".to_string()))
            .await
            .unwrap();
        assert_eq!(response.0.data.expect("data").length, 3);

        // Test LRANGE
        let response = lrange(
            state.clone(),
            Path("mylist".to_string()),
            Query(ListRangeQuery { start: 0, stop: -1 }),
        )
        .await
        .unwrap();
        let values = response.0.data.expect("data");
        assert_eq!(values.len(), 3);

        // Test LPOP
        let response = lpop(
            state.clone(),
            Path("mylist".to_string()),
            Json(ListPopRequest { count: Some(1) }),
        )
        .await
        .unwrap();
        let values = response.0.data.expect("data").values;
        assert_eq!(values.len(), 1);

        // Test RPOP
        let response = rpop(
            state.clone(),
            Path("mylist".to_string()),
            Json(ListPopRequest { count: Some(1) }),
        )
        .await
        .unwrap();
        let values = response.0.data.expect("data").values;
        assert_eq!(values.len(), 1);
    }

    #[tokio::test]
    async fn test_list_routes_index_operations() {
        let (state, _list_repo) = test_state_with_list_repo();
        let state = State(state);

        // Setup list
        let _ = lpush(
            state.clone(),
            Path("mylist".to_string()),
            Json(ListPushRequest {
                values: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            }),
        )
        .await
        .unwrap();

        // Test LINDEX
        let response = lindex(
            state.clone(),
            Path("mylist".to_string()),
            Query(ListIndexQuery { index: 0 }),
        )
        .await
        .unwrap();
        assert!(response.0.data.expect("data").value.is_some());

        // Test LSET
        let response = lset(
            state.clone(),
            Path("mylist".to_string()),
            Json(ListSetRequest {
                index: 0,
                value: "z".to_string(),
            }),
        )
        .await;
        assert!(response.is_ok());

        // Verify set worked
        let response = lindex(
            state.clone(),
            Path("mylist".to_string()),
            Query(ListIndexQuery { index: 0 }),
        )
        .await
        .unwrap();
        assert_eq!(
            response.0.data.expect("data").value.as_deref(),
            Some("z")
        );
    }

    #[tokio::test]
    async fn test_list_routes_insert_remove() {
        let (state, _list_repo) = test_state_with_list_repo();
        let state = State(state);

        // Setup list
        let _ = lpush(
            state.clone(),
            Path("mylist".to_string()),
            Json(ListPushRequest {
                values: vec!["a".to_string(), "b".to_string()],
            }),
        )
        .await
        .unwrap();

        // Test LINSERT
        let response = linsert(
            state.clone(),
            Path("mylist".to_string()),
            Json(ListInsertRequest {
                position: InsertPositionParam::After,
                pivot: "b".to_string(),
                value: "c".to_string(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data").length, 3);

        // Test LREM
        let response = lrem(
            state.clone(),
            Path("mylist".to_string()),
            Json(ListRemoveRequest {
                count: 1,
                value: "a".to_string(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data").removed, 1);

        // Test LTRIM
        let response = ltrim(
            state.clone(),
            Path("mylist".to_string()),
            Json(ListTrimRequest { start: 0, stop: 0 }),
        )
        .await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_list_routes_push_variants_and_pos() {
        let (state, list_repo) = test_state_with_list_repo();
        list_repo.insert(
            "existing",
            vec!["a".to_string(), "b".to_string()],
        );
        let state = State(state);

        let response = lpush_x(
            state.clone(),
            Path("existing".to_string()),
            Json(ListPushRequest {
                values: vec!["z".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data").length, 3);

        let response = rpush_x(
            state.clone(),
            Path("existing".to_string()),
            Json(ListPushRequest {
                values: vec!["y".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data").length, 4);

        let response = lpos(
            state,
            Path("existing".to_string()),
            Query(ListPosQuery {
                element: "a".to_string(),
                rank: Some(1),
                count: Some(2),
                max_len: Some(10),
            }),
        )
        .await
        .unwrap();
        assert_eq!(response.0.data.expect("data").indices, vec![1]);
    }

    #[tokio::test]
    async fn test_list_routes_move_and_blocking() {
        let (state, list_repo) = test_state_with_list_repo();
        list_repo.insert(
            "source",
            vec!["a".to_string(), "b".to_string()],
        );
        let state = State(state);

        let moved = lmove(
            state.clone(),
            Json(ListMoveRequest {
                source: "source".to_string(),
                destination: "dest".to_string(),
                src_direction: ListDirectionParam::Left,
                dst_direction: ListDirectionParam::Right,
            }),
        )
        .await
        .unwrap();
        assert_eq!(moved.0.data.expect("data").value.as_deref(), Some("a"));

        let moved = rpop_lpush(
            state.clone(),
            Json(ListMoveRequest {
                source: "source".to_string(),
                destination: "dest2".to_string(),
                src_direction: ListDirectionParam::Right,
                dst_direction: ListDirectionParam::Left,
            }),
        )
        .await
        .unwrap();
        assert_eq!(moved.0.data.expect("data").value.as_deref(), Some("b"));

        list_repo.insert("block1", vec!["x".to_string()]);
        let response = blpop(
            state.clone(),
            Json(BlockingPopRequest {
                keys: vec!["block1".to_string()],
                timeout_seconds: 1,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = blpop(
            state.clone(),
            Json(BlockingPopRequest {
                keys: vec!["missing".to_string()],
                timeout_seconds: 1,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        list_repo.insert("block2", vec!["y".to_string()]);
        let response = brpop(
            state.clone(),
            Json(BlockingPopRequest {
                keys: vec!["block2".to_string()],
                timeout_seconds: 1,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = brpop(
            state.clone(),
            Json(BlockingPopRequest {
                keys: vec!["missing2".to_string()],
                timeout_seconds: 1,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        list_repo.insert("movesrc", vec!["m1".to_string()]);
        let response = blmove(
            state.clone(),
            Json(BlockingMoveRequest {
                source: "movesrc".to_string(),
                destination: "movedst".to_string(),
                src_direction: ListDirectionParam::Left,
                dst_direction: ListDirectionParam::Right,
                timeout_seconds: 1,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = blmove(
            state.clone(),
            Json(BlockingMoveRequest {
                source: "missing_move".to_string(),
                destination: "movedst".to_string(),
                src_direction: ListDirectionParam::Left,
                dst_direction: ListDirectionParam::Right,
                timeout_seconds: 1,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        list_repo.insert("movesrc2", vec!["m2".to_string()]);
        let response = brpop_lpush(
            state.clone(),
            Json(BlockingMoveRequest {
                source: "movesrc2".to_string(),
                destination: "movedst2".to_string(),
                src_direction: ListDirectionParam::Right,
                dst_direction: ListDirectionParam::Left,
                timeout_seconds: 1,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = brpop_lpush(
            state,
            Json(BlockingMoveRequest {
                source: "missing_move2".to_string(),
                destination: "movedst2".to_string(),
                src_direction: ListDirectionParam::Right,
                dst_direction: ListDirectionParam::Left,
                timeout_seconds: 1,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}
