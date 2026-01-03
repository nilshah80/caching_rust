//! Sorted Set Routes
//!
//! HTTP endpoints for Redis sorted set (ZSET) operations.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};

use crate::api::http::schemas::sorted_sets::{
    LexRangeDto, ScoreRangeDto, ScoredMemberDto, ZAddIncrRequest, ZAddIncrResponse,
    ZAddOptionsDto, ZAddRequest, ZAddResponse, ZBMPopRequest, ZBPopRequest, ZBPopResponse,
    ZCardResponse, ZCountRequest, ZCountResponse, ZDiffRequest, ZDiffStoreRequest,
    ZIncrByRequest, ZIncrByResponse, ZInterCardRequest, ZInterCardResponse, ZLexCountRequest,
    ZMPopRequest, ZMPopResponse, ZMScoreRequest, ZMScoreResponse, ZPopQuery, ZPopResponse,
    ZRandMemberQuery, ZRandMemberResponse, ZRangeByLexRequest, ZRangeByLexResponse,
    ZRangeByScoreRequest, ZRangeQuery, ZRangeResponse, ZRangeStoreRequest, ZRangeStoreResponse,
    ZRankResponse, ZRemRangeByLexRequest, ZRemRangeByRankRequest, ZRemRangeByScoreRequest,
    ZRemRangeResponse, ZRemRequest, ZRemResponse, ZScanQuery, ZScanResponse, ZScoreResponse,
    ZSetAlgebraOptionsDto, ZSetAlgebraRequest, ZSetAlgebraResponse, ZSetAlgebraStoreRequest,
    ZSetAlgebraStoreResponse, ZAggregateDto,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    LexRange, ScoreRange, ScoredMember, ZAddOptions, ZAggregate, ZPopDirection,
    ZRangeOptions, ZSetAlgebraOptions,
};
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create Sorted Set routes
pub fn sorted_set_routes() -> Router<AppState> {
    Router::new()
        // Basic operations
        .route("/api/v1/sorted-sets/{key}/members", post(zadd))
        .route("/api/v1/sorted-sets/{key}/members", delete(zrem))
        .route("/api/v1/sorted-sets/{key}/incr", post(zadd_incr))
        .route("/api/v1/sorted-sets/{key}/score/{member}", get(zscore))
        .route("/api/v1/sorted-sets/{key}/mscore", post(zmscore))
        .route("/api/v1/sorted-sets/{key}/incrby", post(zincrby))
        .route("/api/v1/sorted-sets/{key}/card", get(zcard))
        .route("/api/v1/sorted-sets/{key}/count", post(zcount))
        .route("/api/v1/sorted-sets/{key}/lexcount", post(zlexcount))
        // Rank operations
        .route("/api/v1/sorted-sets/{key}/rank/{member}", get(zrank))
        .route("/api/v1/sorted-sets/{key}/revrank/{member}", get(zrevrank))
        // Range operations
        .route("/api/v1/sorted-sets/{key}/range", get(zrange))
        .route("/api/v1/sorted-sets/{key}/rangebyscore", post(zrangebyscore))
        .route("/api/v1/sorted-sets/{key}/rangebylex", post(zrangebylex))
        .route("/api/v1/sorted-sets/{key}/rangestore", post(zrangestore))
        // Remove range operations
        .route("/api/v1/sorted-sets/{key}/remrangebyrank", post(zremrangebyrank))
        .route("/api/v1/sorted-sets/{key}/remrangebyscore", post(zremrangebyscore))
        .route("/api/v1/sorted-sets/{key}/remrangebylex", post(zremrangebylex))
        // Pop operations
        .route("/api/v1/sorted-sets/{key}/popmin", post(zpopmin))
        .route("/api/v1/sorted-sets/{key}/popmax", post(zpopmax))
        .route("/api/v1/sorted-sets/bzpopmin", post(bzpopmin))
        .route("/api/v1/sorted-sets/bzpopmax", post(bzpopmax))
        .route("/api/v1/sorted-sets/zmpop", post(zmpop))
        .route("/api/v1/sorted-sets/bzmpop", post(bzmpop))
        // Random access
        .route("/api/v1/sorted-sets/{key}/random", get(zrandmember))
        // Set algebra operations
        .route("/api/v1/sorted-sets/union", post(zunion))
        .route("/api/v1/sorted-sets/unionstore", post(zunionstore))
        .route("/api/v1/sorted-sets/inter", post(zinter))
        .route("/api/v1/sorted-sets/interstore", post(zinterstore))
        .route("/api/v1/sorted-sets/intercard", post(zintercard))
        .route("/api/v1/sorted-sets/diff", post(zdiff))
        .route("/api/v1/sorted-sets/diffstore", post(zdiffstore))
        // Scan operation
        .route("/api/v1/sorted-sets/{key}/scan", get(zscan))
}

// ========== Helper functions ==========

fn convert_scored_member(m: ScoredMember) -> ScoredMemberDto {
    ScoredMemberDto::new(m.member, m.score)
}

fn convert_to_scored_member(m: &ScoredMemberDto) -> ScoredMember {
    ScoredMember::new(m.member.clone(), m.score)
}

fn convert_zadd_options(opts: Option<ZAddOptionsDto>) -> Option<ZAddOptions> {
    opts.map(|o| ZAddOptions {
        nx: o.nx,
        xx: o.xx,
        gt: o.gt,
        lt: o.lt,
        ch: o.ch,
    })
}

fn parse_score(s: &str) -> f64 {
    match s.trim().to_lowercase().as_str() {
        "-inf" => f64::NEG_INFINITY,
        "+inf" | "inf" => f64::INFINITY,
        _ => s.parse().unwrap_or(0.0),
    }
}

fn convert_score_range(range: &ScoreRangeDto) -> ScoreRange {
    let min_str = range.min.trim();
    let max_str = range.max.trim();

    let (min, min_exclusive) = if min_str.starts_with('(') {
        (parse_score(&min_str[1..]), true)
    } else {
        (parse_score(min_str), false)
    };

    let (max, max_exclusive) = if max_str.starts_with('(') {
        (parse_score(&max_str[1..]), true)
    } else {
        (parse_score(max_str), false)
    };

    ScoreRange {
        min,
        max,
        min_exclusive,
        max_exclusive,
    }
}

