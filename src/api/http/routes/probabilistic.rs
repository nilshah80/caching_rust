//! Probabilistic Data Structure Routes
//!
//! HTTP routes for Count-Min Sketch, Top-K, and HyperLogLog operations.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use validator::Validate;

use crate::api::http::schemas::probabilistic::{
    CmsIncrByRequest, CmsIncrByResponse, CmsInfoResponse, CmsInitByDimRequest,
    CmsInitByProbRequest, CmsInitResponse, CmsMergeRequest, CmsMergeResponse, CmsQueryRequest,
    CmsQueryResponse, PfAddRequest, PfAddResponse, PfCountRequest, PfCountResponse, PfMergeRequest,
    PfMergeResponse, TopKAddRequest, TopKAddResponse, TopKCountResponse, TopKIncrByRequest,
    TopKIncrByResponse, TopKInfoResponse, TopKListQuery, TopKListResponse, TopKQueryRequest,
    TopKQueryResponse, TopKReserveRequest, TopKReserveResponse,
};
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Build Count-Min Sketch routes
pub fn cms_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/cms/{key}/initbydim", post(cms_init_by_dim))
        .route("/api/v1/cms/{key}/initbyprob", post(cms_init_by_prob))
        .route("/api/v1/cms/{key}/incrby", post(cms_incr_by))
        .route("/api/v1/cms/{key}/query", post(cms_query))
        .route("/api/v1/cms/{key}/merge", post(cms_merge))
        .route("/api/v1/cms/{key}", get(cms_info))
}

/// Build Top-K routes
pub fn topk_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/topk/{key}", post(topk_reserve))
        .route("/api/v1/topk/{key}", get(topk_info))
        .route("/api/v1/topk/{key}/add", post(topk_add))
        .route("/api/v1/topk/{key}/incrby", post(topk_incr_by))
        .route("/api/v1/topk/{key}/query", post(topk_query))
        .route("/api/v1/topk/{key}/count", post(topk_count))
        .route("/api/v1/topk/{key}/list", get(topk_list))
}

/// Build HyperLogLog routes
pub fn hyperloglog_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/hll/{key}/add", post(pf_add))
        .route("/api/v1/hll/count", post(pf_count))
        .route("/api/v1/hll/{key}/merge", post(pf_merge))
}

// ==================== Count-Min Sketch Handlers ====================

/// POST /api/v1/cms/:key/initbydim
///
/// Initialize a Count-Min Sketch by dimensions (CMS.INITBYDIM)
#[utoipa::path(
    post,
    path = "/api/v1/cms/{key}/initbydim",
    params(
        ("key" = String, Path, description = "Count-Min Sketch key")
    ),
    request_body = CmsInitByDimRequest,
    responses(
        (status = 200, description = "CMS created", body = CmsInitResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Count-Min Sketch"
)]
async fn cms_init_by_dim(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CmsInitByDimRequest>,
) -> Result<Json<ApiResponse<CmsInitResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let future = state
        .probabilistic_service
        .cms_init_by_dim(&key, request.width, request.depth);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/cms/:key/initbyprob
///
/// Initialize a Count-Min Sketch by probability (CMS.INITBYPROB)
#[utoipa::path(
    post,
    path = "/api/v1/cms/{key}/initbyprob",
    params(
        ("key" = String, Path, description = "Count-Min Sketch key")
    ),
    request_body = CmsInitByProbRequest,
    responses(
        (status = 200, description = "CMS created", body = CmsInitResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Count-Min Sketch"
)]
async fn cms_init_by_prob(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CmsInitByProbRequest>,
) -> Result<Json<ApiResponse<CmsInitResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let future =
        state
            .probabilistic_service
            .cms_init_by_prob(&key, request.error, request.probability);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/cms/:key/incrby
///
/// Increment item counts in a Count-Min Sketch (CMS.INCRBY)
#[utoipa::path(
    post,
    path = "/api/v1/cms/{key}/incrby",
    params(
        ("key" = String, Path, description = "Count-Min Sketch key")
    ),
    request_body = CmsIncrByRequest,
    responses(
        (status = 200, description = "Items incremented", body = CmsIncrByResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Count-Min Sketch"
)]
async fn cms_incr_by(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CmsIncrByRequest>,
) -> Result<Json<ApiResponse<CmsIncrByResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let items: Vec<(String, u64)> = request
        .items
        .into_iter()
        .map(|i| (i.item, i.increment))
        .collect();

    let result = state.probabilistic_service.cms_incr_by(&key, items).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/cms/:key/query
