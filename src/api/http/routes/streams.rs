//! Stream Routes
//!
//! HTTP endpoints for Redis Stream operations.
//! Consumer group management endpoints are admin-protected.
//! SSE streaming endpoints for real-time data.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{delete, get, post},
    Json, Router,
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::time::Duration;

use crate::api::http::middleware::admin_auth::ADMIN_API_KEY_HEADER;
use crate::api::http::schemas::streams::*;
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Verify admin API key from headers
fn verify_admin_key(headers: &HeaderMap, state: &AppState) -> Result<(), CacheError> {
    let api_key = headers
        .get(ADMIN_API_KEY_HEADER)
        .and_then(|v| v.to_str().ok());

    match api_key {
        Some(key) if key == state.config.admin.api_key => Ok(()),
        _ => Err(CacheError::Unauthorized),
    }
}

/// Create stream routes (conditionally registered based on Redis 5.0+ capability)
pub fn stream_routes() -> Router<AppState> {
    Router::new()
        // Basic stream operations
        .route("/api/v1/streams/{key}/add", post(xadd))
        .route("/api/v1/streams/{key}/length", get(xlen))
        .route("/api/v1/streams/{key}/range", get(xrange))
        .route("/api/v1/streams/{key}/revrange", get(xrevrange))
        .route("/api/v1/streams/{key}/entries", delete(xdel))
        .route("/api/v1/streams/{key}/trim", post(xtrim))
        .route("/api/v1/streams/{key}/info", get(xinfo_stream))
        // Read operations
        .route("/api/v1/streams/read", post(xread))
        .route("/api/v1/streams/read/blocking", post(xread_blocking))
        // SSE streaming
        .route("/api/v1/streams/{key}/subscribe", get(stream_subscribe))
        // Consumer group info (public)
        .route("/api/v1/streams/{key}/groups", get(xinfo_groups))
        .route("/api/v1/streams/{key}/groups/{group}/consumers", get(xinfo_consumers))
        // Consumer group read operations
        .route("/api/v1/streams/{key}/groups/{group}/read", post(xreadgroup))
        .route("/api/v1/streams/{key}/groups/{group}/read/blocking", post(xreadgroup_blocking))
        .route("/api/v1/streams/{key}/groups/{group}/ack", post(xack))
        // Pending entries
        .route("/api/v1/streams/{key}/groups/{group}/pending", get(xpending_summary))
        .route("/api/v1/streams/{key}/groups/{group}/pending/detail", get(xpending))
        // Claim operations
        .route("/api/v1/streams/{key}/groups/{group}/claim", post(xclaim))
        .route("/api/v1/streams/{key}/groups/{group}/autoclaim", post(xautoclaim))
        // SSE streaming for consumer groups
        .route("/api/v1/streams/{key}/groups/{group}/subscribe", get(stream_group_subscribe))
}

/// Create admin-protected stream routes (consumer group management)
pub fn stream_admin_routes() -> Router<AppState> {
    Router::new()
        // Consumer group management (admin-protected)
        .route("/api/v1/streams/{key}/groups", post(xgroup_create))
        .route("/api/v1/streams/{key}/groups/{group}", delete(xgroup_destroy))
        .route("/api/v1/streams/{key}/groups/{group}/setid", post(xgroup_setid))
        .route("/api/v1/streams/{key}/groups/{group}/consumers", post(xgroup_createconsumer))
        .route("/api/v1/streams/{key}/groups/{group}/consumers/{consumer}", delete(xgroup_delconsumer))
        // Stream management (admin-protected)
        .route("/api/v1/streams/{key}/setid", post(xsetid))
}

// ========== Basic Stream Operations ==========

/// POST /api/v1/streams/{key}/add
///
/// Add entry to stream (XADD).
#[utoipa::path(
    post,
    path = "/api/v1/streams/{key}/add",
    params(
        ("key" = String, Path, description = "The stream key")
    ),
    request_body = StreamAddRequest,
    responses(
        (status = 200, description = "Entry added successfully", body = StreamAddResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Streams"
)]
pub async fn xadd(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<StreamAddRequest>,
) -> Result<Json<ApiResponse<StreamAddResponse>>, CacheError> {
    let id = state
        .stream_service
        .xadd(&key, req.fields.clone(), req.into())
        .await?;
    Ok(Json(ApiResponse::success(StreamAddResponse { id })))
}

/// GET /api/v1/streams/{key}/length
///
/// Get stream length (XLEN).
#[utoipa::path(
    get,
    path = "/api/v1/streams/{key}/length",
    params(
        ("key" = String, Path, description = "The stream key")
    ),
    responses(
        (status = 200, description = "Stream length", body = StreamLengthResponse)
    ),
    tag = "Streams"
)]
pub async fn xlen(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<StreamLengthResponse>>, CacheError> {
    let length = state.stream_service.xlen(&key).await?;
    Ok(Json(ApiResponse::success(StreamLengthResponse { length })))
}

/// GET /api/v1/streams/{key}/range
///
/// Get entries in a range (XRANGE).
#[utoipa::path(
    get,
    path = "/api/v1/streams/{key}/range",
    params(
        ("key" = String, Path, description = "The stream key"),
        StreamRangeQuery
    ),
    responses(
        (status = 200, description = "Stream entries", body = StreamEntriesResponse)
    ),
    tag = "Streams"
)]
pub async fn xrange(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<StreamRangeQuery>,
) -> Result<Json<ApiResponse<StreamEntriesResponse>>, CacheError> {
    let entries = state
        .stream_service
        .xrange(&key, &query.start, &query.end, query.count)
        .await?;
    Ok(Json(ApiResponse::success(StreamEntriesResponse { entries })))
}