fn convert_lex_range(range: &LexRangeDto) -> LexRange {
    LexRange {
        min: range.min.clone(),
        max: range.max.clone(),
    }
}

fn convert_aggregate(agg: &ZAggregateDto) -> ZAggregate {
    match agg {
        ZAggregateDto::Sum => ZAggregate::Sum,
        ZAggregateDto::Min => ZAggregate::Min,
        ZAggregateDto::Max => ZAggregate::Max,
    }
}

fn convert_algebra_options(opts: Option<ZSetAlgebraOptionsDto>) -> Option<ZSetAlgebraOptions> {
    opts.map(|o| ZSetAlgebraOptions {
        weights: o.weights,
        aggregate: convert_aggregate(&o.aggregate),
        with_scores: o.with_scores,
    })
}

// ========== Basic operations ==========

/// ZADD - Add members with scores to a sorted set
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/members",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZAddRequest,
    responses(
        (status = 200, description = "Members added successfully", body = ZAddResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn zadd(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZAddRequest>,
) -> Result<Json<ApiResponse<ZAddResponse>>, CacheError> {
    let members: Vec<ScoredMember> = request
        .members
        .iter()
        .map(convert_to_scored_member)
        .collect();
    let options = convert_zadd_options(request.options);
    let result = state.sorted_set_service.zadd(&key, members, options).await?;
    Ok(Json(ApiResponse::success(ZAddResponse {
        count: result.count,
        new_score: result.new_score,
    })))
}

/// ZADD with INCR - Increment the score of a member
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/incr",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZAddIncrRequest,
    responses(
        (status = 200, description = "Score incremented successfully", body = ZAddIncrResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn zadd_incr(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZAddIncrRequest>,
) -> Result<Json<ApiResponse<ZAddIncrResponse>>, CacheError> {
    let options = convert_zadd_options(request.options);
    let new_score = state
        .sorted_set_service
        .zadd_incr(&key, &request.member, request.score, options)
        .await?;
    Ok(Json(ApiResponse::success(ZAddIncrResponse { new_score })))
}

/// ZREM - Remove members from a sorted set
#[utoipa::path(
    delete,
    path = "/api/v1/sorted-sets/{key}/members",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZRemRequest,
    responses(
        (status = 200, description = "Members removed successfully", body = ZRemResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn zrem(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZRemRequest>,
) -> Result<Json<ApiResponse<ZRemResponse>>, CacheError> {
    let removed = state.sorted_set_service.zrem(&key, request.members).await?;
    Ok(Json(ApiResponse::success(ZRemResponse { removed })))
}

/// ZSCORE - Get the score of a member
#[utoipa::path(
    get,
    path = "/api/v1/sorted-sets/{key}/score/{member}",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key"),
        ("member" = String, Path, description = "The member to get score for")
    ),
    responses(
        (status = 200, description = "Member score", body = ZScoreResponse)
    )
)]
pub async fn zscore(
    State(state): State<AppState>,
    Path((key, member)): Path<(String, String)>,
) -> Result<Json<ApiResponse<ZScoreResponse>>, CacheError> {
    let score = state.sorted_set_service.zscore(&key, &member).await?;
    Ok(Json(ApiResponse::success(ZScoreResponse { score })))
}

/// ZMSCORE - Get scores of multiple members
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/mscore",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZMScoreRequest,
    responses(
        (status = 200, description = "Member scores", body = ZMScoreResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn zmscore(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZMScoreRequest>,
) -> Result<Json<ApiResponse<ZMScoreResponse>>, CacheError> {
    let scores = state.sorted_set_service.zmscore(&key, request.members).await?;
    Ok(Json(ApiResponse::success(ZMScoreResponse { scores })))
}

/// ZINCRBY - Increment the score of a member
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/incrby",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZIncrByRequest,
    responses(
        (status = 200, description = "Score incremented successfully", body = ZIncrByResponse)
    )
)]
pub async fn zincrby(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZIncrByRequest>,
) -> Result<Json<ApiResponse<ZIncrByResponse>>, CacheError> {
    let new_score = state
        .sorted_set_service
        .zincrby(&key, &request.member, request.increment)
        .await?;
    Ok(Json(ApiResponse::success(ZIncrByResponse { new_score })))
}

/// ZCARD - Get the number of members in a sorted set
#[utoipa::path(
    get,
    path = "/api/v1/sorted-sets/{key}/card",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    responses(
        (status = 200, description = "Cardinality of the sorted set", body = ZCardResponse)
    )
)]
pub async fn zcard(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<ZCardResponse>>, CacheError> {
    let cardinality = state.sorted_set_service.zcard(&key).await?;
    Ok(Json(ApiResponse::success(ZCardResponse { cardinality })))
}

/// ZCOUNT - Count members with scores in a range
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/count",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZCountRequest,
    responses(
        (status = 200, description = "Count of members in range", body = ZCountResponse)
    )
)]
pub async fn zcount(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZCountRequest>,
) -> Result<Json<ApiResponse<ZCountResponse>>, CacheError> {
    let range = convert_score_range(&request.range);
    let count = state.sorted_set_service.zcount(&key, range).await?;
    Ok(Json(ApiResponse::success(ZCountResponse { count })))
}

/// ZLEXCOUNT - Count members in a lexicographical range
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/lexcount",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZLexCountRequest,
    responses(
        (status = 200, description = "Count of members in lex range", body = ZCountResponse)
    )
)]
pub async fn zlexcount(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZLexCountRequest>,
) -> Result<Json<ApiResponse<ZCountResponse>>, CacheError> {
    let range = convert_lex_range(&request.range);
    let count = state.sorted_set_service.zlexcount(&key, range).await?;
    Ok(Json(ApiResponse::success(ZCountResponse { count })))
}

// ========== Rank operations ==========