///
/// Query item counts in a Count-Min Sketch (CMS.QUERY)
#[utoipa::path(
    post,
    path = "/api/v1/cms/{key}/query",
    params(
        ("key" = String, Path, description = "Count-Min Sketch key")
    ),
    request_body = CmsQueryRequest,
    responses(
        (status = 200, description = "Query results", body = CmsQueryResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Count-Min Sketch"
)]
async fn cms_query(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CmsQueryRequest>,
) -> Result<Json<ApiResponse<CmsQueryResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let future = state.probabilistic_service.cms_query(&key, request.items);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/cms/:key/merge
///
/// Merge multiple Count-Min Sketches (CMS.MERGE)
#[utoipa::path(
    post,
    path = "/api/v1/cms/{key}/merge",
    params(
        ("key" = String, Path, description = "Destination key")
    ),
    request_body = CmsMergeRequest,
    responses(
        (status = 200, description = "Merge successful", body = CmsMergeResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Count-Min Sketch"
)]
async fn cms_merge(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CmsMergeRequest>,
) -> Result<Json<ApiResponse<CmsMergeResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let future = state
        .probabilistic_service
        .cms_merge(&key, request.sources, request.weights);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/cms/:key
///
/// Get information about a Count-Min Sketch (CMS.INFO)
#[utoipa::path(
    get,
    path = "/api/v1/cms/{key}",
    params(
        ("key" = String, Path, description = "Count-Min Sketch key")
    ),
    responses(
        (status = 200, description = "CMS info", body = CmsInfoResponse),
        (status = 404, description = "CMS not found"),
        (status = 500, description = "Server error")
    ),
    tag = "Count-Min Sketch"
)]
async fn cms_info(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<CmsInfoResponse>>, CacheError> {
    let result = state.probabilistic_service.cms_info(&key).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

// ==================== Top-K Handlers ====================

/// POST /api/v1/topk/:key
///
/// Reserve a Top-K filter (TOPK.RESERVE)
#[utoipa::path(
    post,
    path = "/api/v1/topk/{key}",
    params(
        ("key" = String, Path, description = "Top-K key")
    ),
    request_body = TopKReserveRequest,
    responses(
        (status = 200, description = "Top-K created", body = TopKReserveResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Top-K"
)]
async fn topk_reserve(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TopKReserveRequest>,
) -> Result<Json<ApiResponse<TopKReserveResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let future = state.probabilistic_service.topk_reserve(
        &key,
        request.k,
        request.width,
        request.depth,
        request.decay,
    );
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/topk/:key
///
/// Get information about a Top-K filter (TOPK.INFO)
#[utoipa::path(
    get,
    path = "/api/v1/topk/{key}",
    params(
        ("key" = String, Path, description = "Top-K key")
    ),
    responses(
        (status = 200, description = "Top-K info", body = TopKInfoResponse),
        (status = 404, description = "Top-K not found"),
        (status = 500, description = "Server error")
    ),
    tag = "Top-K"
)]
async fn topk_info(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<TopKInfoResponse>>, CacheError> {
    let result = state.probabilistic_service.topk_info(&key).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/topk/:key/add
///
/// Add items to a Top-K filter (TOPK.ADD)
#[utoipa::path(
    post,
    path = "/api/v1/topk/{key}/add",
    params(
        ("key" = String, Path, description = "Top-K key")
    ),
    request_body = TopKAddRequest,
    responses(
        (status = 200, description = "Items added", body = TopKAddResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Top-K"
)]
async fn topk_add(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TopKAddRequest>,
) -> Result<Json<ApiResponse<TopKAddResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let future = state.probabilistic_service.topk_add(&key, request.items);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/topk/:key/incrby
///
/// Increment item counts in a Top-K filter (TOPK.INCRBY)
#[utoipa::path(
    post,
    path = "/api/v1/topk/{key}/incrby",
    params(
        ("key" = String, Path, description = "Top-K key")
    ),
    request_body = TopKIncrByRequest,
    responses(
        (status = 200, description = "Items incremented", body = TopKIncrByResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Top-K"
)]
async fn topk_incr_by(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TopKIncrByRequest>,
) -> Result<Json<ApiResponse<TopKIncrByResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let items: Vec<(String, u64)> = request
        .items
        .into_iter()
        .map(|i| (i.item, i.increment))
        .collect();

    let future = state.probabilistic_service.topk_incr_by(&key, items);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/topk/:key/query
///
/// Query if items are in the Top-K (TOPK.QUERY)
#[utoipa::path(
    post,
    path = "/api/v1/topk/{key}/query",
    params(
        ("key" = String, Path, description = "Top-K key")
    ),
    request_body = TopKQueryRequest,
    responses(
        (status = 200, description = "Query results", body = TopKQueryResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Top-K"
)]
async fn topk_query(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TopKQueryRequest>,
) -> Result<Json<ApiResponse<TopKQueryResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let future = state.probabilistic_service.topk_query(&key, request.items);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/topk/:key/count
///
/// Get counts of items in a Top-K filter (TOPK.COUNT)
#[utoipa::path(
    post,
    path = "/api/v1/topk/{key}/count",
    params(
        ("key" = String, Path, description = "Top-K key")
    ),
    request_body = TopKQueryRequest,
    responses(
        (status = 200, description = "Count results", body = TopKCountResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "Top-K"
)]
async fn topk_count(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TopKQueryRequest>,
) -> Result<Json<ApiResponse<TopKCountResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let future = state.probabilistic_service.topk_count(&key, request.items);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/topk/:key/list
///
/// List items in a Top-K filter (TOPK.LIST)
#[utoipa::path(
    get,
    path = "/api/v1/topk/{key}/list",
    params(
        ("key" = String, Path, description = "Top-K key"),
        ("with_count" = Option<bool>, Query, description = "Include counts in response")
    ),
    responses(
        (status = 200, description = "Top-K items", body = TopKListResponse),
        (status = 404, description = "Top-K not found"),
        (status = 500, description = "Server error")
    ),
    tag = "Top-K"
)]
async fn topk_list(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<TopKListQuery>,
) -> Result<Json<ApiResponse<TopKListResponse>>, CacheError> {
    let result = state
        .probabilistic_service
        .topk_list(&key, query.with_count)
        .await?;

    Ok(Json(ApiResponse::new(result.into())))
}