/// GET /api/v1/streams/{key}/revrange
///
/// Get entries in reverse order (XREVRANGE).
#[utoipa::path(
    get,
    path = "/api/v1/streams/{key}/revrange",
    params(
        ("key" = String, Path, description = "The stream key"),
        StreamRangeQuery
    ),
    responses(
        (status = 200, description = "Stream entries in reverse order", body = StreamEntriesResponse)
    ),
    tag = "Streams"
)]
pub async fn xrevrange(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<StreamRangeQuery>,
) -> Result<Json<ApiResponse<StreamEntriesResponse>>, CacheError> {
    let entries = state
        .stream_service
        .xrevrange(&key, &query.end, &query.start, query.count)
        .await?;
    Ok(Json(ApiResponse::success(StreamEntriesResponse { entries })))
}

/// DELETE /api/v1/streams/{key}/entries
///
/// Delete entries from stream (XDEL).
#[utoipa::path(
    delete,
    path = "/api/v1/streams/{key}/entries",
    params(
        ("key" = String, Path, description = "The stream key")
    ),
    request_body = StreamDeleteRequest,
    responses(
        (status = 200, description = "Entries deleted", body = StreamDeleteResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Streams"
)]
pub async fn xdel(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<StreamDeleteRequest>,
) -> Result<Json<ApiResponse<StreamDeleteResponse>>, CacheError> {
    let deleted = state.stream_service.xdel(&key, req.ids).await?;
    Ok(Json(ApiResponse::success(StreamDeleteResponse { deleted })))
}

/// POST /api/v1/streams/{key}/trim
///
/// Trim stream (XTRIM).
#[utoipa::path(
    post,
    path = "/api/v1/streams/{key}/trim",
    params(
        ("key" = String, Path, description = "The stream key")
    ),
    request_body = StreamTrimRequest,
    responses(
        (status = 200, description = "Stream trimmed", body = StreamTrimResponse)
    ),
    tag = "Streams"
)]
pub async fn xtrim(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<StreamTrimRequest>,
) -> Result<Json<ApiResponse<StreamTrimResponse>>, CacheError> {
    let trimmed = state
        .stream_service
        .xtrim(&key, req.strategy.into())
        .await?;
    Ok(Json(ApiResponse::success(StreamTrimResponse { trimmed })))
}

/// GET /api/v1/streams/{key}/info
///
/// Get stream information (XINFO STREAM).
#[utoipa::path(
    get,
    path = "/api/v1/streams/{key}/info",
    params(
        ("key" = String, Path, description = "The stream key"),
        StreamInfoQuery
    ),
    responses(
        (status = 200, description = "Stream information", body = StreamInfoResponse)
    ),
    tag = "Streams"
)]
pub async fn xinfo_stream(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<StreamInfoQuery>,
) -> Result<Json<ApiResponse<StreamInfoResponse>>, CacheError> {
    let info = state.stream_service.xinfo_stream(&key, query.full).await?;
    Ok(Json(ApiResponse::success(info)))
}

// ========== Read Operations ==========