/// ZRANK - Get the rank of a member
#[utoipa::path(
    get,
    path = "/api/v1/sorted-sets/{key}/rank/{member}",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key"),
        ("member" = String, Path, description = "The member to get rank for")
    ),
    responses(
        (status = 200, description = "Member rank", body = ZRankResponse)
    )
)]
pub async fn zrank(
    State(state): State<AppState>,
    Path((key, member)): Path<(String, String)>,
) -> Result<Json<ApiResponse<ZRankResponse>>, CacheError> {
    let rank = state.sorted_set_service.zrank(&key, &member).await?;
    Ok(Json(ApiResponse::success(ZRankResponse { rank })))
}

/// ZREVRANK - Get the reverse rank of a member
#[utoipa::path(
    get,
    path = "/api/v1/sorted-sets/{key}/revrank/{member}",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key"),
        ("member" = String, Path, description = "The member to get reverse rank for")
    ),
    responses(
        (status = 200, description = "Member reverse rank", body = ZRankResponse)
    )
)]
pub async fn zrevrank(
    State(state): State<AppState>,
    Path((key, member)): Path<(String, String)>,
) -> Result<Json<ApiResponse<ZRankResponse>>, CacheError> {
    let rank = state.sorted_set_service.zrevrank(&key, &member).await?;
    Ok(Json(ApiResponse::success(ZRankResponse { rank })))
}

// ========== Range operations ==========

/// ZRANGE - Get members in a range by index
#[utoipa::path(
    get,
    path = "/api/v1/sorted-sets/{key}/range",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key"),
        ("start" = i64, Query, description = "Start index"),
        ("stop" = i64, Query, description = "Stop index"),
        ("with_scores" = Option<bool>, Query, description = "Include scores in response"),
        ("rev" = Option<bool>, Query, description = "Reverse order")
    ),
    responses(
        (status = 200, description = "Members in range", body = ZRangeResponse)
    )
)]
pub async fn zrange(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<ZRangeQuery>,
) -> Result<Json<ApiResponse<ZRangeResponse>>, CacheError> {
    let options = Some(ZRangeOptions {
        with_scores: query.with_scores,
        rev: query.rev,
        offset: None,
        count: None,
    });
    let members = state
        .sorted_set_service
        .zrange(&key, query.start, query.stop, options)
        .await?;
    let members = members.into_iter().map(convert_scored_member).collect();
    Ok(Json(ApiResponse::success(ZRangeResponse { members })))
}

/// ZRANGEBYSCORE - Get members with scores in a range
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/rangebyscore",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZRangeByScoreRequest,
    responses(
        (status = 200, description = "Members in score range", body = ZRangeResponse)
    )
)]
pub async fn zrangebyscore(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZRangeByScoreRequest>,
) -> Result<Json<ApiResponse<ZRangeResponse>>, CacheError> {
    let range = convert_score_range(&request.range);
    let options = Some(ZRangeOptions {
        with_scores: request.with_scores,
        rev: request.rev,
        offset: request.offset,
        count: request.count,
    });
    let members = state
        .sorted_set_service
        .zrangebyscore(&key, range, options)
        .await?;
    let members = members.into_iter().map(convert_scored_member).collect();
    Ok(Json(ApiResponse::success(ZRangeResponse { members })))
}

/// ZRANGEBYLEX - Get members in a lexicographical range
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/rangebylex",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZRangeByLexRequest,
    responses(
        (status = 200, description = "Members in lex range", body = ZRangeByLexResponse)
    )
)]
pub async fn zrangebylex(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZRangeByLexRequest>,
) -> Result<Json<ApiResponse<ZRangeByLexResponse>>, CacheError> {
    let range = convert_lex_range(&request.range);
    let options = Some(ZRangeOptions {
        with_scores: false,
        rev: request.rev,
        offset: request.offset,
        count: request.count,
    });
    let members = state
        .sorted_set_service
        .zrangebylex(&key, range, options)
        .await?;
    Ok(Json(ApiResponse::success(ZRangeByLexResponse { members })))
}

/// ZRANGESTORE - Store a range in a new key
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/rangestore",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The source sorted set key")
    ),
    request_body = ZRangeStoreRequest,
    responses(
        (status = 200, description = "Range stored successfully", body = ZRangeStoreResponse)
    )
)]
pub async fn zrangestore(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZRangeStoreRequest>,
) -> Result<Json<ApiResponse<ZRangeStoreResponse>>, CacheError> {
    let options = Some(ZRangeOptions {
        with_scores: request.with_scores,
        rev: request.rev,
        offset: None,
        count: None,
    });
    let count = state
        .sorted_set_service
        .zrangestore(&request.destination, &key, request.start, request.stop, options)
        .await?;
    Ok(Json(ApiResponse::success(ZRangeStoreResponse { count })))
}

// ========== Remove range operations ==========

/// ZREMRANGEBYRANK - Remove members by rank range
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/remrangebyrank",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZRemRangeByRankRequest,
    responses(
        (status = 200, description = "Members removed successfully", body = ZRemRangeResponse)
    )
)]
pub async fn zremrangebyrank(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZRemRangeByRankRequest>,
) -> Result<Json<ApiResponse<ZRemRangeResponse>>, CacheError> {
    let removed = state
        .sorted_set_service
        .zremrangebyrank(&key, request.start, request.stop)
        .await?;
    Ok(Json(ApiResponse::success(ZRemRangeResponse { removed })))
}

/// ZREMRANGEBYSCORE - Remove members by score range
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/remrangebyscore",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZRemRangeByScoreRequest,
    responses(
        (status = 200, description = "Members removed successfully", body = ZRemRangeResponse)
    )
)]
pub async fn zremrangebyscore(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZRemRangeByScoreRequest>,
) -> Result<Json<ApiResponse<ZRemRangeResponse>>, CacheError> {
    let range = convert_score_range(&request.range);
    let removed = state
        .sorted_set_service
        .zremrangebyscore(&key, range)
        .await?;
    Ok(Json(ApiResponse::success(ZRemRangeResponse { removed })))
}

