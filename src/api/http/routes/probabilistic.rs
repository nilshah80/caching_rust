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
    PfMergeResponse, TDigestAckResponse, TDigestAddRequest, TDigestCreateRequest,
    TDigestInfoResponse, TDigestMergeRequest, TDigestQuantileRequest, TDigestRanksRequest,
    TDigestRanksResponse, TDigestScalarResponse, TDigestTrimmedMeanRequest, TDigestValuesRequest,
    TDigestValuesResponse, TopKAddRequest, TopKAddResponse, TopKCountResponse, TopKIncrByRequest,
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

/// Build T-Digest routes
pub fn tdigest_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/tdigest/{key}", post(tdigest_create))
        .route("/api/v1/tdigest/{key}", get(tdigest_info))
        .route("/api/v1/tdigest/{key}/add", post(tdigest_add))
        .route("/api/v1/tdigest/{key}/quantile", post(tdigest_quantile))
        .route("/api/v1/tdigest/{key}/cdf", post(tdigest_cdf))
        .route("/api/v1/tdigest/{key}/rank", post(tdigest_rank))
        .route("/api/v1/tdigest/{key}/revrank", post(tdigest_revrank))
        .route("/api/v1/tdigest/{key}/byrank", post(tdigest_byrank))
        .route("/api/v1/tdigest/{key}/byrevrank", post(tdigest_byrevrank))
        .route("/api/v1/tdigest/{key}/min", get(tdigest_min))
        .route("/api/v1/tdigest/{key}/max", get(tdigest_max))
        .route("/api/v1/tdigest/{key}/merge", post(tdigest_merge))
        .route("/api/v1/tdigest/{key}/reset", post(tdigest_reset))
        .route(
            "/api/v1/tdigest/{key}/trimmed_mean",
            post(tdigest_trimmed_mean),
        )
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

// ==================== T-Digest Handlers ====================