/// POST /api/v1/streams/read
///
/// Read entries from one or more streams (XREAD).
#[utoipa::path(
    post,
    path = "/api/v1/streams/read",
    request_body = StreamReadRequest,
    responses(
        (status = 200, description = "Entries read", body = StreamReadResponse),
        (status = 204, description = "No entries available (timeout)"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Streams"
)]
pub async fn xread(
    State(state): State<AppState>,
    Json(req): Json<StreamReadRequest>,
) -> Result<impl IntoResponse, CacheError> {
    let streams: Vec<(String, String)> = (&req).into();
    let result = state
        .stream_service
        .xread(streams, req.to_options())
        .await?;

    match result {
        Some(entries) => Ok(Json(ApiResponse::success(entries)).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

/// POST /api/v1/streams/read/blocking
///
/// Read entries with blocking (XREAD BLOCK). Max timeout 30s.
#[utoipa::path(
    post,
    path = "/api/v1/streams/read/blocking",
    request_body = StreamReadBlockingRequest,
    responses(
        (status = 200, description = "Entries read", body = StreamReadResponse),
        (status = 204, description = "No entries available (timeout)"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Streams"
)]
pub async fn xread_blocking(
    State(state): State<AppState>,
    Json(req): Json<StreamReadBlockingRequest>,
) -> Result<impl IntoResponse, CacheError> {
    let streams: Vec<(String, String)> = req
        .streams
        .iter()
        .map(|s| (s.key.clone(), s.id.clone()))
        .collect();

    let result = state
        .stream_service
        .xread_blocking(streams, req.count, req.timeout_seconds)
        .await?;

    match result {
        Some(entries) => Ok(Json(ApiResponse::success(entries)).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

// ========== SSE Streaming ==========

/// GET /api/v1/streams/{key}/subscribe
///
/// Subscribe to stream entries via Server-Sent Events.
#[utoipa::path(
    get,
    path = "/api/v1/streams/{key}/subscribe",
    params(
        ("key" = String, Path, description = "The stream key"),
        StreamSubscribeQuery
    ),
    responses(
        (status = 200, description = "SSE stream of entries", content_type = "text/event-stream")
    ),
    tag = "Streams"
)]
pub async fn stream_subscribe(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<StreamSubscribeQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let mut last_id = query.last_id.clone();
        let count = query.count;
        let block_ms = crate::application::services::StreamService::default_sse_block_ms();

        loop {
            let streams = vec![(key.clone(), last_id.clone())];
            let options = crate::domain::entities::XReadOptions {
                count,
                block_ms: Some(block_ms),
            };

            match state.stream_service.xread(streams, options).await {
                Ok(Some(results)) => {
                    for result in results {
                        for entry in result.entries {
                            last_id = entry.id.clone();
                            let data = serde_json::to_string(&entry).unwrap_or_default();
                            yield Ok(Event::default().event("message").data(data));
                        }
                    }
                }
                Ok(None) => {
                    // No data, continue waiting
                }
                Err(e) => {
                    let error_data = serde_json::json!({ "error": e.to_string() }).to_string();
                    yield Ok(Event::default().event("error").data(error_data));
                    break;
                }
            }

            // Allow cancellation check
            tokio::task::yield_now().await;
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

// ========== Consumer Group Info ==========

/// GET /api/v1/streams/{key}/groups
///
/// Get information about consumer groups (XINFO GROUPS).
#[utoipa::path(
    get,
    path = "/api/v1/streams/{key}/groups",
    params(
        ("key" = String, Path, description = "The stream key")
    ),
    responses(
        (status = 200, description = "Consumer group information", body = ConsumerGroupInfoResponse)
    ),
    tag = "Streams"
)]
pub async fn xinfo_groups(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<ConsumerGroupInfoResponse>>, CacheError> {
    let groups = state.stream_service.xinfo_groups(&key).await?;
    Ok(Json(ApiResponse::success(groups)))
}

/// GET /api/v1/streams/{key}/groups/{group}/consumers
///
/// Get information about consumers in a group (XINFO CONSUMERS).
#[utoipa::path(
    get,
    path = "/api/v1/streams/{key}/groups/{group}/consumers",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name")
    ),
    responses(
        (status = 200, description = "Consumer information", body = ConsumerInfoResponse)
    ),
    tag = "Streams"
)]
pub async fn xinfo_consumers(
    State(state): State<AppState>,
    Path((key, group)): Path<(String, String)>,
) -> Result<Json<ApiResponse<ConsumerInfoResponse>>, CacheError> {
    let consumers = state.stream_service.xinfo_consumers(&key, &group).await?;
    Ok(Json(ApiResponse::success(consumers)))
}

// ========== Consumer Group Read Operations ==========

/// POST /api/v1/streams/{key}/groups/{group}/read
///
/// Read entries as a consumer group member (XREADGROUP).
#[utoipa::path(
    post,
    path = "/api/v1/streams/{key}/groups/{group}/read",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name")
    ),
    request_body = StreamReadGroupRequest,
    responses(
        (status = 200, description = "Entries read", body = StreamReadResponse),
        (status = 204, description = "No entries available (timeout)"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Streams"
)]
pub async fn xreadgroup(
    State(state): State<AppState>,
    Path((key, group)): Path<(String, String)>,
    Json(req): Json<StreamReadGroupRequest>,
) -> Result<impl IntoResponse, CacheError> {
    // Use path key with the ID from request (defaults to ">" for new entries)
    // The path key is authoritative - we ignore any key in the request body
    let id = req.streams.first().map(|s| s.id.clone()).unwrap_or_else(|| ">".to_string());
    let streams = vec![(key, id)];

    let result = state
        .stream_service
        .xreadgroup(&group, &req.consumer, streams, req.to_options())
        .await?;

    match result {
        Some(entries) => Ok(Json(ApiResponse::success(entries)).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

/// POST /api/v1/streams/{key}/groups/{group}/read/blocking
///
/// Read entries with blocking as a consumer group member (XREADGROUP BLOCK). Max timeout 30s.
#[utoipa::path(
    post,
    path = "/api/v1/streams/{key}/groups/{group}/read/blocking",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name")
    ),
    request_body = StreamReadGroupBlockingRequest,
    responses(
        (status = 200, description = "Entries read", body = StreamReadResponse),
        (status = 204, description = "No entries available (timeout)"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Streams"
)]
pub async fn xreadgroup_blocking(
    State(state): State<AppState>,
    Path((key, group)): Path<(String, String)>,
    Json(req): Json<StreamReadGroupBlockingRequest>,
) -> Result<impl IntoResponse, CacheError> {
    // Use path key with the ID from request (defaults to ">" for new entries)
    // The path key is authoritative - we ignore any key in the request body
    let id = req.streams.first().map(|s| s.id.clone()).unwrap_or_else(|| ">".to_string());
    let streams = vec![(key, id)];

    let result = state
        .stream_service
        .xreadgroup_blocking(
            &group,
            &req.consumer,
            streams,
            req.count,
            req.no_ack,
            req.timeout_seconds,
        ).await?;

    match result {
        Some(entries) => Ok(Json(ApiResponse::success(entries)).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

/// POST /api/v1/streams/{key}/groups/{group}/ack
///
/// Acknowledge entries (XACK).
#[utoipa::path(
    post,
    path = "/api/v1/streams/{key}/groups/{group}/ack",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name")
    ),
    request_body = StreamAckRequest,
    responses(
        (status = 200, description = "Entries acknowledged", body = StreamAckResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Streams"
)]
pub async fn xack(
    State(state): State<AppState>,
    Path((key, group)): Path<(String, String)>,
    Json(req): Json<StreamAckRequest>,
) -> Result<Json<ApiResponse<StreamAckResponse>>, CacheError> {
    let acknowledged = state.stream_service.xack(&key, &group, req.ids).await?;
    Ok(Json(ApiResponse::success(StreamAckResponse { acknowledged })))
}

// ========== Pending Entry Operations ==========

/// GET /api/v1/streams/{key}/groups/{group}/pending
///
/// Get pending entries summary (XPENDING).
#[utoipa::path(
    get,
    path = "/api/v1/streams/{key}/groups/{group}/pending",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name")
    ),
    responses(
        (status = 200, description = "Pending summary", body = PendingSummaryResponse)
    ),
    tag = "Streams"
)]
pub async fn xpending_summary(
    State(state): State<AppState>,
    Path((key, group)): Path<(String, String)>,
) -> Result<Json<ApiResponse<PendingSummaryResponse>>, CacheError> {
    let summary = state.stream_service.xpending_summary(&key, &group).await?;
    Ok(Json(ApiResponse::success(summary)))
}

/// GET /api/v1/streams/{key}/groups/{group}/pending/detail
///
/// Get pending entries with details (XPENDING with range).
#[utoipa::path(
    get,
    path = "/api/v1/streams/{key}/groups/{group}/pending/detail",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name"),
        PendingQuery
    ),
    responses(
        (status = 200, description = "Pending entries", body = PendingEntriesResponse)
    ),
    tag = "Streams"
)]
pub async fn xpending(
    State(state): State<AppState>,
    Path((key, group)): Path<(String, String)>,
    Query(query): Query<PendingQuery>,
) -> Result<Json<ApiResponse<PendingEntriesResponse>>, CacheError> {
    let entries = state
        .stream_service
        .xpending(&key, &group, query.into())
        .await?;
    Ok(Json(ApiResponse::success(entries)))
}

