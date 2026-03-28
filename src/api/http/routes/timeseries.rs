//! RedisTimeSeries routes.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post},
};
use validator::Validate;

use crate::api::http::schemas::functions::FunctionSuccessResponse;
use crate::api::http::schemas::timeseries::{
    Aggregation, DuplicatePolicy, Sample, TimeSeriesAddRequest, TimeSeriesCreateRequest,
    TimeSeriesGetResponse, TimeSeriesMGetItem, TimeSeriesMGetRequest, TimeSeriesMGetResponse,
    TimeSeriesMRangeRequest, TimeSeriesMRangeResponse, TimeSeriesRangeItem, TimeSeriesRangeQuery,
    TimeSeriesRangeResponse, TimeSeriesWriteResponse,
    TsAlterRequest, TsCreateRuleRequest, TsDelQuery, TsDelResponse,
    TsIncrDecrRequest, TsMaddRequest, TsMaddResponse,
    TsMrevRangeRequest, TsQueryIndexRequest, TsQueryIndexResponse, TsInfoResponse,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    TimeSeriesCreateOptions, TimeSeriesRangeOptions, TimeSeriesSample, TsAggregation,
    TsDuplicatePolicy,
};
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

pub fn timeseries_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/timeseries", post(ts_create))
        // Static paths MUST come before wildcard {key} paths
        .route("/api/v1/timeseries/madd", post(ts_madd))
        .route("/api/v1/timeseries/mget", post(ts_mget))
        .route("/api/v1/timeseries/mrange", post(ts_mrange))
        .route("/api/v1/timeseries/mrevrange", post(ts_mrev_range))
        .route("/api/v1/timeseries/queryindex", post(ts_query_index))
        // Wildcard {key} paths
        .route("/api/v1/timeseries/{key}", get(ts_get).patch(ts_alter))
        .route("/api/v1/timeseries/{key}/samples", post(ts_add).delete(ts_del))
        .route("/api/v1/timeseries/{key}/range", get(ts_range))
        .route("/api/v1/timeseries/{key}/revrange", get(ts_rev_range))
        .route("/api/v1/timeseries/{key}/incrby", post(ts_incr_by))
        .route("/api/v1/timeseries/{key}/decrby", post(ts_decr_by))
        .route("/api/v1/timeseries/{key}/info", get(ts_info))
        .route("/api/v1/timeseries/{key}/rules", post(ts_create_rule))
        .route("/api/v1/timeseries/{key}/rules/{dest_key}", delete(ts_delete_rule))
}

fn require_timeseries(state: &AppState) -> Result<(), CacheError> {
    if !state.capabilities.modules.timeseries {
        return Err(CacheError::ModuleNotAvailable(
            "RedisTimeSeries module is not available".to_string(),
        ));
    }
    Ok(())
}

fn to_duplicate_policy(policy: DuplicatePolicy) -> TsDuplicatePolicy {
    match policy {
        DuplicatePolicy::Block => TsDuplicatePolicy::Block,
        DuplicatePolicy::First => TsDuplicatePolicy::First,
        DuplicatePolicy::Last => TsDuplicatePolicy::Last,
        DuplicatePolicy::Min => TsDuplicatePolicy::Min,
        DuplicatePolicy::Max => TsDuplicatePolicy::Max,
        DuplicatePolicy::Sum => TsDuplicatePolicy::Sum,
    }
}