/// POST /api/v1/tdigest/:key
///
/// Create a t-digest sketch (TDIGEST.CREATE)
#[utoipa::path(
    post,
    path = "/api/v1/tdigest/{key}",
    params(("key" = String, Path, description = "T-Digest key")),
    request_body = TDigestCreateRequest,
    responses(
        (status = 200, description = "Sketch created", body = TDigestAckResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_create(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TDigestCreateRequest>,
) -> Result<Json<ApiResponse<TDigestAckResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state
        .probabilistic_service
        .tdigest_create(&key, request.compression)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/tdigest/:key
///
/// Get t-digest metadata (TDIGEST.INFO)
#[utoipa::path(
    get,
    path = "/api/v1/tdigest/{key}",
    params(("key" = String, Path, description = "T-Digest key")),
    responses(
        (status = 200, description = "T-Digest info", body = TDigestInfoResponse),
        (status = 404, description = "Sketch not found"),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_info(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<TDigestInfoResponse>>, CacheError> {
    let result = state.probabilistic_service.tdigest_info(&key).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/tdigest/:key/add
///
/// Add observations (TDIGEST.ADD)
#[utoipa::path(
    post,
    path = "/api/v1/tdigest/{key}/add",
    params(("key" = String, Path, description = "T-Digest key")),
    request_body = TDigestAddRequest,
    responses(
        (status = 200, description = "Values added", body = TDigestAckResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_add(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TDigestAddRequest>,
) -> Result<Json<ApiResponse<TDigestAckResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state
        .probabilistic_service
        .tdigest_add(&key, request.values)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/tdigest/:key/quantile
///
/// Estimate quantiles (TDIGEST.QUANTILE)
#[utoipa::path(
    post,
    path = "/api/v1/tdigest/{key}/quantile",
    params(("key" = String, Path, description = "T-Digest key")),
    request_body = TDigestQuantileRequest,
    responses(
        (status = 200, description = "Quantile estimates", body = TDigestValuesResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_quantile(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TDigestQuantileRequest>,
) -> Result<Json<ApiResponse<TDigestValuesResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state
        .probabilistic_service
        .tdigest_quantile(&key, request.quantiles)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/tdigest/:key/cdf
///
/// Estimate CDF (TDIGEST.CDF)
#[utoipa::path(
    post,
    path = "/api/v1/tdigest/{key}/cdf",
    params(("key" = String, Path, description = "T-Digest key")),
    request_body = TDigestValuesRequest,
    responses(
        (status = 200, description = "CDF estimates", body = TDigestValuesResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_cdf(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TDigestValuesRequest>,
) -> Result<Json<ApiResponse<TDigestValuesResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state
        .probabilistic_service
        .tdigest_cdf(&key, request.values)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/tdigest/:key/rank
///
/// Estimate ranks (TDIGEST.RANK)
#[utoipa::path(
    post,
    path = "/api/v1/tdigest/{key}/rank",
    params(("key" = String, Path, description = "T-Digest key")),
    request_body = TDigestValuesRequest,
    responses(
        (status = 200, description = "Rank estimates", body = TDigestRanksResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_rank(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TDigestValuesRequest>,
) -> Result<Json<ApiResponse<TDigestRanksResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state
        .probabilistic_service
        .tdigest_rank(&key, request.values)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/tdigest/:key/revrank
///
/// Estimate reverse ranks (TDIGEST.REVRANK)
#[utoipa::path(
    post,
    path = "/api/v1/tdigest/{key}/revrank",
    params(("key" = String, Path, description = "T-Digest key")),
    request_body = TDigestValuesRequest,
    responses(
        (status = 200, description = "Reverse rank estimates", body = TDigestRanksResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_revrank(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TDigestValuesRequest>,
) -> Result<Json<ApiResponse<TDigestRanksResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state
        .probabilistic_service
        .tdigest_revrank(&key, request.values)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/tdigest/:key/byrank
///
/// Lookup values by rank (TDIGEST.BYRANK)
#[utoipa::path(
    post,
    path = "/api/v1/tdigest/{key}/byrank",
    params(("key" = String, Path, description = "T-Digest key")),
    request_body = TDigestRanksRequest,
    responses(
        (status = 200, description = "Values at ranks", body = TDigestValuesResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_byrank(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TDigestRanksRequest>,
) -> Result<Json<ApiResponse<TDigestValuesResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state
        .probabilistic_service
        .tdigest_byrank(&key, request.ranks)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/tdigest/:key/byrevrank
///
/// Lookup values by reverse rank (TDIGEST.BYREVRANK)
#[utoipa::path(
    post,
    path = "/api/v1/tdigest/{key}/byrevrank",
    params(("key" = String, Path, description = "T-Digest key")),
    request_body = TDigestRanksRequest,
    responses(
        (status = 200, description = "Values at reverse ranks", body = TDigestValuesResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_byrevrank(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TDigestRanksRequest>,
) -> Result<Json<ApiResponse<TDigestValuesResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state
        .probabilistic_service
        .tdigest_byrevrank(&key, request.ranks)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/tdigest/:key/min
///
/// Get the smallest observation (TDIGEST.MIN)
#[utoipa::path(
    get,
    path = "/api/v1/tdigest/{key}/min",
    params(("key" = String, Path, description = "T-Digest key")),
    responses(
        (status = 200, description = "Minimum value", body = TDigestScalarResponse),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_min(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<TDigestScalarResponse>>, CacheError> {
    let result = state.probabilistic_service.tdigest_min(&key).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/tdigest/:key/max
///
/// Get the largest observation (TDIGEST.MAX)
#[utoipa::path(
    get,
    path = "/api/v1/tdigest/{key}/max",
    params(("key" = String, Path, description = "T-Digest key")),
    responses(
        (status = 200, description = "Maximum value", body = TDigestScalarResponse),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_max(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<TDigestScalarResponse>>, CacheError> {
    let result = state.probabilistic_service.tdigest_max(&key).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/tdigest/:key/merge
///
/// Merge sketches into `key` (TDIGEST.MERGE)
#[utoipa::path(
    post,
    path = "/api/v1/tdigest/{key}/merge",
    params(("key" = String, Path, description = "Destination T-Digest key")),
    request_body = TDigestMergeRequest,
    responses(
        (status = 200, description = "Merge successful", body = TDigestAckResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_merge(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TDigestMergeRequest>,
) -> Result<Json<ApiResponse<TDigestAckResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state
        .probabilistic_service
        .tdigest_merge(
            &key,
            request.sources,
            request.compression,
            request.override_existing,
        )
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/tdigest/:key/reset
///
/// Reset a sketch (TDIGEST.RESET)
#[utoipa::path(
    post,
    path = "/api/v1/tdigest/{key}/reset",
    params(("key" = String, Path, description = "T-Digest key")),
    responses(
        (status = 200, description = "Sketch reset", body = TDigestAckResponse),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_reset(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<TDigestAckResponse>>, CacheError> {
    let result = state.probabilistic_service.tdigest_reset(&key).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/tdigest/:key/trimmed_mean
///
/// Compute a trimmed mean between two quantiles (TDIGEST.TRIMMED_MEAN)
#[utoipa::path(
    post,
    path = "/api/v1/tdigest/{key}/trimmed_mean",
    params(("key" = String, Path, description = "T-Digest key")),
    request_body = TDigestTrimmedMeanRequest,
    responses(
        (status = 200, description = "Trimmed mean", body = TDigestScalarResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisBloom module not available"),
        (status = 500, description = "Server error")
    ),
    tag = "T-Digest"
)]
pub async fn tdigest_trimmed_mean(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TDigestTrimmedMeanRequest>,
) -> Result<Json<ApiResponse<TDigestScalarResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state
        .probabilistic_service
        .tdigest_trimmed_mean(&key, request.low_cut_quantile, request.high_cut_quantile)
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

    #[test]
    fn test_tdigest_routes_structure() {
        let _routes = tdigest_routes();
    }

    #[tokio::test]
    async fn test_tdigest_routes_smoke() {
        let (state, _) = test_state_with_probabilistic_repo();
        let app = Router::new().merge(tdigest_routes()).with_state(state);

        // CREATE
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tdigest/td-test")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"compression":100}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // CREATE with empty body (compression optional)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tdigest/td-test")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // ADD
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tdigest/td-test/add")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"values":[1.0,2.5,3.75]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // ADD rejects empty values
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tdigest/td-test/add")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"values":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // QUANTILE
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tdigest/td-test/quantile")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"quantiles":[0.5,0.9]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // QUANTILE rejects out-of-range
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tdigest/td-test/quantile")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"quantiles":[1.5]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // CDF / RANK / REVRANK
        for path in ["cdf", "rank", "revrank"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/v1/tdigest/td-test/{path}"))
                        .header("Content-Type", "application/json")
                        .body(Body::from(r#"{"values":[1.0,2.0]}"#))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
        }

        // BYRANK / BYREVRANK
        for path in ["byrank", "byrevrank"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/v1/tdigest/td-test/{path}"))
                        .header("Content-Type", "application/json")
                        .body(Body::from(r#"{"ranks":[0,1,2]}"#))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
        }

        // MIN / MAX / INFO / RESET
        for (method, path) in [
            ("GET", "/api/v1/tdigest/td-test/min"),
            ("GET", "/api/v1/tdigest/td-test/max"),
            ("GET", "/api/v1/tdigest/td-test"),
            ("POST", "/api/v1/tdigest/td-test/reset"),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{method} {path}");
        }

        // MERGE
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tdigest/td-dest/merge")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"sources":["td-a","td-b"],"compression":100,"override_existing":true}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // TRIMMED_MEAN
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tdigest/td-test/trimmed_mean")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"low_cut_quantile":0.1,"high_cut_quantile":0.9}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // TRIMMED_MEAN rejects out-of-range
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tdigest/td-test/trimmed_mean")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"low_cut_quantile":-0.1,"high_cut_quantile":0.9}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