// ========== Claim Operations ==========

/// POST /api/v1/streams/{key}/groups/{group}/claim
///
/// Claim pending entries (XCLAIM).
#[utoipa::path(
    post,
    path = "/api/v1/streams/{key}/groups/{group}/claim",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name")
    ),
    request_body = StreamClaimRequest,
    responses(
        (status = 200, description = "Entries claimed", body = StreamClaimResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Streams"
)]
pub async fn xclaim(
    State(state): State<AppState>,
    Path((key, group)): Path<(String, String)>,
    Json(req): Json<StreamClaimRequest>,
) -> Result<Json<ApiResponse<StreamClaimResponse>>, CacheError> {
    let result = state
        .stream_service
        .xclaim(&key, &group, &req.consumer, req.ids.clone(), req.to_options())
        .await?;
    Ok(Json(ApiResponse::success(result)))
}

/// POST /api/v1/streams/{key}/groups/{group}/autoclaim
///
/// Auto-claim pending entries (XAUTOCLAIM).
#[utoipa::path(
    post,
    path = "/api/v1/streams/{key}/groups/{group}/autoclaim",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name")
    ),
    request_body = StreamAutoClaimRequest,
    responses(
        (status = 200, description = "Entries auto-claimed", body = StreamAutoClaimResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Streams"
)]
pub async fn xautoclaim(
    State(state): State<AppState>,
    Path((key, group)): Path<(String, String)>,
    Json(req): Json<StreamAutoClaimRequest>,
) -> Result<Json<ApiResponse<StreamAutoClaimResponse>>, CacheError> {
    let result = state
        .stream_service
        .xautoclaim(
            &key,
            &group,
            &req.consumer,
            req.min_idle_time_ms,
            &req.start,
            req.to_options(),
        ).await?;
    Ok(Json(ApiResponse::success(result)))
}

// ========== SSE Streaming for Consumer Groups ==========

/// GET /api/v1/streams/{key}/groups/{group}/subscribe
///
/// Subscribe to stream entries as a consumer group member via Server-Sent Events.
#[utoipa::path(
    get,
    path = "/api/v1/streams/{key}/groups/{group}/subscribe",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name"),
        StreamGroupSubscribeQuery
    ),
    responses(
        (status = 200, description = "SSE stream of entries", content_type = "text/event-stream"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Streams"
)]
pub async fn stream_group_subscribe(
    State(state): State<AppState>,
    Path((key, group)): Path<(String, String)>,
    Query(query): Query<StreamGroupSubscribeQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let consumer = query.consumer.clone();
        let count = query.count;
        let no_ack = query.no_ack;
        let block_ms = crate::application::services::StreamService::default_sse_block_ms();

        loop {
            let streams = vec![(key.clone(), ">".to_string())];
            let options = crate::domain::entities::XReadGroupOptions {
                count,
                block_ms: Some(block_ms),
                no_ack,
            };

            match state.stream_service.xreadgroup(&group, &consumer, streams, options).await {
                Ok(Some(results)) => {
                    for result in results {
                        for entry in result.entries {
                            let data = serde_json::to_string(&entry).unwrap_or_default();
                            yield Ok(Event::default().event("message").data(data));
                        }
                    }
                }
                Ok(None) => {
                    // No data, continue waiting
                }
                Err(e) => {
                    let error_data = serde_json::json!({ "error": e.to_string() }).to_string();
                    yield Ok(Event::default().event("error").data(error_data));
                    break;
                }
            }

            // Allow cancellation check
            tokio::task::yield_now().await;
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

// ========== Admin-Protected Consumer Group Management ==========

/// POST /api/v1/streams/{key}/groups
///
/// Create a consumer group (XGROUP CREATE). Requires admin authentication.
#[utoipa::path(
    post,
    path = "/api/v1/streams/{key}/groups",
    params(
        ("key" = String, Path, description = "The stream key")
    ),
    request_body = ConsumerGroupCreateRequest,
    responses(
        (status = 200, description = "Group created", body = ConsumerGroupCreateResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("admin_key" = [])
    ),
    tag = "Streams (Admin)"
)]
pub async fn xgroup_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
    Json(req): Json<ConsumerGroupCreateRequest>,
) -> Result<Json<ApiResponse<ConsumerGroupCreateResponse>>, CacheError> {
    verify_admin_key(&headers, &state)?;

    state.stream_service.xgroup_create(&key, &req.group, &req.id, req.to_options()).await?;

    Ok(Json(ApiResponse::success(ConsumerGroupCreateResponse {
        created: true,
    })))
}