/// ZREMRANGEBYLEX - Remove members by lexicographical range
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/remrangebylex",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key")
    ),
    request_body = ZRemRangeByLexRequest,
    responses(
        (status = 200, description = "Members removed successfully", body = ZRemRangeResponse)
    )
)]
pub async fn zremrangebylex(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ZRemRangeByLexRequest>,
) -> Result<Json<ApiResponse<ZRemRangeResponse>>, CacheError> {
    let range = convert_lex_range(&request.range);
    let removed = state
        .sorted_set_service
        .zremrangebylex(&key, range)
        .await?;
    Ok(Json(ApiResponse::success(ZRemRangeResponse { removed })))
}

// ========== Pop operations ==========

/// ZPOPMIN - Remove and return members with lowest scores
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/popmin",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key"),
        ("count" = Option<i64>, Query, description = "Number of members to pop")
    ),
    responses(
        (status = 200, description = "Popped members", body = ZPopResponse)
    )
)]
pub async fn zpopmin(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<ZPopQuery>,
) -> Result<Json<ApiResponse<ZPopResponse>>, CacheError> {
    let members = state.sorted_set_service.zpopmin(&key, query.count).await?;
    let members = members.into_iter().map(convert_scored_member).collect();
    Ok(Json(ApiResponse::success(ZPopResponse { members })))
}

/// ZPOPMAX - Remove and return members with highest scores
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/{key}/popmax",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key"),
        ("count" = Option<i64>, Query, description = "Number of members to pop")
    ),
    responses(
        (status = 200, description = "Popped members", body = ZPopResponse)
    )
)]
pub async fn zpopmax(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<ZPopQuery>,
) -> Result<Json<ApiResponse<ZPopResponse>>, CacheError> {
    let members = state.sorted_set_service.zpopmax(&key, query.count).await?;
    let members = members.into_iter().map(convert_scored_member).collect();
    Ok(Json(ApiResponse::success(ZPopResponse { members })))
}

/// BZPOPMIN - Blocking pop of member with lowest score
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/bzpopmin",
    tag = "Sorted Sets",
    request_body = ZBPopRequest,
    responses(
        (status = 200, description = "Popped member", body = ZBPopResponse)
    )
)]
pub async fn bzpopmin(
    State(state): State<AppState>,
    Json(request): Json<ZBPopRequest>,
) -> Result<Json<ApiResponse<ZBPopResponse>>, CacheError> {
    let result = state
        .sorted_set_service
        .bzpopmin(request.keys, request.timeout)
        .await?;
    let (key, members) = match result {
        Some(r) => (
            Some(r.key),
            r.members.into_iter().map(convert_scored_member).collect(),
        ),
        None => (None, Vec::new()),
    };
    Ok(Json(ApiResponse::success(ZBPopResponse { key, members })))
}

/// BZPOPMAX - Blocking pop of member with highest score
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/bzpopmax",
    tag = "Sorted Sets",
    request_body = ZBPopRequest,
    responses(
        (status = 200, description = "Popped member", body = ZBPopResponse)
    )
)]
pub async fn bzpopmax(
    State(state): State<AppState>,
    Json(request): Json<ZBPopRequest>,
) -> Result<Json<ApiResponse<ZBPopResponse>>, CacheError> {
    let result = state
        .sorted_set_service
        .bzpopmax(request.keys, request.timeout)
        .await?;
    let (key, members) = match result {
        Some(r) => (
            Some(r.key),
            r.members.into_iter().map(convert_scored_member).collect(),
        ),
        None => (None, Vec::new()),
    };
    Ok(Json(ApiResponse::success(ZBPopResponse { key, members })))
}

/// ZMPOP - Pop members from multiple keys
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/zmpop",
    tag = "Sorted Sets",
    request_body = ZMPopRequest,
    responses(
        (status = 200, description = "Popped members", body = ZMPopResponse)
    )
)]
pub async fn zmpop(
    State(state): State<AppState>,
    Json(request): Json<ZMPopRequest>,
) -> Result<Json<ApiResponse<ZMPopResponse>>, CacheError> {
    let direction = match request.direction.to_lowercase().as_str() {
        "min" => ZPopDirection::Min,
        "max" => ZPopDirection::Max,
        _ => {
            return Err(CacheError::InvalidInput(
                "Direction must be 'min' or 'max'".to_string(),
            ))
        }
    };
    let result = state.sorted_set_service.zmpop(request.keys, direction, request.count).await?;
    let (key, members) = match result {
        Some(r) => (
            Some(r.key),
            r.members.into_iter().map(convert_scored_member).collect(),
        ),
        None => (None, Vec::new()),
    };
    Ok(Json(ApiResponse::success(ZMPopResponse { key, members })))
}

/// BZMPOP - Blocking pop from multiple keys
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/bzmpop",
    tag = "Sorted Sets",
    request_body = ZBMPopRequest,
    responses(
        (status = 200, description = "Popped members", body = ZMPopResponse)
    )
)]
pub async fn bzmpop(
    State(state): State<AppState>,
    Json(request): Json<ZBMPopRequest>,
) -> Result<Json<ApiResponse<ZMPopResponse>>, CacheError> {
    let direction = match request.direction.to_lowercase().as_str() {
        "min" => ZPopDirection::Min,
        "max" => ZPopDirection::Max,
        _ => {
            return Err(CacheError::InvalidInput(
                "Direction must be 'min' or 'max'".to_string(),
            ))
        }
    };
    let result = state.sorted_set_service.bzmpop(request.keys, direction, request.timeout, request.count).await?;
    let (key, members) = match result {
        Some(r) => (
            Some(r.key),
            r.members.into_iter().map(convert_scored_member).collect(),
        ),
        None => (None, Vec::new()),
    };
    Ok(Json(ApiResponse::success(ZMPopResponse { key, members })))
}

// ========== Random access ==========

/// ZRANDMEMBER - Get random members
#[utoipa::path(
    get,
    path = "/api/v1/sorted-sets/{key}/random",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key"),
        ("count" = Option<i64>, Query, description = "Number of members to return"),
        ("with_scores" = Option<bool>, Query, description = "Include scores in response")
    ),
    responses(
        (status = 200, description = "Random members", body = ZRandMemberResponse)
    )
)]
pub async fn zrandmember(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<ZRandMemberQuery>,
) -> Result<Json<ApiResponse<ZRandMemberResponse>>, CacheError> {
    let members = state
        .sorted_set_service
        .zrandmember(&key, query.count, query.with_scores)
        .await?;
    let members = members.into_iter().map(convert_scored_member).collect();
    Ok(Json(ApiResponse::success(ZRandMemberResponse { members })))
}