fn to_aggregation(aggregation: Aggregation) -> TsAggregation {
    match aggregation {
        Aggregation::Avg => TsAggregation::Avg,
        Aggregation::Sum => TsAggregation::Sum,
        Aggregation::Min => TsAggregation::Min,
        Aggregation::Max => TsAggregation::Max,
        Aggregation::Range => TsAggregation::Range,
        Aggregation::Count => TsAggregation::Count,
        Aggregation::First => TsAggregation::First,
        Aggregation::Last => TsAggregation::Last,
        Aggregation::StdP => TsAggregation::StdP,
        Aggregation::StdS => TsAggregation::StdS,
        Aggregation::VarP => TsAggregation::VarP,
        Aggregation::VarS => TsAggregation::VarS,
        Aggregation::Twa => TsAggregation::Twa,
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/timeseries",
    request_body = TimeSeriesCreateRequest,
    responses(
        (status = 200, description = "Time series created"),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_create(
    State(state): State<AppState>,
    Json(request): Json<TimeSeriesCreateRequest>,
) -> Result<Json<ApiResponse<FunctionSuccessResponse>>, CacheError> {
    require_timeseries(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let options = TimeSeriesCreateOptions {
        retention_ms: request.retention_ms,
        chunk_size: request.chunk_size,
        duplicate_policy: request.duplicate_policy.map(to_duplicate_policy),
        labels: request.labels,
    };
    state
        .timeseries_service
        .ts_create(&request.key, options)
        .await?;
    Ok(Json(ApiResponse::success(FunctionSuccessResponse {
        success: true,
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/timeseries/{key}/samples",
    params(("key" = String, Path, description = "Time series key")),
    request_body = TimeSeriesAddRequest,
    responses(
        (status = 200, description = "Sample added", body = TimeSeriesWriteResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_add(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TimeSeriesAddRequest>,
) -> Result<Json<ApiResponse<TimeSeriesWriteResponse>>, CacheError> {
    require_timeseries(&state)?;
    let sample = Sample {
        timestamp: request.timestamp,
        value: request.value,
    };
    sample
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let timestamp = state
        .timeseries_service
        .ts_add(
            &key,
            TimeSeriesSample {
                timestamp: request.timestamp,
                value: request.value,
            },
        )
        .await?;
    Ok(Json(ApiResponse::success(TimeSeriesWriteResponse {
        timestamp,
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/timeseries/{key}",
    params(("key" = String, Path, description = "Time series key")),
    responses(
        (status = 200, description = "Latest sample", body = TimeSeriesGetResponse),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_get(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<TimeSeriesGetResponse>>, CacheError> {
    require_timeseries(&state)?;
    let sample = state
        .timeseries_service
        .ts_get(&key)
        .await?
        .map(|sample| Sample {
            timestamp: sample.timestamp,
            value: sample.value,
        });
    Ok(Json(ApiResponse::success(TimeSeriesGetResponse { sample })))
}

#[utoipa::path(
    get,
    path = "/api/v1/timeseries/{key}/range",
    params(
        ("key" = String, Path, description = "Time series key"),
        ("from" = i64, Query, description = "Start timestamp"),
        ("to" = i64, Query, description = "End timestamp"),
        ("count" = Option<u64>, Query, description = "Optional sample limit"),
        ("aggregation" = Option<Aggregation>, Query, description = "Aggregation type"),
        ("bucket_duration_ms" = Option<u64>, Query, description = "Aggregation bucket duration")
    ),
    responses(
        (status = 200, description = "Range samples", body = TimeSeriesRangeResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_range(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<TimeSeriesRangeQuery>,
) -> Result<Json<ApiResponse<TimeSeriesRangeResponse>>, CacheError> {
    require_timeseries(&state)?;
    query
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let options = TimeSeriesRangeOptions {
        count: query.count,
        aggregation: query.aggregation.map(to_aggregation),
        bucket_duration_ms: query.bucket_duration_ms,
    };
    let samples = state
        .timeseries_service
        .ts_range(&key, query.from, query.to, options)
        .await?
        .into_iter()
        .map(|sample| Sample {
            timestamp: sample.timestamp,
            value: sample.value,
        })
        .collect();
    Ok(Json(ApiResponse::success(TimeSeriesRangeResponse {
        samples,
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/timeseries/mget",
    request_body = TimeSeriesMGetRequest,
    responses(
        (status = 200, description = "Latest samples", body = TimeSeriesMGetResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_mget(
    State(state): State<AppState>,
    Json(request): Json<TimeSeriesMGetRequest>,
) -> Result<Json<ApiResponse<TimeSeriesMGetResponse>>, CacheError> {
    require_timeseries(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let series = state
        .timeseries_service
        .ts_mget(&request.filters)
        .await?
        .into_iter()
        .map(|item| TimeSeriesMGetItem {
            key: item.key,
            labels: item.labels,
            sample: item.sample.map(|sample| Sample {
                timestamp: sample.timestamp,
                value: sample.value,
            }),
        })
        .collect();
    Ok(Json(ApiResponse::success(TimeSeriesMGetResponse {
        series,
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/timeseries/mrange",
    request_body = TimeSeriesMRangeRequest,
    responses(
        (status = 200, description = "Multi-series range samples", body = TimeSeriesMRangeResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_mrange(
    State(state): State<AppState>,
    Json(request): Json<TimeSeriesMRangeRequest>,
) -> Result<Json<ApiResponse<TimeSeriesMRangeResponse>>, CacheError> {
    require_timeseries(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let options = TimeSeriesRangeOptions {
        count: request.count,
        aggregation: request.aggregation.map(to_aggregation),
        bucket_duration_ms: request.bucket_duration_ms,
    };
    let series = state
        .timeseries_service
        .ts_mrange(request.from, request.to, &request.filters, options)
        .await?
        .into_iter()
        .map(|item| TimeSeriesRangeItem {
            key: item.key,
            labels: item.labels,
            samples: item
                .samples
                .into_iter()
                .map(|sample| Sample {
                    timestamp: sample.timestamp,
                    value: sample.value,
                })
                .collect(),
        })
        .collect();
    Ok(Json(ApiResponse::success(TimeSeriesMRangeResponse {
        series,
    })))
}

#[utoipa::path(
    patch,
    path = "/api/v1/timeseries/{key}",
    params(("key" = String, Path, description = "Time series key")),
    request_body = TsAlterRequest,
    responses(
        (status = 200, description = "Time series altered"),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_alter(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TsAlterRequest>,
) -> Result<Json<ApiResponse<FunctionSuccessResponse>>, CacheError> {
    require_timeseries(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let options = TimeSeriesCreateOptions {
        retention_ms: request.retention_ms,
        chunk_size: request.chunk_size,
        duplicate_policy: request.duplicate_policy.map(to_duplicate_policy),
        labels: request.labels,
    };
    state.timeseries_service.ts_alter(&key, options).await?;
    Ok(Json(ApiResponse::success(FunctionSuccessResponse {
        success: true,
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/timeseries/madd",
    request_body = TsMaddRequest,
    responses(
        (status = 200, description = "Samples added", body = TsMaddResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_madd(
    State(state): State<AppState>,
    Json(request): Json<TsMaddRequest>,
) -> Result<Json<ApiResponse<TsMaddResponse>>, CacheError> {
    require_timeseries(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let items: Vec<(String, TimeSeriesSample)> = request
        .items
        .into_iter()
        .map(|item| {
            (
                item.key,
                TimeSeriesSample {
                    timestamp: item.timestamp,
                    value: item.value,
                },
            )
        })
        .collect();
    let timestamps = state.timeseries_service.ts_madd(&items).await?;
    Ok(Json(ApiResponse::success(TsMaddResponse { timestamps })))
}

#[utoipa::path(
    post,
    path = "/api/v1/timeseries/{key}/incrby",
    params(("key" = String, Path, description = "Time series key")),
    request_body = TsIncrDecrRequest,
    responses(
        (status = 200, description = "Value incremented", body = TimeSeriesWriteResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_incr_by(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TsIncrDecrRequest>,
) -> Result<Json<ApiResponse<TimeSeriesWriteResponse>>, CacheError> {
    require_timeseries(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let timestamp = state
        .timeseries_service
        .ts_incr_by(&key, request.value, request.timestamp)
        .await?;
    Ok(Json(ApiResponse::success(TimeSeriesWriteResponse {
        timestamp,
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/timeseries/{key}/decrby",
    params(("key" = String, Path, description = "Time series key")),
    request_body = TsIncrDecrRequest,
    responses(
        (status = 200, description = "Value decremented", body = TimeSeriesWriteResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_decr_by(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TsIncrDecrRequest>,
) -> Result<Json<ApiResponse<TimeSeriesWriteResponse>>, CacheError> {
    require_timeseries(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let timestamp = state
        .timeseries_service
        .ts_decr_by(&key, request.value, request.timestamp)
        .await?;
    Ok(Json(ApiResponse::success(TimeSeriesWriteResponse {
        timestamp,
    })))
}

#[utoipa::path(
    delete,
    path = "/api/v1/timeseries/{key}/samples",
    params(
        ("key" = String, Path, description = "Time series key"),
        ("from" = i64, Query, description = "Start timestamp"),
        ("to" = i64, Query, description = "End timestamp")
    ),
    responses(
        (status = 200, description = "Samples deleted", body = TsDelResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_del(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<TsDelQuery>,
) -> Result<Json<ApiResponse<TsDelResponse>>, CacheError> {
    require_timeseries(&state)?;
    query
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let deleted = state
        .timeseries_service
        .ts_del(&key, query.from, query.to)
        .await?;
    Ok(Json(ApiResponse::success(TsDelResponse { deleted })))
}

#[utoipa::path(
    get,
    path = "/api/v1/timeseries/{key}/revrange",
    params(
        ("key" = String, Path, description = "Time series key"),
        ("from" = i64, Query, description = "Start timestamp"),
        ("to" = i64, Query, description = "End timestamp"),
        ("count" = Option<u64>, Query, description = "Optional sample limit"),
        ("aggregation" = Option<Aggregation>, Query, description = "Aggregation type"),
        ("bucket_duration_ms" = Option<u64>, Query, description = "Aggregation bucket duration")
    ),
    responses(
        (status = 200, description = "Reverse range samples", body = TimeSeriesRangeResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_rev_range(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<TimeSeriesRangeQuery>,
) -> Result<Json<ApiResponse<TimeSeriesRangeResponse>>, CacheError> {
    require_timeseries(&state)?;
    query
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let options = TimeSeriesRangeOptions {
        count: query.count,
        aggregation: query.aggregation.map(to_aggregation),
        bucket_duration_ms: query.bucket_duration_ms,
    };
    let samples = state
        .timeseries_service
        .ts_rev_range(&key, query.from, query.to, options)
        .await?
        .into_iter()
        .map(|sample| Sample {
            timestamp: sample.timestamp,
            value: sample.value,
        })
        .collect();
    Ok(Json(ApiResponse::success(TimeSeriesRangeResponse {
        samples,
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/timeseries/mrevrange",
    request_body = TsMrevRangeRequest,
    responses(
        (status = 200, description = "Multi-series reverse range samples", body = TimeSeriesMRangeResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_mrev_range(
    State(state): State<AppState>,
    Json(request): Json<TsMrevRangeRequest>,
) -> Result<Json<ApiResponse<TimeSeriesMRangeResponse>>, CacheError> {
    require_timeseries(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let options = TimeSeriesRangeOptions {
        count: request.count,
        aggregation: request.aggregation.map(to_aggregation),
        bucket_duration_ms: request.bucket_duration_ms,
    };
    let series = state
        .timeseries_service
        .ts_mrev_range(request.from, request.to, &request.filters, options)
        .await?
        .into_iter()
        .map(|item| TimeSeriesRangeItem {
            key: item.key,
            labels: item.labels,
            samples: item
                .samples
                .into_iter()
                .map(|sample| Sample {
                    timestamp: sample.timestamp,
                    value: sample.value,
                })
                .collect(),
        })
        .collect();
    Ok(Json(ApiResponse::success(TimeSeriesMRangeResponse {
        series,
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/timeseries/queryindex",
    request_body = TsQueryIndexRequest,
    responses(
        (status = 200, description = "Matching keys", body = TsQueryIndexResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_query_index(
    State(state): State<AppState>,
    Json(request): Json<TsQueryIndexRequest>,
) -> Result<Json<ApiResponse<TsQueryIndexResponse>>, CacheError> {
    require_timeseries(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let keys = state
        .timeseries_service
        .ts_query_index(&request.filters)
        .await?;
    Ok(Json(ApiResponse::success(TsQueryIndexResponse { keys })))
}

#[utoipa::path(
    get,
    path = "/api/v1/timeseries/{key}/info",
    params(("key" = String, Path, description = "Time series key")),
    responses(
        (status = 200, description = "Time series info", body = TsInfoResponse),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_info(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<TsInfoResponse>>, CacheError> {
    require_timeseries(&state)?;
    let info = state.timeseries_service.ts_info(&key).await?;
    Ok(Json(ApiResponse::success(TsInfoResponse { info })))
}

#[utoipa::path(
    post,
    path = "/api/v1/timeseries/{key}/rules",
    params(("key" = String, Path, description = "Source time series key")),
    request_body = TsCreateRuleRequest,
    responses(
        (status = 200, description = "Compaction rule created"),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_create_rule(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<TsCreateRuleRequest>,
) -> Result<Json<ApiResponse<FunctionSuccessResponse>>, CacheError> {
    require_timeseries(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    state
        .timeseries_service
        .ts_create_rule(
            &key,
            &request.dest_key,
            to_aggregation(request.aggregation),
            request.bucket_duration_ms,
        )
        .await?;
    Ok(Json(ApiResponse::success(FunctionSuccessResponse {
        success: true,
    })))
}

#[utoipa::path(
    delete,
    path = "/api/v1/timeseries/{key}/rules/{dest_key}",
    params(
        ("key" = String, Path, description = "Source time series key"),
        ("dest_key" = String, Path, description = "Destination time series key")
    ),
    responses(
        (status = 200, description = "Compaction rule deleted"),
        (status = 501, description = "RedisTimeSeries not available")
    ),
    tag = "TimeSeries"
)]
pub async fn ts_delete_rule(
    State(state): State<AppState>,
    Path((key, dest_key)): Path<(String, String)>,
) -> Result<Json<ApiResponse<FunctionSuccessResponse>>, CacheError> {
    require_timeseries(&state)?;
    state
        .timeseries_service
        .ts_delete_rule(&key, &dest_key)
        .await?;
    Ok(Json(ApiResponse::success(FunctionSuccessResponse {
        success: true,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http::schemas::timeseries::TsMaddItem;
    use crate::test_support::test_state_with_timeseries_repo;

    #[test]
    fn test_timeseries_routes_creation() {
        let _ = timeseries_routes();
    }

    #[tokio::test]
    async fn test_ts_create_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        let result = ts_create(
            State(state),
            Json(TimeSeriesCreateRequest {
                key: "metrics".to_string(),
                retention_ms: None,
                chunk_size: None,
                duplicate_policy: None,
                labels: std::collections::HashMap::new(),
            }),
        )
        .await
        .expect("create");
        assert!(result.0.data.expect("data").success);
    }

    #[tokio::test]
    async fn test_ts_add_and_get_handlers() {
        let (state, _) = test_state_with_timeseries_repo();
        let _ = ts_create(
            State(state.clone()),
            Json(TimeSeriesCreateRequest {
                key: "metrics".to_string(),
                retention_ms: None,
                chunk_size: None,
                duplicate_policy: None,
                labels: std::collections::HashMap::new(),
            }),
        )
        .await
        .expect("create");

        let add = ts_add(
            State(state.clone()),
            Path("metrics".to_string()),
            Json(TimeSeriesAddRequest {
                timestamp: 100,
                value: 1.5,
            }),
        )
        .await
        .expect("add");
        assert_eq!(add.0.data.expect("data").timestamp, 100);

        let get = ts_get(State(state), Path("metrics".to_string()))
            .await
            .expect("get");
        assert_eq!(get.0.data.expect("data").sample.expect("sample").value, 1.5);
    }

    #[tokio::test]
    async fn test_ts_range_mget_mrange_handlers() {
        let (state, _) = test_state_with_timeseries_repo();
        let _ = ts_create(
            State(state.clone()),
            Json(TimeSeriesCreateRequest {
                key: "metrics".to_string(),
                retention_ms: None,
                chunk_size: None,
                duplicate_policy: None,
                labels: std::collections::HashMap::new(),
            }),
        )
        .await
        .expect("create");
        let _ = ts_add(
            State(state.clone()),
            Path("metrics".to_string()),
            Json(TimeSeriesAddRequest {
                timestamp: 100,
                value: 1.0,
            }),
        )
        .await
        .expect("add1");
        let _ = ts_add(
            State(state.clone()),
            Path("metrics".to_string()),
            Json(TimeSeriesAddRequest {
                timestamp: 200,
                value: 2.0,
            }),
        )
        .await
        .expect("add2");

        let range = ts_range(
            State(state.clone()),
            Path("metrics".to_string()),
            Query(TimeSeriesRangeQuery {
                from: 0,
                to: 500,
                count: None,
                aggregation: None,
                bucket_duration_ms: None,
            }),
        )
        .await
        .expect("range");
        assert_eq!(range.0.data.expect("data").samples.len(), 2);

        let mget = ts_mget(
            State(state.clone()),
            Json(TimeSeriesMGetRequest {
                filters: vec!["metrics".to_string()],
            }),
        )
        .await
        .expect("mget");
        assert_eq!(mget.0.data.expect("data").series.len(), 1);

        let mrange = ts_mrange(
            State(state),
            Json(TimeSeriesMRangeRequest {
                from: 0,
                to: 500,
                filters: vec!["metrics".to_string()],
                count: None,
                aggregation: None,
                bucket_duration_ms: None,
            }),
        )
        .await
        .expect("mrange");
        assert_eq!(mrange.0.data.expect("data").series.len(), 1);
    }

    #[tokio::test]
    async fn test_ts_create_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_create(
            State(state),
            Json(TimeSeriesCreateRequest {
                key: "metrics".to_string(),
                retention_ms: None,
                chunk_size: None,
                duplicate_policy: None,
                labels: std::collections::HashMap::new(),
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_add_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_add(
            State(state),
            Path("metrics".to_string()),
            Json(TimeSeriesAddRequest {
                timestamp: 100,
                value: 1.0,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_range_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_range(
            State(state),
            Path("metrics".to_string()),
            Query(TimeSeriesRangeQuery {
                from: 0,
                to: 500,
                count: None,
                aggregation: None,
                bucket_duration_ms: None,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_mget_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_mget(
            State(state),
            Json(TimeSeriesMGetRequest {
                filters: vec!["sensor=temp".to_string()],
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_mrange_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_mrange(
            State(state),
            Json(TimeSeriesMRangeRequest {
                from: 0,
                to: 500,
                filters: vec!["sensor=temp".to_string()],
                count: None,
                aggregation: None,
                bucket_duration_ms: None,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_create_with_options() {
        let (state, _) = test_state_with_timeseries_repo();
        let mut labels = std::collections::HashMap::new();
        labels.insert("sensor".to_string(), "temp".to_string());
        let result = ts_create(
            State(state),
            Json(TimeSeriesCreateRequest {
                key: "metrics".to_string(),
                retention_ms: Some(60000),
                chunk_size: Some(4096),
                duplicate_policy: Some(DuplicatePolicy::Last),
                labels,
            }),
        )
        .await
        .expect("create with options");
        assert!(result.0.data.expect("data").success);
    }

    #[tokio::test]
    async fn test_ts_range_with_aggregation() {
        let (state, _) = test_state_with_timeseries_repo();
        let _ = ts_create(
            State(state.clone()),
            Json(TimeSeriesCreateRequest {
                key: "metrics".to_string(),
                retention_ms: None,
                chunk_size: None,
                duplicate_policy: None,
                labels: std::collections::HashMap::new(),
            }),
        )
        .await
        .expect("create");
        let _ = ts_add(
            State(state.clone()),
            Path("metrics".to_string()),
            Json(TimeSeriesAddRequest {
                timestamp: 100,
                value: 1.0,
            }),
        )
        .await
        .expect("add");

        let range = ts_range(
            State(state),
            Path("metrics".to_string()),
            Query(TimeSeriesRangeQuery {
                from: 0,
                to: 500,
                count: Some(10),
                aggregation: Some(Aggregation::Avg),
                bucket_duration_ms: Some(100),
            }),
        )
        .await
        .expect("range with agg");
        assert!(!range.0.data.expect("data").samples.is_empty());
    }

    #[tokio::test]
    async fn test_ts_get_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_get(State(state), Path("metrics".to_string())).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_alter_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        let _ = ts_create(
            State(state.clone()),
            Json(TimeSeriesCreateRequest {
                key: "metrics".to_string(),
                retention_ms: None,
                chunk_size: None,
                duplicate_policy: None,
                labels: std::collections::HashMap::new(),
            }),
        )
        .await
        .expect("create");
        let result = ts_alter(
            State(state),
            Path("metrics".to_string()),
            Json(TsAlterRequest {
                retention_ms: Some(120000),
                chunk_size: None,
                duplicate_policy: Some(DuplicatePolicy::Last),
                labels: std::collections::HashMap::new(),
            }),
        )
        .await
        .expect("alter");
        assert!(result.0.data.expect("data").success);
    }

    #[tokio::test]
    async fn test_ts_alter_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_alter(
            State(state),
            Path("metrics".to_string()),
            Json(TsAlterRequest {
                retention_ms: None,
                chunk_size: None,
                duplicate_policy: None,
                labels: std::collections::HashMap::new(),
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_madd_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        let result = ts_madd(
            State(state),
            Json(TsMaddRequest {
                items: vec![
                    TsMaddItem {
                        key: "k1".to_string(),
                        timestamp: 100,
                        value: 1.0,
                    },
                    TsMaddItem {
                        key: "k2".to_string(),
                        timestamp: 200,
                        value: 2.0,
                    },
                ],
            }),
        )
        .await
        .expect("madd");
        assert_eq!(result.0.data.expect("data").timestamps, vec![100, 200]);
    }

    #[tokio::test]
    async fn test_ts_madd_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_madd(
            State(state),
            Json(TsMaddRequest {
                items: vec![TsMaddItem {
                    key: "k1".to_string(),
                    timestamp: 100,
                    value: 1.0,
                }],
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_incr_by_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        let result = ts_incr_by(
            State(state),
            Path("metrics".to_string()),
            Json(TsIncrDecrRequest {
                value: 5.0,
                timestamp: Some(999),
            }),
        )
        .await
        .expect("incr_by");
        assert_eq!(result.0.data.expect("data").timestamp, 999);
    }

    #[tokio::test]
    async fn test_ts_incr_by_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_incr_by(
            State(state),
            Path("metrics".to_string()),
            Json(TsIncrDecrRequest {
                value: 5.0,
                timestamp: None,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_decr_by_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        let result = ts_decr_by(
            State(state),
            Path("metrics".to_string()),
            Json(TsIncrDecrRequest {
                value: 3.0,
                timestamp: None,
            }),
        )
        .await
        .expect("decr_by");
        assert_eq!(result.0.data.expect("data").timestamp, 1);
    }

    #[tokio::test]
    async fn test_ts_decr_by_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_decr_by(
            State(state),
            Path("metrics".to_string()),
            Json(TsIncrDecrRequest {
                value: 3.0,
                timestamp: None,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_del_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        // Create a key and add a sample so we can delete it
        let _ = ts_create(
            State(state.clone()),
            Json(TimeSeriesCreateRequest {
                key: "metrics".to_string(),
                retention_ms: None,
                chunk_size: None,
                duplicate_policy: None,
                labels: std::collections::HashMap::new(),
            }),
        )
        .await
        .expect("create");
        let _ = ts_add(
            State(state.clone()),
            Path("metrics".to_string()),
            Json(TimeSeriesAddRequest {
                timestamp: 50,
                value: 1.0,
            }),
        )
        .await
        .expect("add");
        let result = ts_del(
            State(state),
            Path("metrics".to_string()),
            Query(TsDelQuery { from: 0, to: 100 }),
        )
        .await
        .expect("del");
        assert_eq!(result.0.data.expect("data").deleted, 1);
    }

    #[tokio::test]
    async fn test_ts_del_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_del(
            State(state),
            Path("metrics".to_string()),
            Query(TsDelQuery { from: 0, to: 100 }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_rev_range_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        // Create a key and add samples for rev_range to return
        let _ = ts_create(
            State(state.clone()),
            Json(TimeSeriesCreateRequest {
                key: "metrics".to_string(),
                retention_ms: None,
                chunk_size: None,
                duplicate_policy: None,
                labels: std::collections::HashMap::new(),
            }),
        )
        .await
        .expect("create");
        let _ = ts_add(
            State(state.clone()),
            Path("metrics".to_string()),
            Json(TimeSeriesAddRequest {
                timestamp: 100,
                value: 1.0,
            }),
        )
        .await
        .expect("add1");
        let _ = ts_add(
            State(state.clone()),
            Path("metrics".to_string()),
            Json(TimeSeriesAddRequest {
                timestamp: 200,
                value: 2.0,
            }),
        )
        .await
        .expect("add2");
        let result = ts_rev_range(
            State(state),
            Path("metrics".to_string()),
            Query(TimeSeriesRangeQuery {
                from: 0,
                to: 500,
                count: None,
                aggregation: None,
                bucket_duration_ms: None,
            }),
        )
        .await
        .expect("rev_range");
        let samples = result.0.data.expect("data").samples;
        assert_eq!(samples.len(), 2);
        // Reverse order: 200 first, then 100
        assert_eq!(samples[0].timestamp, 200);
        assert_eq!(samples[1].timestamp, 100);
    }

    #[tokio::test]
    async fn test_ts_rev_range_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_rev_range(
            State(state),
            Path("metrics".to_string()),
            Query(TimeSeriesRangeQuery {
                from: 0,
                to: 500,
                count: None,
                aggregation: None,
                bucket_duration_ms: None,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_mrev_range_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        let result = ts_mrev_range(
            State(state),
            Json(TsMrevRangeRequest {
                from: 0,
                to: 500,
                filters: vec!["sensor=temp".to_string()],
                count: None,
                aggregation: None,
                bucket_duration_ms: None,
            }),
        )
        .await
        .expect("mrev_range");
        assert_eq!(result.0.data.expect("data").series.len(), 1);
    }

    #[tokio::test]
    async fn test_ts_mrev_range_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_mrev_range(
            State(state),
            Json(TsMrevRangeRequest {
                from: 0,
                to: 500,
                filters: vec!["sensor=temp".to_string()],
                count: None,
                aggregation: None,
                bucket_duration_ms: None,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_query_index_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        let result = ts_query_index(
            State(state),
            Json(TsQueryIndexRequest {
                filters: vec!["sensor=temp".to_string()],
            }),
        )
        .await
        .expect("query_index");
        assert_eq!(
            result.0.data.expect("data").keys,
            vec!["sensor=temp".to_string()]
        );
    }

    #[tokio::test]
    async fn test_ts_query_index_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_query_index(
            State(state),
            Json(TsQueryIndexRequest {
                filters: vec!["sensor=temp".to_string()],
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_info_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        let result = ts_info(State(state), Path("metrics".to_string()))
            .await
            .expect("info");
        let info = result.0.data.expect("data").info;
        assert_eq!(info["key"], "metrics");
    }

    #[tokio::test]
    async fn test_ts_info_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_info(State(state), Path("metrics".to_string())).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_create_rule_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        let result = ts_create_rule(
            State(state),
            Path("source".to_string()),
            Json(TsCreateRuleRequest {
                dest_key: "dest".to_string(),
                aggregation: Aggregation::Avg,
                bucket_duration_ms: 60000,
            }),
        )
        .await
        .expect("create_rule");
        assert!(result.0.data.expect("data").success);
    }

    #[tokio::test]
    async fn test_ts_create_rule_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_create_rule(
            State(state),
            Path("source".to_string()),
            Json(TsCreateRuleRequest {
                dest_key: "dest".to_string(),
                aggregation: Aggregation::Avg,
                bucket_duration_ms: 60000,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_ts_delete_rule_handler() {
        let (state, _) = test_state_with_timeseries_repo();
        let result = ts_delete_rule(
            State(state),
            Path(("source".to_string(), "dest".to_string())),
        )
        .await
        .expect("delete_rule");
        assert!(result.0.data.expect("data").success);
    }

    #[tokio::test]
    async fn test_ts_delete_rule_501_when_disabled() {
        let (mut state, _) = test_state_with_timeseries_repo();
        let mut caps = (*state.capabilities).clone();
        caps.modules.timeseries = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = ts_delete_rule(
            State(state),
            Path(("source".to_string(), "dest".to_string())),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }
}