/// DELETE /api/v1/streams/{key}/groups/{group}
///
/// Delete a consumer group (XGROUP DESTROY). Requires admin authentication.
#[utoipa::path(
    delete,
    path = "/api/v1/streams/{key}/groups/{group}",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name")
    ),
    responses(
        (status = 200, description = "Group destroyed"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Group not found")
    ),
    security(
        ("admin_key" = [])
    ),
    tag = "Streams (Admin)"
)]
pub async fn xgroup_destroy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((key, group)): Path<(String, String)>,
) -> Result<impl IntoResponse, CacheError> {
    verify_admin_key(&headers, &state)?;

    let destroyed = state.stream_service.xgroup_destroy(&key, &group).await?;

    if destroyed {
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

/// POST /api/v1/streams/{key}/groups/{group}/setid
///
/// Set consumer group last ID (XGROUP SETID). Requires admin authentication.
#[utoipa::path(
    post,
    path = "/api/v1/streams/{key}/groups/{group}/setid",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name")
    ),
    request_body = ConsumerGroupSetIdRequest,
    responses(
        (status = 200, description = "ID set"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("admin_key" = [])
    ),
    tag = "Streams (Admin)"
)]
pub async fn xgroup_setid(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((key, group)): Path<(String, String)>,
    Json(req): Json<ConsumerGroupSetIdRequest>,
) -> Result<impl IntoResponse, CacheError> {
    verify_admin_key(&headers, &state)?;

    state.stream_service.xgroup_setid(&key, &group, &req.id, req.entries_read).await?;

    Ok(StatusCode::OK)
}

/// POST /api/v1/streams/{key}/groups/{group}/consumers
///
/// Create a consumer (XGROUP CREATECONSUMER). Requires admin authentication.
#[utoipa::path(
    post,
    path = "/api/v1/streams/{key}/groups/{group}/consumers",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name")
    ),
    request_body = ConsumerCreateRequest,
    responses(
        (status = 200, description = "Consumer created", body = ConsumerOperationResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("admin_key" = [])
    ),
    tag = "Streams (Admin)"
)]
pub async fn xgroup_createconsumer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((key, group)): Path<(String, String)>,
    Json(req): Json<ConsumerCreateRequest>,
) -> Result<Json<ApiResponse<ConsumerOperationResponse>>, CacheError> {
    verify_admin_key(&headers, &state)?;

    let created =
        state.stream_service.xgroup_createconsumer(&key, &group, &req.consumer).await?;

    Ok(Json(ApiResponse::success(ConsumerOperationResponse {
        result: if created { 1 } else { 0 },
    })))
}

/// DELETE /api/v1/streams/{key}/groups/{group}/consumers/{consumer}
///
/// Delete a consumer (XGROUP DELCONSUMER). Requires admin authentication.
#[utoipa::path(
    delete,
    path = "/api/v1/streams/{key}/groups/{group}/consumers/{consumer}",
    params(
        ("key" = String, Path, description = "The stream key"),
        ("group" = String, Path, description = "The consumer group name"),
        ("consumer" = String, Path, description = "The consumer name")
    ),
    responses(
        (status = 200, description = "Consumer deleted", body = ConsumerOperationResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("admin_key" = [])
    ),
    tag = "Streams (Admin)"
)]
pub async fn xgroup_delconsumer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((key, group, consumer)): Path<(String, String, String)>,
) -> Result<Json<ApiResponse<ConsumerOperationResponse>>, CacheError> {
    verify_admin_key(&headers, &state)?;

    let pending_count =
        state.stream_service.xgroup_delconsumer(&key, &group, &consumer).await?;

    Ok(Json(ApiResponse::success(ConsumerOperationResponse {
        result: pending_count,
    })))
}