// ==================== HyperLogLog Handlers ====================

/// POST /api/v1/hll/:key/add
///
/// Add elements to a HyperLogLog (PFADD)
#[utoipa::path(
    post,
    path = "/api/v1/hll/{key}/add",
    params(
        ("key" = String, Path, description = "HyperLogLog key")
    ),
    request_body = PfAddRequest,
    responses(
        (status = 200, description = "Elements added", body = PfAddResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "HyperLogLog"
)]
async fn pf_add(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<PfAddRequest>,
) -> Result<Json<ApiResponse<PfAddResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let future = state.probabilistic_service.pf_add(&key, request.elements);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/hll/count
///
/// Count unique elements in HyperLogLog(s) (PFCOUNT)
#[utoipa::path(
    post,
    path = "/api/v1/hll/count",
    request_body = PfCountRequest,
    responses(
        (status = 200, description = "Count result", body = PfCountResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "HyperLogLog"
)]
async fn pf_count(
    State(state): State<AppState>,
    Json(request): Json<PfCountRequest>,
) -> Result<Json<ApiResponse<PfCountResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.probabilistic_service.pf_count(request.keys).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/hll/:key/merge
///
/// Merge multiple HyperLogLogs (PFMERGE)
#[utoipa::path(
    post,
    path = "/api/v1/hll/{key}/merge",
    params(
        ("key" = String, Path, description = "Destination HyperLogLog key")
    ),
    request_body = PfMergeRequest,
    responses(
        (status = 200, description = "Merge successful", body = PfMergeResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Server error")
    ),
    tag = "HyperLogLog"
)]
async fn pf_merge(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<PfMergeRequest>,
) -> Result<Json<ApiResponse<PfMergeResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let future = state.probabilistic_service.pf_merge(&key, request.sources);
    let result = future.await?;

    Ok(Json(ApiResponse::new(result.into())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state_with_probabilistic_repo;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[test]
    fn test_cms_routes_structure() {
        // Verify routes are created without panic
        let _routes = cms_routes();
    }

    #[test]
    fn test_topk_routes_structure() {
        let _routes = topk_routes();
    }

    #[test]
    fn test_hyperloglog_routes_structure() {
        let _routes = hyperloglog_routes();
    }

    #[tokio::test]
    async fn test_probabilistic_routes() {
        let (state, _) = test_state_with_probabilistic_repo();
        let app = Router::new()
            .merge(cms_routes())
            .merge(topk_routes())
            .merge(hyperloglog_routes())
            .with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cms/cms-test/initbydim")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"width":2000,"depth":5}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cms/cms-test/initbyprob")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"error":0.01,"probability":0.001}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cms/cms-test/incrby")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"items":[{"item":"a","increment":2},{"item":"b","increment":3}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cms/cms-test/query")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["a","b"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cms/cms-dest/merge")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"sources":["cms-src1","cms-src2"],"weights":[1,2]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/cms/cms-test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/topk/topk-test")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"k":10}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/topk/topk-test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/topk/topk-test/add")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["a","b"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/topk/topk-test/incrby")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":[{"item":"a","increment":2}]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/topk/topk-test/query")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["a","b"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/topk/topk-test/count")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"items":["a","b"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/topk/topk-test/list?with_count=true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/hll/hll-test/add")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"elements":["a","b"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/hll/count")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"keys":["hll-test"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/hll/hll-test/merge")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"sources":["hll-src1","hll-src2"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