// ========== Set algebra operations ==========

/// ZUNION - Get the union of multiple sorted sets
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/union",
    tag = "Sorted Sets",
    request_body = ZSetAlgebraRequest,
    responses(
        (status = 200, description = "Union result", body = ZSetAlgebraResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn zunion(
    State(state): State<AppState>,
    Json(request): Json<ZSetAlgebraRequest>,
) -> Result<Json<ApiResponse<ZSetAlgebraResponse>>, CacheError> {
    let options = convert_algebra_options(request.options);
    let members = state.sorted_set_service.zunion(request.keys, options).await?;
    let members = members.into_iter().map(convert_scored_member).collect();
    Ok(Json(ApiResponse::success(ZSetAlgebraResponse { members })))
}

/// ZUNIONSTORE - Store the union of multiple sorted sets
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/unionstore",
    tag = "Sorted Sets",
    request_body = ZSetAlgebraStoreRequest,
    responses(
        (status = 200, description = "Union stored successfully", body = ZSetAlgebraStoreResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn zunionstore(
    State(state): State<AppState>,
    Json(request): Json<ZSetAlgebraStoreRequest>,
) -> Result<Json<ApiResponse<ZSetAlgebraStoreResponse>>, CacheError> {
    let options = convert_algebra_options(request.options);
    let count = state
        .sorted_set_service
        .zunionstore(&request.destination, request.keys, options)
        .await?;
    Ok(Json(ApiResponse::success(ZSetAlgebraStoreResponse { count })))
}

/// ZINTER - Get the intersection of multiple sorted sets
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/inter",
    tag = "Sorted Sets",
    request_body = ZSetAlgebraRequest,
    responses(
        (status = 200, description = "Intersection result", body = ZSetAlgebraResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn zinter(
    State(state): State<AppState>,
    Json(request): Json<ZSetAlgebraRequest>,
) -> Result<Json<ApiResponse<ZSetAlgebraResponse>>, CacheError> {
    let options = convert_algebra_options(request.options);
    let members = state.sorted_set_service.zinter(request.keys, options).await?;
    let members = members.into_iter().map(convert_scored_member).collect();
    Ok(Json(ApiResponse::success(ZSetAlgebraResponse { members })))
}

/// ZINTERSTORE - Store the intersection of multiple sorted sets
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/interstore",
    tag = "Sorted Sets",
    request_body = ZSetAlgebraStoreRequest,
    responses(
        (status = 200, description = "Intersection stored successfully", body = ZSetAlgebraStoreResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn zinterstore(
    State(state): State<AppState>,
    Json(request): Json<ZSetAlgebraStoreRequest>,
) -> Result<Json<ApiResponse<ZSetAlgebraStoreResponse>>, CacheError> {
    let options = convert_algebra_options(request.options);
    let count = state
        .sorted_set_service
        .zinterstore(&request.destination, request.keys, options)
        .await?;
    Ok(Json(ApiResponse::success(ZSetAlgebraStoreResponse { count })))
}

/// ZINTERCARD - Get the cardinality of the intersection
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/intercard",
    tag = "Sorted Sets",
    request_body = ZInterCardRequest,
    responses(
        (status = 200, description = "Intersection cardinality", body = ZInterCardResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn zintercard(
    State(state): State<AppState>,
    Json(request): Json<ZInterCardRequest>,
) -> Result<Json<ApiResponse<ZInterCardResponse>>, CacheError> {
    let cardinality = state
        .sorted_set_service
        .zintercard(request.keys, request.limit)
        .await?;
    Ok(Json(ApiResponse::success(ZInterCardResponse { cardinality })))
}

/// ZDIFF - Get the difference of sorted sets
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/diff",
    tag = "Sorted Sets",
    request_body = ZDiffRequest,
    responses(
        (status = 200, description = "Difference result", body = ZSetAlgebraResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn zdiff(
    State(state): State<AppState>,
    Json(request): Json<ZDiffRequest>,
) -> Result<Json<ApiResponse<ZSetAlgebraResponse>>, CacheError> {
    let members = state
        .sorted_set_service
        .zdiff(request.keys, request.with_scores)
        .await?;
    let members = members.into_iter().map(convert_scored_member).collect();
    Ok(Json(ApiResponse::success(ZSetAlgebraResponse { members })))
}

/// ZDIFFSTORE - Store the difference of sorted sets
#[utoipa::path(
    post,
    path = "/api/v1/sorted-sets/diffstore",
    tag = "Sorted Sets",
    request_body = ZDiffStoreRequest,
    responses(
        (status = 200, description = "Difference stored successfully", body = ZSetAlgebraStoreResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn zdiffstore(
    State(state): State<AppState>,
    Json(request): Json<ZDiffStoreRequest>,
) -> Result<Json<ApiResponse<ZSetAlgebraStoreResponse>>, CacheError> {
    let count = state
        .sorted_set_service
        .zdiffstore(&request.destination, request.keys)
        .await?;
    Ok(Json(ApiResponse::success(ZSetAlgebraStoreResponse { count })))
}

// ========== Scan operation ==========

/// ZSCAN - Incrementally iterate sorted set members
#[utoipa::path(
    get,
    path = "/api/v1/sorted-sets/{key}/scan",
    tag = "Sorted Sets",
    params(
        ("key" = String, Path, description = "The sorted set key"),
        ("cursor" = Option<u64>, Query, description = "Cursor position (0 to start)"),
        ("pattern" = Option<String>, Query, description = "Pattern to match members"),
        ("count" = Option<u64>, Query, description = "Hint for number of members to return")
    ),
    responses(
        (status = 200, description = "Scan result", body = ZScanResponse)
    )
)]
pub async fn zscan(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<ZScanQuery>,
) -> Result<Json<ApiResponse<ZScanResponse>>, CacheError> {
    let result = state
        .sorted_set_service
        .zscan(&key, query.cursor, query.pattern.as_deref(), query.count)
        .await?;
    let members = result.members.into_iter().map(convert_scored_member).collect();
    Ok(Json(ApiResponse::success(ZScanResponse {
        cursor: result.cursor,
        members,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state_with_sorted_set_repo;
    use axum::extract::{Path, Query, State};
    use axum::Json;
    use std::collections::HashSet;

    fn member(name: &str, score: f64) -> ScoredMemberDto {
        ScoredMemberDto::new(name.to_string(), score)
    }

    #[test]
    fn test_sorted_set_helpers() {
        let dto = member("alpha", 1.0);
        let parsed = convert_to_scored_member(&dto);
        assert_eq!(parsed.member, "alpha");
        assert_eq!(parsed.score, 1.0);

        let roundtrip = convert_scored_member(parsed);
        assert_eq!(roundtrip.member, "alpha");
        assert_eq!(roundtrip.score, 1.0);

        let opts = convert_zadd_options(Some(ZAddOptionsDto {
            nx: true,
            xx: false,
            gt: false,
            lt: false,
            ch: true,
        }))
        .expect("opts");
        assert!(opts.nx);
        assert!(opts.ch);

        assert_eq!(parse_score("-inf"), f64::NEG_INFINITY);
        assert_eq!(parse_score("+inf"), f64::INFINITY);
        assert_eq!(parse_score("inf"), f64::INFINITY);
        assert_eq!(parse_score("3.5"), 3.5);
        assert_eq!(parse_score("not-a-number"), 0.0);

        let range = convert_score_range(&ScoreRangeDto {
            min: "(1.5".to_string(),
            max: "2.0".to_string(),
        });
        assert!(range.min_exclusive);
        assert!(!range.max_exclusive);
        assert_eq!(range.min, 1.5);
        assert_eq!(range.max, 2.0);

        let range = convert_score_range(&ScoreRangeDto {
            min: "1".to_string(),
            max: "(2.5".to_string(),
        });
        assert!(!range.min_exclusive);
        assert!(range.max_exclusive);

        let lex = convert_lex_range(&LexRangeDto {
            min: "[a".to_string(),
            max: "[z".to_string(),
        });
        assert_eq!(lex.min, "[a");
        assert_eq!(lex.max, "[z");

        assert!(matches!(convert_aggregate(&ZAggregateDto::Sum), ZAggregate::Sum));
        assert!(matches!(convert_aggregate(&ZAggregateDto::Min), ZAggregate::Min));
        assert!(matches!(convert_aggregate(&ZAggregateDto::Max), ZAggregate::Max));

        let algebra_opts = convert_algebra_options(Some(ZSetAlgebraOptionsDto {
            weights: Some(vec![1.0, 2.0]),
            aggregate: ZAggregateDto::Max,
            with_scores: true,
        }))
        .expect("algebra options");
        assert_eq!(algebra_opts.weights, Some(vec![1.0, 2.0]));
        assert!(matches!(algebra_opts.aggregate, ZAggregate::Max));
        assert!(algebra_opts.with_scores);
    }

    #[tokio::test]
    async fn test_sorted_set_routes_basic() {
        let (state, _repo) = test_state_with_sorted_set_repo();
        let state = State(state);

        let added = zadd(
            state.clone(),
            Path("zset".to_string()),
            Json(ZAddRequest {
                members: vec![member("a", 1.0), member("b", 2.0), member("c", 3.0)],
                options: Some(ZAddOptionsDto {
                    nx: false,
                    xx: false,
                    gt: false,
                    lt: false,
                    ch: true,
                }),
            }),
        )
        .await
        .unwrap();
        assert_eq!(added.0.data.expect("data").count, 3);

        let incr = zadd_incr(
            state.clone(),
            Path("zset".to_string()),
            Json(ZAddIncrRequest {
                member: "b".to_string(),
                score: 1.5,
                options: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(incr.0.data.expect("data").new_score, Some(3.5));

        let score = zscore(state.clone(), Path(("zset".to_string(), "b".to_string())))
            .await
            .unwrap();
        assert_eq!(score.0.data.expect("data").score, Some(3.5));

        let scores = zmscore(
            state.clone(),
            Path("zset".to_string()),
            Json(ZMScoreRequest {
                members: vec!["a".to_string(), "b".to_string(), "missing".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            scores.0.data.expect("data").scores,
            vec![Some(1.0), Some(3.5), None]
        );

        let incr_by = zincrby(
            state.clone(),
            Path("zset".to_string()),
            Json(ZIncrByRequest {
                member: "a".to_string(),
                increment: 1.0,
            }),
        )
        .await
        .unwrap();
        assert_eq!(incr_by.0.data.expect("data").new_score, 2.0);

        let card = zcard(state.clone(), Path("zset".to_string()))
            .await
            .unwrap();
        assert_eq!(card.0.data.expect("data").cardinality, 3);

        let count = zcount(
            state.clone(),
            Path("zset".to_string()),
            Json(ZCountRequest {
                range: ScoreRangeDto {
                    min: "(1".to_string(),
                    max: "3.5".to_string(),
                },
            }),
        )
        .await
        .unwrap();
        assert_eq!(count.0.data.expect("data").count, 3);

        let lexcount = zlexcount(
            state.clone(),
            Path("zset".to_string()),
            Json(ZLexCountRequest {
                range: LexRangeDto {
                    min: "[a".to_string(),
                    max: "[z".to_string(),
                },
            }),
        )
        .await
        .unwrap();
        assert_eq!(lexcount.0.data.expect("data").count, 3);

        let rank = zrank(
            state.clone(),
            Path(("zset".to_string(), "a".to_string())),
        )
        .await
        .unwrap();
        assert_eq!(rank.0.data.expect("data").rank, Some(0));

        let revrank = zrevrank(
            state.clone(),
            Path(("zset".to_string(), "a".to_string())),
        )
        .await
        .unwrap();
        assert_eq!(revrank.0.data.expect("data").rank, Some(2));

        let removed = zrem(
            state,
            Path("zset".to_string()),
            Json(ZRemRequest {
                members: vec!["c".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(removed.0.data.expect("data").removed, 1);
    }

    #[tokio::test]
    async fn test_sorted_set_routes_range_and_remove() {
        let (state, _repo) = test_state_with_sorted_set_repo();
        let state = State(state);

        let _ = zadd(
            state.clone(),
            Path("range_set".to_string()),
            Json(ZAddRequest {
                members: vec![
                    member("a", 1.0),
                    member("b", 2.0),
                    member("c", 3.0),
                    member("d", 4.0),
                ],
                options: None,
            }),
        )
        .await
        .unwrap();

        let ranged = zrange(
            state.clone(),
            Path("range_set".to_string()),
            Query(ZRangeQuery {
                start: 0,
                stop: 2,
                with_scores: true,
                rev: true,
            }),
        )
        .await
        .unwrap();
        assert_eq!(ranged.0.data.expect("data").members.len(), 3);

        let by_score = zrangebyscore(
            state.clone(),
            Path("range_set".to_string()),
            Json(ZRangeByScoreRequest {
                range: ScoreRangeDto {
                    min: "1".to_string(),
                    max: "(4".to_string(),
                },
                with_scores: true,
                rev: true,
                offset: Some(0),
                count: Some(2),
            }),
        )
        .await
        .unwrap();
        let members = by_score.0.data.expect("data").members;
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].member, "c");

        let by_lex = zrangebylex(
            state.clone(),
            Path("range_set".to_string()),
            Json(ZRangeByLexRequest {
                range: LexRangeDto {
                    min: "-".to_string(),
                    max: "+".to_string(),
                },
                rev: true,
                offset: Some(0),
                count: Some(10),
            }),
        )
        .await
        .unwrap();
        assert_eq!(by_lex.0.data.expect("data").members.len(), 4);

        let stored = zrangestore(
            state.clone(),
            Path("range_set".to_string()),
            Json(ZRangeStoreRequest {
                destination: "range_dest".to_string(),
                start: 0,
                stop: 1,
                with_scores: true,
                rev: false,
            }),
        )
        .await
        .unwrap();
        assert_eq!(stored.0.data.expect("data").count, 2);

        let removed = zremrangebyrank(
            state.clone(),
            Path("range_set".to_string()),
            Json(ZRemRangeByRankRequest { start: 0, stop: 0 }),
        )
        .await
        .unwrap();
        assert_eq!(removed.0.data.expect("data").removed, 1);

        let _ = zadd(
            state.clone(),
            Path("score_set".to_string()),
            Json(ZAddRequest {
                members: vec![
                    member("a", 1.0),
                    member("b", 2.0),
                    member("c", 3.0),
                    member("d", 4.0),
                ],
                options: None,
            }),
        )
        .await
        .unwrap();

        let removed = zremrangebyscore(
            state.clone(),
            Path("score_set".to_string()),
            Json(ZRemRangeByScoreRequest {
                range: ScoreRangeDto {
                    min: "2".to_string(),
                    max: "3".to_string(),
                },
            }),
        )
        .await
        .unwrap();
        assert_eq!(removed.0.data.expect("data").removed, 2);

        let removed = zremrangebylex(
            state,
            Path("score_set".to_string()),
            Json(ZRemRangeByLexRequest {
                range: LexRangeDto {
                    min: "-".to_string(),
                    max: "+".to_string(),
                },
            }),
        )
        .await
        .unwrap();
        assert_eq!(removed.0.data.expect("data").removed, 0);
    }

    #[tokio::test]
    async fn test_sorted_set_routes_pop_and_blocking() {
        let (state, _repo) = test_state_with_sorted_set_repo();
        let state = State(state);

        let _ = zadd(
            state.clone(),
            Path("popset".to_string()),
            Json(ZAddRequest {
                members: vec![member("a", 1.0), member("b", 2.0), member("c", 3.0)],
                options: None,
            }),
        )
        .await
        .unwrap();

        let popped = zpopmin(
            state.clone(),
            Path("popset".to_string()),
            Query(ZPopQuery { count: Some(1) }),
        )
        .await
        .unwrap();
        assert_eq!(popped.0.data.expect("data").members.len(), 1);

        let popped = zpopmax(
            state.clone(),
            Path("popset".to_string()),
            Query(ZPopQuery { count: Some(1) }),
        )
        .await
        .unwrap();
        assert_eq!(popped.0.data.expect("data").members.len(), 1);

        let blocked = bzpopmin(
            state.clone(),
            Json(ZBPopRequest {
                keys: vec!["popset".to_string()],
                timeout: 1.0,
            }),
        )
        .await
        .unwrap();
        assert_eq!(blocked.0.data.expect("data").key, Some("popset".to_string()));

        let blocked = bzpopmin(
            state.clone(),
            Json(ZBPopRequest {
                keys: vec!["missing".to_string()],
                timeout: 1.0,
            }),
        )
        .await
        .unwrap();
        assert!(blocked.0.data.expect("data").key.is_none());

        let _ = zadd(
            state.clone(),
            Path("blockset".to_string()),
            Json(ZAddRequest {
                members: vec![member("d", 4.0)],
                options: None,
            }),
        )
        .await
        .unwrap();

        let blocked = bzpopmax(
            state.clone(),
            Json(ZBPopRequest {
                keys: vec!["blockset".to_string()],
                timeout: 1.0,
            }),
        )
        .await
        .unwrap();
        assert_eq!(blocked.0.data.expect("data").key, Some("blockset".to_string()));

        let blocked = bzpopmax(
            state.clone(),
            Json(ZBPopRequest {
                keys: vec!["missing".to_string()],
                timeout: 1.0,
            }),
        )
        .await
        .unwrap();
        let data = blocked.0.data.expect("data");
        assert!(data.key.is_none());
        assert!(data.members.is_empty());

        let _ = zadd(
            state.clone(),
            Path("mpopset".to_string()),
            Json(ZAddRequest {
                members: vec![member("m1", 1.0)],
                options: None,
            }),
        )
        .await
        .unwrap();

        let popped = zmpop(
            state.clone(),
            Json(ZMPopRequest {
                keys: vec!["mpopset".to_string()],
                direction: "min".to_string(),
                count: Some(1),
            }),
        )
        .await
        .unwrap();
        assert_eq!(popped.0.data.expect("data").key, Some("mpopset".to_string()));

        let popped = zmpop(
            state.clone(),
            Json(ZMPopRequest {
                keys: vec!["missing".to_string()],
                direction: "min".to_string(),
                count: Some(1),
            }),
        )
        .await
        .unwrap();
        assert!(popped.0.data.expect("data").key.is_none());

        let err = zmpop(
            state.clone(),
            Json(ZMPopRequest {
                keys: vec!["popset".to_string()],
                direction: "side".to_string(),
                count: Some(1),
            }),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let _ = zadd(
            state.clone(),
            Path("bmpopset".to_string()),
            Json(ZAddRequest {
                members: vec![member("m2", 2.0)],
                options: None,
            }),
        )
        .await
        .unwrap();

        let popped = bzmpop(
            state.clone(),
            Json(ZBMPopRequest {
                keys: vec!["bmpopset".to_string()],
                direction: "max".to_string(),
                timeout: 1.0,
                count: Some(1),
            }),
        )
        .await
        .unwrap();
        assert_eq!(popped.0.data.expect("data").key, Some("bmpopset".to_string()));

        let popped = bzmpop(
            state.clone(),
            Json(ZBMPopRequest {
                keys: vec!["missing".to_string()],
                direction: "max".to_string(),
                timeout: 1.0,
                count: Some(1),
            }),
        )
        .await
        .unwrap();
        assert!(popped.0.data.expect("data").key.is_none());

        let err = bzmpop(
            state,
            Json(ZBMPopRequest {
                keys: vec!["popset".to_string()],
                direction: "invalid".to_string(),
                timeout: 1.0,
                count: Some(1),
            }),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_sorted_set_routes_random_scan_and_algebra() {
        let (state, _repo) = test_state_with_sorted_set_repo();
        let state = State(state);

        let _ = zadd(
            state.clone(),
            Path("randset".to_string()),
            Json(ZAddRequest {
                members: vec![member("x", 1.0), member("y", 2.0), member("z", 3.0)],
                options: None,
            }),
        )
        .await
        .unwrap();

        let random = zrandmember(
            state.clone(),
            Path("randset".to_string()),
            Query(ZRandMemberQuery {
                count: Some(2),
                with_scores: true,
            }),
        )
        .await
        .unwrap();
        let random_members = random.0.data.expect("data").members;
        assert_eq!(random_members.len(), 2);
        let allowed: HashSet<String> =
            ["x", "y", "z"].iter().map(|m| m.to_string()).collect();
        for m in random_members {
            assert!(allowed.contains(&m.member));
        }

        let scan = zscan(
            state.clone(),
            Path("randset".to_string()),
            Query(ZScanQuery {
                cursor: 0,
                pattern: Some("x".to_string()),
                count: Some(1),
            }),
        )
        .await
        .unwrap();
        let scan_data = scan.0.data.expect("data");
        assert_eq!(scan_data.cursor, 0);
        assert_eq!(scan_data.members.len(), 3);

        let _ = zadd(
            state.clone(),
            Path("union1".to_string()),
            Json(ZAddRequest {
                members: vec![member("a", 1.0), member("b", 2.0)],
                options: None,
            }),
        )
        .await
        .unwrap();
        let _ = zadd(
            state.clone(),
            Path("union2".to_string()),
            Json(ZAddRequest {
                members: vec![member("b", 3.0), member("c", 4.0)],
                options: None,
            }),
        )
        .await
        .unwrap();

        let union = zunion(
            state.clone(),
            Json(ZSetAlgebraRequest {
                keys: vec!["union1".to_string(), "union2".to_string()],
                options: Some(ZSetAlgebraOptionsDto {
                    weights: Some(vec![1.0, 2.0]),
                    aggregate: ZAggregateDto::Sum,
                    with_scores: true,
                }),
            }),
        )
        .await
        .unwrap();
        assert_eq!(union.0.data.expect("data").members.len(), 3);

        let unionstore = zunionstore(
            state.clone(),
            Json(ZSetAlgebraStoreRequest {
                destination: "union_dest".to_string(),
                keys: vec!["union1".to_string(), "union2".to_string()],
                options: Some(ZSetAlgebraOptionsDto {
                    weights: Some(vec![1.0, 2.0]),
                    aggregate: ZAggregateDto::Sum,
                    with_scores: false,
                }),
            }),
        )
        .await
        .unwrap();
        assert_eq!(unionstore.0.data.expect("data").count, 3);

        let inter = zinter(
            state.clone(),
            Json(ZSetAlgebraRequest {
                keys: vec!["union1".to_string(), "union2".to_string()],
                options: Some(ZSetAlgebraOptionsDto {
                    weights: Some(vec![1.0, 1.0]),
                    aggregate: ZAggregateDto::Sum,
                    with_scores: true,
                }),
            }),
        )
        .await
        .unwrap();
        assert_eq!(inter.0.data.expect("data").members.len(), 1);

        let interstore = zinterstore(
            state.clone(),
            Json(ZSetAlgebraStoreRequest {
                destination: "inter_dest".to_string(),
                keys: vec!["union1".to_string(), "union2".to_string()],
                options: Some(ZSetAlgebraOptionsDto {
                    weights: Some(vec![1.0, 1.0]),
                    aggregate: ZAggregateDto::Sum,
                    with_scores: false,
                }),
            }),
        )
        .await
        .unwrap();
        assert_eq!(interstore.0.data.expect("data").count, 1);

        let intercard = zintercard(
            state.clone(),
            Json(ZInterCardRequest {
                keys: vec!["union1".to_string(), "union2".to_string()],
                limit: Some(1),
            }),
        )
        .await
        .unwrap();
        assert_eq!(intercard.0.data.expect("data").cardinality, 1);

        let diff = zdiff(
            state.clone(),
            Json(ZDiffRequest {
                keys: vec!["union1".to_string(), "union2".to_string()],
                with_scores: true,
            }),
        )
        .await
        .unwrap();
        assert_eq!(diff.0.data.expect("data").members.len(), 1);

        let diffstore = zdiffstore(
            state,
            Json(ZDiffStoreRequest {
                destination: "diff_dest".to_string(),
                keys: vec!["union1".to_string(), "union2".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(diffstore.0.data.expect("data").count, 1);
    }
}