/// POST /api/v1/streams/{key}/setid
///
/// Set stream last ID (XSETID). Requires admin authentication.
#[utoipa::path(
    post,
    path = "/api/v1/streams/{key}/setid",
    params(
        ("key" = String, Path, description = "The stream key")
    ),
    request_body = StreamSetIdRequest,
    responses(
        (status = 200, description = "ID set"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("admin_key" = [])
    ),
    tag = "Streams (Admin)"
)]
pub async fn xsetid(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
    Json(req): Json<StreamSetIdRequest>,
) -> Result<impl IntoResponse, CacheError> {
    verify_admin_key(&headers, &state)?;

    state
        .stream_service
        .xsetid(
            &key,
            &req.last_id,
            req.entries_added,
            req.max_deleted_id.as_deref(),
        ).await?;

    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, Bytes, HttpBody};
    use axum::extract::{Path, Query, State};
    use axum::http::Request;
    use futures::future::poll_fn;
    use std::collections::{HashMap, VecDeque};
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use tokio::time::{timeout, Duration};
    use tower::ServiceExt;

    use crate::application::services::{
        AdminService, BloomService, HashService, JsonService, KeyService, ListService, SearchService, SetService,
        SortedSetService, StreamService, StringService,
    };
    use crate::domain::entities::{
        AutoClaimResult, ClaimResult, ConsumerGroupInfo, ConsumerInfo, PendingEntry, PendingSummary,
        StreamEntry, StreamInfo, StreamReadResult, XReadGroupOptions, XReadOptions,
    };
    use crate::domain::repositories::StreamRepository;
    use crate::infrastructure::config::Settings;
    use crate::infrastructure::redis::capabilities::RedisCapabilities;
    use crate::infrastructure::redis::connection::InstrumentedPool;
    use crate::test_support::{
        MockAdminRepository, MockBloomRepository, MockHashRepository, MockJsonRepository, MockKeyRepository, MockListRepository,
        MockSearchRepository, MockSetRepository, MockSortedSetRepository, MockStreamRepository,
        MockStringRepository,
    };

    struct SequenceStreamRepository {
        base: MockStreamRepository,
        xread_results: Mutex<VecDeque<Result<Option<Vec<StreamReadResult>>, CacheError>>>,
        xread_blocking_results: Mutex<VecDeque<Result<Option<Vec<StreamReadResult>>, CacheError>>>,
        xreadgroup_results: Mutex<VecDeque<Result<Option<Vec<StreamReadResult>>, CacheError>>>,
        xreadgroup_blocking_results:
            Mutex<VecDeque<Result<Option<Vec<StreamReadResult>>, CacheError>>>,
        xgroup_destroy_results: Mutex<VecDeque<Result<bool, CacheError>>>,
    }

    impl SequenceStreamRepository {
        fn new() -> Self {
            Self {
                base: MockStreamRepository::new(),
                xread_results: Mutex::new(VecDeque::new()),
                xread_blocking_results: Mutex::new(VecDeque::new()),
                xreadgroup_results: Mutex::new(VecDeque::new()),
                xreadgroup_blocking_results: Mutex::new(VecDeque::new()),
                xgroup_destroy_results: Mutex::new(VecDeque::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl StreamRepository for SequenceStreamRepository {
        async fn xadd(
            &self,
            key: &str,
            fields: &HashMap<String, String>,
            options: crate::domain::entities::XAddOptions,
        ) -> Result<String, CacheError> {
            self.base.xadd(key, fields, options).await
        }

        async fn xlen(&self, key: &str) -> Result<i64, CacheError> {
            self.base.xlen(key).await
        }

        async fn xrange(
            &self,
            key: &str,
            start: &str,
            end: &str,
            count: Option<i64>,
        ) -> Result<Vec<StreamEntry>, CacheError> {
            self.base.xrange(key, start, end, count).await
        }

        async fn xrevrange(
            &self,
            key: &str,
            end: &str,
            start: &str,
            count: Option<i64>,
        ) -> Result<Vec<StreamEntry>, CacheError> {
            self.base.xrevrange(key, end, start, count).await
        }

        async fn xdel(&self, key: &str, ids: &[String]) -> Result<i64, CacheError> {
            self.base.xdel(key, ids).await
        }

        async fn xtrim(
            &self,
            key: &str,
            strategy: crate::domain::entities::XTrimStrategy,
        ) -> Result<i64, CacheError> {
            self.base.xtrim(key, strategy).await
        }

        async fn xinfo_stream(&self, key: &str, full: bool) -> Result<StreamInfo, CacheError> {
            self.base.xinfo_stream(key, full).await
        }

        async fn xread(
            &self,
            streams: &[(String, String)],
            options: XReadOptions,
        ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
            let next = { self.xread_results.lock().expect("xread lock").pop_front() };
            if let Some(result) = next {
                return result;
            }
            self.base.xread(streams, options).await
        }

        async fn xread_blocking(
            &self,
            streams: &[(String, String)],
            count: Option<i64>,
            timeout: Duration,
        ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
            let next = {
                self.xread_blocking_results
                    .lock()
                    .expect("xread_blocking lock")
                    .pop_front()
            };
            if let Some(result) = next {
                return result;
            }
            self.base.xread_blocking(streams, count, timeout).await
        }

        async fn xgroup_create(
            &self,
            key: &str,
            group: &str,
            id: &str,
            options: crate::domain::entities::XGroupCreateOptions,
        ) -> Result<(), CacheError> {
            self.base.xgroup_create(key, group, id, options).await
        }

        async fn xgroup_destroy(&self, key: &str, group: &str) -> Result<bool, CacheError> {
            let next = {
                self.xgroup_destroy_results
                    .lock()
                    .expect("xgroup_destroy lock")
                    .pop_front()
            };
            if let Some(result) = next {
                return result;
            }
            self.base.xgroup_destroy(key, group).await
        }

        async fn xgroup_setid(
            &self,
            key: &str,
            group: &str,
            id: &str,
            entries_read: Option<i64>,
        ) -> Result<(), CacheError> {
            self.base
                .xgroup_setid(key, group, id, entries_read)
                .await
        }

        async fn xgroup_createconsumer(
            &self,
            key: &str,
            group: &str,
            consumer: &str,
        ) -> Result<bool, CacheError> {
            self.base.xgroup_createconsumer(key, group, consumer).await
        }

        async fn xgroup_delconsumer(
            &self,
            key: &str,
            group: &str,
            consumer: &str,
        ) -> Result<i64, CacheError> {
            self.base.xgroup_delconsumer(key, group, consumer).await
        }

        async fn xinfo_groups(&self, key: &str) -> Result<Vec<ConsumerGroupInfo>, CacheError> {
            self.base.xinfo_groups(key).await
        }

        async fn xinfo_consumers(
            &self,
            key: &str,
            group: &str,
        ) -> Result<Vec<ConsumerInfo>, CacheError> {
            self.base.xinfo_consumers(key, group).await
        }

        async fn xreadgroup(
            &self,
            group: &str,
            consumer: &str,
            streams: &[(String, String)],
            options: XReadGroupOptions,
        ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
            let next = {
                self.xreadgroup_results
                    .lock()
                    .expect("xreadgroup lock")
                    .pop_front()
            };
            if let Some(result) = next {
                return result;
            }
            self.base
                .xreadgroup(group, consumer, streams, options)
                .await
        }

        async fn xreadgroup_blocking(
            &self,
            group: &str,
            consumer: &str,
            streams: &[(String, String)],
            count: Option<i64>,
            no_ack: bool,
            timeout: Duration,
        ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
            let next = {
                self.xreadgroup_blocking_results
                    .lock()
                    .expect("xreadgroup_blocking lock")
                    .pop_front()
            };
            if let Some(result) = next {
                return result;
            }
            self.base
                .xreadgroup_blocking(group, consumer, streams, count, no_ack, timeout)
                .await
        }

        async fn xack(
            &self,
            key: &str,
            group: &str,
            ids: &[String],
        ) -> Result<i64, CacheError> {
            self.base.xack(key, group, ids).await
        }

        async fn xpending_summary(
            &self,
            key: &str,
            group: &str,
        ) -> Result<PendingSummary, CacheError> {
            self.base.xpending_summary(key, group).await
        }

        async fn xpending(
            &self,
            key: &str,
            group: &str,
            options: crate::domain::entities::XPendingOptions,
        ) -> Result<Vec<PendingEntry>, CacheError> {
            self.base.xpending(key, group, options).await
        }

        async fn xclaim(
            &self,
            key: &str,
            group: &str,
            consumer: &str,
            ids: &[String],
            options: crate::domain::entities::XClaimOptions,
        ) -> Result<ClaimResult, CacheError> {
            self.base
                .xclaim(key, group, consumer, ids, options)
                .await
        }

        async fn xautoclaim(
            &self,
            key: &str,
            group: &str,
            consumer: &str,
            min_idle_time_ms: i64,
            start: &str,
            options: crate::domain::entities::XAutoClaimOptions,
        ) -> Result<AutoClaimResult, CacheError> {
            self.base
                .xautoclaim(key, group, consumer, min_idle_time_ms, start, options)
                .await
        }

        async fn xsetid(
            &self,
            key: &str,
            last_id: &str,
            entries_added: Option<i64>,
            max_deleted_id: Option<&str>,
        ) -> Result<(), CacheError> {
            self.base
                .xsetid(key, last_id, entries_added, max_deleted_id)
                .await
        }
    }

    fn state_with_stream_repo(stream_repo: Arc<dyn StreamRepository>) -> AppState {
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
        let key_service =
            Arc::new(KeyService::new_with_repository(Arc::new(MockKeyRepository::new())));
        let admin_service =
            Arc::new(AdminService::new_with_repository(Arc::new(MockAdminRepository::default())));
        let stream_service = Arc::new(StreamService::new_with_repository(stream_repo));
        let json_service =
            Arc::new(JsonService::new_with_repository(Arc::new(MockJsonRepository::new())));
        let search_service =
            Arc::new(SearchService::new_with_repository(Arc::new(MockSearchRepository::new())));
        let bloom_service =
            Arc::new(BloomService::new_with_repository(Arc::new(MockBloomRepository::new())));

        AppState::new_with_services(
            pool,
            config,
            capabilities,
            string_service,
            hash_service,
            list_service,
            set_service,
            sorted_set_service,
            key_service,
            admin_service,
            stream_service,
            json_service,
            search_service,
            bloom_service,
        )
    }

    fn sample_read_result(key: &str) -> StreamReadResult {
        let mut fields = HashMap::new();
        fields.insert("field".to_string(), "value".to_string());
        StreamReadResult {
            key: key.to_string(),
            entries: vec![StreamEntry {
                id: "1-0".to_string(),
                fields,
            }],
        }
    }

    async fn next_sse_frame(body: &mut Body) -> Option<Bytes> {
        let frame = timeout(
            Duration::from_secs(1),
            poll_fn(|cx| Pin::new(&mut *body).poll_frame(cx)),
        )
        .await
        .ok()??;
        match frame {
            Ok(frame) => frame.into_data().ok(),
            Err(_) => None,
        }
    }

    #[tokio::test]
    async fn test_stream_routes_basic() {
        let state = state_with_stream_repo(Arc::new(MockStreamRepository::new()));
        let app = stream_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/stream/add")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"fields":{"a":"b"}}"#))
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
                    .uri("/api/v1/streams/stream/length")
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
                    .method("GET")
                    .uri("/api/v1/streams/stream/range?start=-&end=+")
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
                    .method("GET")
                    .uri("/api/v1/streams/stream/revrange?start=-&end=+")
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
                    .method("DELETE")
                    .uri("/api/v1/streams/stream/entries")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"ids":["1-0"]}"#))
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
                    .uri("/api/v1/streams/stream/trim")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"strategy":"maxlen","count":10,"approximate":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/streams/stream/info?full=true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_stream_read_routes() {
        let state = state_with_stream_repo(Arc::new(MockStreamRepository::new()));
        let app = stream_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/read")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"streams":[{"key":"stream","id":"0"}],"count":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/read/blocking")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"streams":[{"key":"stream","id":"0"}],"timeout_seconds":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_stream_read_branches_with_custom_repo() {
        let repo = Arc::new(SequenceStreamRepository::new());
        repo.xread_results
            .lock()
            .expect("xread lock")
            .push_back(Ok(None));
        repo.xread_blocking_results
            .lock()
            .expect("xread_blocking lock")
            .push_back(Ok(Some(vec![])));
        let state = state_with_stream_repo(repo);
        let app = stream_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/read")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"streams":[{"key":"stream","id":"0"}]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/read/blocking")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"streams":[{"key":"stream","id":"0"}],"timeout_seconds":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_stream_group_read_routes() {
        let state = state_with_stream_repo(Arc::new(MockStreamRepository::new()));
        let app = stream_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/streams/stream/groups")
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
                    .method("GET")
                    .uri("/api/v1/streams/stream/groups/group/consumers")
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
                    .uri("/api/v1/streams/stream/groups/group/read")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"consumer":"c1","streams":[{"key":"stream","id":">"}]}"#,
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
                    .uri("/api/v1/streams/stream/groups/group/read/blocking")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"consumer":"c1","streams":[{"key":"stream","id":">"}],"timeout_seconds":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/stream/groups/group/ack")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"ids":["1-0"]}"#))
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
                    .uri("/api/v1/streams/stream/groups/group/pending")
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
                    .method("GET")
                    .uri("/api/v1/streams/stream/groups/group/pending/detail")
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
                    .uri("/api/v1/streams/stream/groups/group/claim")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"consumer":"c1","ids":["1-0"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/stream/groups/group/autoclaim")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"consumer":"c1","min_idle_time_ms":0}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_stream_group_read_empty_streams() {
        let state = state_with_stream_repo(Arc::new(MockStreamRepository::new()));
        let app = stream_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/stream/groups/group/read")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"consumer":"c1","streams":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/stream/groups/group/read/blocking")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"consumer":"c1","streams":[],"timeout_seconds":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_stream_group_read_branches_with_custom_repo() {
        let repo = Arc::new(SequenceStreamRepository::new());
        repo.xreadgroup_results
            .lock()
            .expect("xreadgroup lock")
            .push_back(Ok(None));
        repo.xreadgroup_blocking_results
            .lock()
            .expect("xreadgroup_blocking lock")
            .push_back(Ok(Some(vec![])));
        let state = state_with_stream_repo(repo);
        let app = stream_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/stream/groups/group/read")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"consumer":"c1","streams":[{"key":"stream","id":">"}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/stream/groups/group/read/blocking")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"consumer":"c1","streams":[{"key":"stream","id":">"}],"timeout_seconds":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_stream_admin_routes() {
        let state = state_with_stream_repo(Arc::new(MockStreamRepository::new()));
        let admin_key = state.config.admin.api_key.clone();
        let app = stream_admin_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/stream/groups")
                    .header(ADMIN_API_KEY_HEADER, admin_key.as_str())
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"group":"group"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/streams/stream/groups/group")
                    .header(ADMIN_API_KEY_HEADER, admin_key.as_str())
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
                    .uri("/api/v1/streams/stream/groups/group/setid")
                    .header(ADMIN_API_KEY_HEADER, admin_key.as_str())
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"id":"0"}"#))
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
                    .uri("/api/v1/streams/stream/groups/group/consumers")
                    .header(ADMIN_API_KEY_HEADER, admin_key.as_str())
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"consumer":"c1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/streams/stream/groups/group/consumers/c1")
                    .header(ADMIN_API_KEY_HEADER, admin_key.as_str())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/stream/setid")
                    .header(ADMIN_API_KEY_HEADER, admin_key.as_str())
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"last_id":"1-0"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_stream_admin_routes_unauthorized() {
        let state = state_with_stream_repo(Arc::new(MockStreamRepository::new()));
        let app = stream_admin_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/streams/stream/groups")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"group":"group"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_stream_admin_group_destroy_not_found() {
        let repo = Arc::new(SequenceStreamRepository::new());
        repo.xgroup_destroy_results
            .lock()
            .expect("xgroup_destroy lock")
            .push_back(Ok(false));
        let state = state_with_stream_repo(repo);
        let admin_key = state.config.admin.api_key.clone();
        let app = stream_admin_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/streams/stream/groups/group")
                    .header(ADMIN_API_KEY_HEADER, admin_key.as_str())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_stream_subscribe_sse_message_and_error() {
        let repo = Arc::new(SequenceStreamRepository::new());
        repo.xread_results
            .lock()
            .expect("xread lock")
            .extend([
                Ok(Some(vec![sample_read_result("stream")])),
                Err(CacheError::Internal("boom".to_string())),
            ]);
        let state = state_with_stream_repo(repo);

        let response = stream_subscribe(
            State(state),
            Path("stream".to_string()),
            Query(StreamSubscribeQuery {
                last_id: "0".to_string(),
                count: Some(1),
            }),
        )
        .await
        .into_response();
        let mut body = response.into_body();

        let first = next_sse_frame(&mut body).await.expect("first frame");
        let first_text = String::from_utf8_lossy(&first);
        assert!(first_text.contains("event: message"));

        let second = next_sse_frame(&mut body).await.expect("second frame");
        let second_text = String::from_utf8_lossy(&second);
        assert!(second_text.contains("event: error"));
    }

    #[tokio::test]
    async fn test_stream_subscribe_sse_none_then_error() {
        let repo = Arc::new(SequenceStreamRepository::new());
        repo.xread_results
            .lock()
            .expect("xread lock")
            .extend([
                Ok(None),
                Err(CacheError::Internal("boom".to_string())),
            ]);
        let state = state_with_stream_repo(repo);

        let response = stream_subscribe(
            State(state),
            Path("stream".to_string()),
            Query(StreamSubscribeQuery {
                last_id: "0".to_string(),
                count: Some(1),
            }),
        )
        .await
        .into_response();
        let mut body = response.into_body();

        let frame = next_sse_frame(&mut body).await.expect("error frame");
        let text = String::from_utf8_lossy(&frame);
        assert!(text.contains("event: error"));
    }

    #[tokio::test]
    async fn test_stream_group_subscribe_sse_message_and_error() {
        let repo = Arc::new(SequenceStreamRepository::new());
        repo.xreadgroup_results
            .lock()
            .expect("xreadgroup lock")
            .extend([
                Ok(Some(vec![sample_read_result("stream")])),
                Err(CacheError::Internal("boom".to_string())),
            ]);
        let state = state_with_stream_repo(repo);

        let response = stream_group_subscribe(
            State(state),
            Path(("stream".to_string(), "group".to_string())),
            Query(StreamGroupSubscribeQuery {
                consumer: "c1".to_string(),
                count: Some(1),
                no_ack: false,
            }),
        )
        .await
        .into_response();
        let mut body = response.into_body();

        let first = next_sse_frame(&mut body).await.expect("first frame");
        let first_text = String::from_utf8_lossy(&first);
        assert!(first_text.contains("event: message"));

        let second = next_sse_frame(&mut body).await.expect("second frame");
        let second_text = String::from_utf8_lossy(&second);
        assert!(second_text.contains("event: error"));
    }

    #[tokio::test]
    async fn test_stream_group_subscribe_sse_none_then_error() {
        let repo = Arc::new(SequenceStreamRepository::new());
        repo.xreadgroup_results
            .lock()
            .expect("xreadgroup lock")
            .extend([
                Ok(None),
                Err(CacheError::Internal("boom".to_string())),
            ]);
        let state = state_with_stream_repo(repo);

        let response = stream_group_subscribe(
            State(state),
            Path(("stream".to_string(), "group".to_string())),
            Query(StreamGroupSubscribeQuery {
                consumer: "c1".to_string(),
                count: Some(1),
                no_ack: false,
            }),
        )
        .await
        .into_response();
        let mut body = response.into_body();

        let frame = next_sse_frame(&mut body).await.expect("error frame");
        let text = String::from_utf8_lossy(&frame);
        assert!(text.contains("event: error"));
    }
}
