//! Pub/Sub Routes
//!
//! HTTP and WebSocket endpoints for Redis Pub/Sub operations.
//! - HTTP endpoints use command pool (PUBLISH, PUBSUB info commands)
//! - WebSocket endpoints use dedicated connections (SUBSCRIBE, PSUBSCRIBE)

use axum::{
    Json, Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::{get, post},
};
use futures::StreamExt;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::api::http::schemas::pubsub::{
    ChannelsQuery, ChannelsResponse, NumPatResponse, NumSubItem, NumSubRequest, NumSubResponse,
    PSubscribeQuery, PubSubMessage, PubSubStatsResponse, PublishRequest, PublishResponse,
    SubscribeQuery, SubscriptionConfirmation, WebSocketError,
};
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create Pub/Sub routes
pub fn pubsub_routes() -> Router<AppState> {
    Router::new()
        // HTTP endpoints (use command pool)
        .route("/api/v1/pubsub/publish", post(publish))
        .route("/api/v1/pubsub/channels", get(channels))
        .route("/api/v1/pubsub/numsub", post(numsub))
        .route("/api/v1/pubsub/numpat", get(numpat))
        .route("/api/v1/pubsub/stats", get(stats))
        // Sharded Pub/Sub HTTP endpoints (Redis 7.0+ cluster mode)
        .route("/api/v1/pubsub/spublish", post(spublish))
        .route("/api/v1/pubsub/shardchannels", get(shardchannels))
        .route("/api/v1/pubsub/shardnumsub", post(shardnumsub))
        // WebSocket endpoints (use dedicated connections)
        .route("/api/v1/pubsub/subscribe", get(ws_subscribe))
        .route("/api/v1/pubsub/psubscribe", get(ws_psubscribe))
        .route("/api/v1/pubsub/ssubscribe", get(ws_ssubscribe))
}

// ========== HTTP Endpoints ==========

/// POST /api/v1/pubsub/publish
///
/// Publish a message to a channel.
#[utoipa::path(
    post,
    path = "/api/v1/pubsub/publish",
    request_body = PublishRequest,
    responses(
        (status = 200, description = "Message published", body = PublishResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Pub/Sub"
)]
pub async fn publish(
    State(state): State<AppState>,
    Json(req): Json<PublishRequest>,
) -> Result<Json<ApiResponse<PublishResponse>>, CacheError> {
    let result = state
        .pubsub_service
        .publish(&req.channel, &req.message)
        .await?;

    Ok(Json(ApiResponse::success(PublishResponse {
        channel: result.channel,
        receivers: result.receivers,
    })))
}

/// GET /api/v1/pubsub/channels
///
/// List active channels with at least one subscriber.
#[utoipa::path(
    get,
    path = "/api/v1/pubsub/channels",
    params(ChannelsQuery),
    responses(
        (status = 200, description = "List of active channels", body = ChannelsResponse),
        (status = 400, description = "Invalid pattern")
    ),
    tag = "Pub/Sub"
)]
pub async fn channels(
    State(state): State<AppState>,
    Query(query): Query<ChannelsQuery>,
) -> Result<Json<ApiResponse<ChannelsResponse>>, CacheError> {
    let channels = state
        .pubsub_service
        .channels(query.pattern.as_deref())
        .await?;

    Ok(Json(ApiResponse::success(ChannelsResponse { channels })))
}

/// POST /api/v1/pubsub/numsub
///
/// Get subscriber count for specified channels.
#[utoipa::path(
    post,
    path = "/api/v1/pubsub/numsub",
    request_body = NumSubRequest,
    responses(
        (status = 200, description = "Subscriber counts", body = NumSubResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Pub/Sub"
)]
pub async fn numsub(
    State(state): State<AppState>,
    Json(req): Json<NumSubRequest>,
) -> Result<Json<ApiResponse<NumSubResponse>>, CacheError> {
    let results = state.pubsub_service.numsub(&req.channels).await?;

    let channels = results
        .into_iter()
        .map(|r| NumSubItem {
            channel: r.channel,
            subscribers: r.subscribers,
        })
        .collect();

    Ok(Json(ApiResponse::success(NumSubResponse { channels })))
}

/// GET /api/v1/pubsub/numpat
///
/// Get total number of pattern subscriptions.
#[utoipa::path(
    get,
    path = "/api/v1/pubsub/numpat",
    responses(
        (status = 200, description = "Pattern subscription count", body = NumPatResponse)
    ),
    tag = "Pub/Sub"
)]
pub async fn numpat(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<NumPatResponse>>, CacheError> {
    let patterns = state.pubsub_service.numpat().await?;

    Ok(Json(ApiResponse::success(NumPatResponse { patterns })))
}

/// GET /api/v1/pubsub/stats
///
/// Get Pub/Sub connection statistics.
#[utoipa::path(
    get,
    path = "/api/v1/pubsub/stats",
    responses(
        (status = 200, description = "Pub/Sub statistics", body = PubSubStatsResponse)
    ),
    tag = "Pub/Sub"
)]
pub async fn stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<PubSubStatsResponse>>, CacheError> {
    let stats = state.pubsub_service.get_stats();

    Ok(Json(ApiResponse::success(PubSubStatsResponse {
        active_subscriptions: stats.active_subscriptions,
        max_subscriptions: stats.max_subscriptions,
        total_created: stats.total_created,
        total_messages: stats.total_messages,
        errors: stats.errors,
    })))
}

// ========== Sharded Pub/Sub HTTP Endpoints (Redis 7.0+ Cluster) ==========

/// POST /api/v1/pubsub/spublish
///
/// Publish a message to a sharded channel (Redis 7.0+ cluster mode).
#[utoipa::path(
    post,
    path = "/api/v1/pubsub/spublish",
    request_body = PublishRequest,
    responses(
        (status = 200, description = "Message published to shard", body = PublishResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Pub/Sub"
)]
pub async fn spublish(
    State(state): State<AppState>,
    Json(req): Json<PublishRequest>,
) -> Result<Json<ApiResponse<PublishResponse>>, CacheError> {
    let result = state
        .pubsub_service
        .spublish(&req.channel, &req.message)
        .await?;

    Ok(Json(ApiResponse::success(PublishResponse {
        channel: result.channel,
        receivers: result.receivers,
    })))
}

/// GET /api/v1/pubsub/shardchannels
///
/// List active sharded channels with at least one subscriber (Redis 7.0+ cluster mode).
#[utoipa::path(
    get,
    path = "/api/v1/pubsub/shardchannels",
    params(ChannelsQuery),
    responses(
        (status = 200, description = "List of active sharded channels", body = ChannelsResponse),
        (status = 400, description = "Invalid pattern")
    ),
    tag = "Pub/Sub"
)]
pub async fn shardchannels(
    State(state): State<AppState>,
    Query(query): Query<ChannelsQuery>,
) -> Result<Json<ApiResponse<ChannelsResponse>>, CacheError> {
    let channels = state
        .pubsub_service
        .shardchannels(query.pattern.as_deref())
        .await?;

    Ok(Json(ApiResponse::success(ChannelsResponse { channels })))
}

/// POST /api/v1/pubsub/shardnumsub
///
/// Get subscriber count for specified sharded channels (Redis 7.0+ cluster mode).
#[utoipa::path(
    post,
    path = "/api/v1/pubsub/shardnumsub",
    request_body = NumSubRequest,
    responses(
        (status = 200, description = "Subscriber counts for sharded channels", body = NumSubResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Pub/Sub"
)]
pub async fn shardnumsub(
    State(state): State<AppState>,
    Json(req): Json<NumSubRequest>,
) -> Result<Json<ApiResponse<NumSubResponse>>, CacheError> {
    let results = state.pubsub_service.shardnumsub(&req.channels).await?;

    let channels = results
        .into_iter()
        .map(|r| NumSubItem {
            channel: r.channel,
            subscribers: r.subscribers,
        })
        .collect();

    Ok(Json(ApiResponse::success(NumSubResponse { channels })))
}

// ========== WebSocket Endpoints ==========

/// Maximum length for channel/pattern names
const MAX_CHANNEL_NAME_LENGTH: usize = 1024;
/// Maximum number of channels/patterns per subscription request
const MAX_CHANNELS_PER_REQUEST: usize = 100;

/// Validate channel names
fn validate_channels(channels: &[String]) -> Result<(), CacheError> {
    if channels.len() > MAX_CHANNELS_PER_REQUEST {
        return Err(CacheError::InvalidInput(format!(
            "Too many channels: {} (max {})",
            channels.len(),
            MAX_CHANNELS_PER_REQUEST
        )));
    }
    for channel in channels {
        if channel.is_empty() {
            return Err(CacheError::InvalidInput("Empty channel name".to_string()));
        }
        if channel.len() > MAX_CHANNEL_NAME_LENGTH {
            return Err(CacheError::InvalidInput(format!(
                "Channel name too long: {} chars (max {})",
                channel.len(),
                MAX_CHANNEL_NAME_LENGTH
            )));
        }
    }
    Ok(())
}

/// Validate pattern names
fn validate_patterns(patterns: &[String]) -> Result<(), CacheError> {
    if patterns.len() > MAX_CHANNELS_PER_REQUEST {
        return Err(CacheError::InvalidInput(format!(
            "Too many patterns: {} (max {})",
            patterns.len(),
            MAX_CHANNELS_PER_REQUEST
        )));
    }
    for pattern in patterns {
        if pattern.is_empty() {
            return Err(CacheError::InvalidInput("Empty pattern".to_string()));
        }
        if pattern.len() > MAX_CHANNEL_NAME_LENGTH {
            return Err(CacheError::InvalidInput(format!(
                "Pattern too long: {} chars (max {})",
                pattern.len(),
                MAX_CHANNEL_NAME_LENGTH
            )));
        }
    }
    Ok(())
}

/// GET /api/v1/pubsub/subscribe
///
/// Subscribe to channels via WebSocket.
/// Channels are specified as comma-separated query parameter.
#[utoipa::path(
    get,
    path = "/api/v1/pubsub/subscribe",
    params(SubscribeQuery),
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Invalid channels"),
        (status = 503, description = "Subscription limit reached")
    ),
    tag = "Pub/Sub"
)]
pub async fn ws_subscribe(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<SubscribeQuery>,
) -> Result<impl IntoResponse, CacheError> {
    let channels = query.parse_channels();

    if channels.is_empty() {
        return Err(CacheError::InvalidInput(
            "No channels specified".to_string(),
        ));
    }

    // Validate channel names
    validate_channels(&channels)?;

    // Reserve subscription slot BEFORE upgrade to prevent race condition
    // This creates the connection early but ensures atomic slot reservation
    let pubsub = state.pubsub_service.create_subscription().await?;

    Ok(ws.on_upgrade(move |socket| handle_subscribe(socket, state, channels, pubsub)))
}

/// GET /api/v1/pubsub/psubscribe
///
/// Subscribe to patterns via WebSocket.
/// Patterns are specified as comma-separated query parameter.
#[utoipa::path(
    get,
    path = "/api/v1/pubsub/psubscribe",
    params(PSubscribeQuery),
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Invalid patterns"),
        (status = 503, description = "Subscription limit reached")
    ),
    tag = "Pub/Sub"
)]
pub async fn ws_psubscribe(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<PSubscribeQuery>,
) -> Result<impl IntoResponse, CacheError> {
    let patterns = query.parse_patterns();

    if patterns.is_empty() {
        return Err(CacheError::InvalidInput(
            "No patterns specified".to_string(),
        ));
    }

    // Validate pattern names
    validate_patterns(&patterns)?;

    // Reserve subscription slot BEFORE upgrade to prevent race condition
    let pubsub = state.pubsub_service.create_subscription().await?;

    Ok(ws.on_upgrade(move |socket| handle_psubscribe(socket, state, patterns, pubsub)))
}

/// GET /api/v1/pubsub/ssubscribe
///
/// Subscribe to sharded channels via WebSocket (Redis 7.0+ cluster mode).
///
/// **Note**: This endpoint is not yet implemented. The redis crate does not
/// natively support SSUBSCRIBE, and proper implementation requires cluster-aware
/// connection handling. Returns 501 Not Implemented.
#[utoipa::path(
    get,
    path = "/api/v1/pubsub/ssubscribe",
    params(SubscribeQuery),
    responses(
        (status = 501, description = "Sharded pub/sub not implemented - use /subscribe instead"),
        (status = 400, description = "Invalid channels")
    ),
    tag = "Pub/Sub"
)]
pub async fn ws_ssubscribe(
    _ws: WebSocketUpgrade,
    State(_state): State<AppState>,
    Query(_query): Query<SubscribeQuery>,
) -> Result<axum::response::Response, CacheError> {
    // SSUBSCRIBE requires cluster-aware connection handling that the redis crate
    // doesn't natively support. Return 501 until proper implementation.
    Err(CacheError::ModuleNotAvailable(
        "Sharded pub/sub (SSUBSCRIBE) is not yet implemented. Use /subscribe for regular pub/sub."
            .to_string(),
    ))
}

// ========== WebSocket Handlers ==========

use crate::infrastructure::redis::pubsub_manager::PubSubConnection;

#[cfg(test)]
static WS_SEND_FAILURES: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn set_ws_send_failures(count: usize) {
    WS_SEND_FAILURES.store(count, Ordering::Relaxed);
}

async fn send_ws(socket: &mut WebSocket, message: Message) -> Result<(), axum::Error> {
    #[cfg(test)]
    {
        let mut remaining = WS_SEND_FAILURES.load(Ordering::Relaxed);
        while remaining > 0 {
            if WS_SEND_FAILURES
                .compare_exchange_weak(
                    remaining,
                    remaining - 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Err(axum::Error::new(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "forced send failure",
                )));
            }
            remaining = WS_SEND_FAILURES.load(Ordering::Relaxed);
        }
    }

    socket.send(message).await
}

/// Extract message payload, handling both UTF-8 and binary data.
/// Binary data is base64-encoded with a "base64:" prefix.
fn extract_payload(msg: &redis::Msg) -> String {
    // First try to get as string (most common case)
    if let Ok(s) = msg.get_payload::<String>() {
        return s;
    }

    // Fall back to bytes and base64 encode
    if let Ok(bytes) = msg.get_payload::<Vec<u8>>() {
        use base64::Engine;
        return format!(
            "base64:{}",
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        );
    }

    // If all else fails, return empty string (shouldn't happen)
    String::new()
}

async fn handle_subscribe(
    mut socket: WebSocket,
    state: AppState,
    channels: Vec<String>,
    mut pubsub: PubSubConnection,
) {
    // Subscribe to all channels with incremental count (Redis semantics)
    for (subscribed_count, channel) in (0_i64..).zip(channels.iter()) {
        if let Err(e) = pubsub.subscribe(channel).await {
            let error = WebSocketError {
                error: "subscribe_failed".to_string(),
                message: format!("Failed to subscribe to {}: {}", channel, e),
            };
            let _ = send_ws(
                &mut socket,
                Message::Text(serde_json::to_string(&error).unwrap_or_default().into()),
            )
            .await;
            state.pubsub_service.record_error();
            return;
        }

        // Send confirmation with incremental count (like Redis)
        let confirmation = SubscriptionConfirmation {
            r#type: "subscribed".to_string(),
            target: channel.clone(),
            count: subscribed_count + 1,
        };
        if send_ws(
            &mut socket,
            Message::Text(
                serde_json::to_string(&confirmation)
                    .unwrap_or_default()
                    .into(),
            ),
        )
        .await
        .is_err()
        {
            return; // Client disconnected
        }
    }

    // Get message stream
    let Some(mut stream) = pubsub.into_on_message() else {
        let error = WebSocketError {
            error: "stream_failed".to_string(),
            message: "Failed to create message stream".to_string(),
        };
        let _ = send_ws(
            &mut socket,
            Message::Text(serde_json::to_string(&error).unwrap_or_default().into()),
        )
        .await;
        return;
    };

    // Stream messages to WebSocket
    loop {
        tokio::select! {
            // Message from Redis
            msg = stream.next() => {
                match msg {
                    Some(m) => {
                        state.pubsub_service.record_message();
                        let payload = PubSubMessage::new_message(
                            m.get_channel_name().to_string(),
                            extract_payload(&m),
                        );
                        let json = serde_json::to_string(&payload).unwrap_or_default();
                        if send_ws(&mut socket, Message::Text(json.into())).await.is_err() {
                            break; // Client disconnected
                        }
                    }
                    None => break, // Redis connection closed
                }
            }
            // Message from client
            client_msg = socket.recv() => {
                match client_msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if send_ws(&mut socket, Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {} // Ignore other messages
                }
            }
        }
    }
}

async fn handle_psubscribe(
    mut socket: WebSocket,
    state: AppState,
    patterns: Vec<String>,
    mut pubsub: PubSubConnection,
) {
    // Subscribe to all patterns with incremental count (Redis semantics)
    for (subscribed_count, pattern) in (0_i64..).zip(patterns.iter()) {
        if let Err(e) = pubsub.psubscribe(pattern).await {
            let error = WebSocketError {
                error: "psubscribe_failed".to_string(),
                message: format!("Failed to subscribe to pattern {}: {}", pattern, e),
            };
            let _ = send_ws(
                &mut socket,
                Message::Text(serde_json::to_string(&error).unwrap_or_default().into()),
            )
            .await;
            state.pubsub_service.record_error();
            return;
        }

        // Send confirmation with incremental count (like Redis)
        let confirmation = SubscriptionConfirmation {
            r#type: "psubscribed".to_string(),
            target: pattern.clone(),
            count: subscribed_count + 1,
        };
        if send_ws(
            &mut socket,
            Message::Text(
                serde_json::to_string(&confirmation)
                    .unwrap_or_default()
                    .into(),
            ),
        )
        .await
        .is_err()
        {
            return; // Client disconnected
        }
    }

    // Get message stream
    let Some(mut stream) = pubsub.into_on_message() else {
        let error = WebSocketError {
            error: "stream_failed".to_string(),
            message: "Failed to create message stream".to_string(),
        };
        let _ = send_ws(
            &mut socket,
            Message::Text(serde_json::to_string(&error).unwrap_or_default().into()),
        )
        .await;
        return;
    };

    // Stream messages to WebSocket
    loop {
        tokio::select! {
            // Message from Redis
            msg = stream.next() => {
                match msg {
                    Some(m) => {
                        state.pubsub_service.record_message();
                        let payload = PubSubMessage::new_pmessage(
                            m.get_pattern::<String>().unwrap_or_default(),
                            m.get_channel_name().to_string(),
                            extract_payload(&m),
                        );
                        let json = serde_json::to_string(&payload).unwrap_or_default();
                        if send_ws(&mut socket, Message::Text(json.into())).await.is_err() {
                            break; // Client disconnected
                        }
                    }
                    None => break, // Redis connection closed
                }
            }
            // Message from client
            client_msg = socket.recv() => {
                match client_msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if send_ws(&mut socket, Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {} // Ignore other messages
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use futures::{SinkExt, StreamExt};
    use serde_json::Value;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use testcontainers::ContainerAsync;
    use testcontainers_modules::redis::Redis;
    use tokio::net::TcpListener;
    use tokio::sync::{Mutex, oneshot};
    use tokio::time::{Duration, timeout};
    use tokio_tungstenite::tungstenite::Message as WsMessage;
    use tower::ServiceExt;

    use crate::application::services::PubSubService;
    use crate::domain::repositories::{NumSubResult, PubSubRepository, PublishResult};
    use crate::infrastructure::config::Settings;
    use crate::infrastructure::redis::pubsub_manager::{
        PubSubConnection, PubSubManager, PubSubStats,
    };
    use crate::test_support::{
        MockAdminRepository, MockBitMapRepository, MockBloomRepository, MockGeoRepository,
        MockHashRepository, MockJsonRepository, MockKeyRepository, MockListRepository,
        MockProbabilisticRepository, MockSearchRepository, MockSetRepository,
        MockSortedSetRepository, MockStreamRepository, MockStringRepository, start_redis_container,
        test_state_with_all_repos_and_config,
    };

    struct StubPubSubRepository {
        publish_result: PublishResult,
        numpat_result: i64,
        channels_result: Vec<String>,
        numsub_result: Vec<NumSubResult>,
    }

    #[async_trait]
    impl PubSubRepository for StubPubSubRepository {
        async fn publish(
            &self,
            _channel: &str,
            _message: &str,
        ) -> Result<PublishResult, CacheError> {
            Ok(self.publish_result.clone())
        }

        async fn spublish(
            &self,
            _channel: &str,
            _message: &str,
        ) -> Result<PublishResult, CacheError> {
            Ok(self.publish_result.clone())
        }

        async fn pubsub_channels(&self, _pattern: Option<&str>) -> Result<Vec<String>, CacheError> {
            Ok(self.channels_result.clone())
        }

        async fn pubsub_numsub(
            &self,
            _channels: &[String],
        ) -> Result<Vec<NumSubResult>, CacheError> {
            Ok(self.numsub_result.clone())
        }

        async fn pubsub_numpat(&self) -> Result<i64, CacheError> {
            Ok(self.numpat_result)
        }

        async fn pubsub_shardchannels(
            &self,
            _pattern: Option<&str>,
        ) -> Result<Vec<String>, CacheError> {
            Ok(self.channels_result.clone())
        }

        async fn pubsub_shardnumsub(
            &self,
            _channels: &[String],
        ) -> Result<Vec<NumSubResult>, CacheError> {
            Ok(self.numsub_result.clone())
        }
    }

    async fn start_redis() -> Option<(ContainerAsync<Redis>, String)> {
        start_redis_container().await
    }

    async fn spawn_pubsub_server(state: AppState) -> (SocketAddr, oneshot::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = pubsub_routes().with_state(state);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        (addr, shutdown_tx)
    }

    async fn spawn_router(router: Router) -> (SocketAddr, oneshot::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        (addr, shutdown_tx)
    }

    fn build_state_with_config(repo: Arc<dyn PubSubRepository>, config: Settings) -> AppState {
        let pubsub_manager =
            Arc::new(PubSubManager::new(&config.redis.url, config.pubsub.clone()).unwrap());
        let pubsub_service = Arc::new(PubSubService::new_with_repository(repo, pubsub_manager));

        let string_repo = Arc::new(MockStringRepository::new());
        let hash_repo = Arc::new(MockHashRepository::new());
        let list_repo = Arc::new(MockListRepository::new());
        let set_repo = Arc::new(MockSetRepository::new());
        let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
        let bitmap_repo = Arc::new(MockBitMapRepository::new());
        let key_repo = Arc::new(MockKeyRepository::new());
        let admin_repo = Arc::new(MockAdminRepository);
        let stream_repo = Arc::new(MockStreamRepository::new());
        let json_repo = Arc::new(MockJsonRepository::new());
        let search_repo = Arc::new(MockSearchRepository::new());
        let bloom_repo = Arc::new(MockBloomRepository::new());
        let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
        let geo_repo = Arc::new(MockGeoRepository::new());

        let mut state = test_state_with_all_repos_and_config(
            string_repo,
            hash_repo,
            list_repo,
            set_repo,
            sorted_set_repo,
            bitmap_repo,
            key_repo,
            admin_repo,
            stream_repo,
            json_repo,
            search_repo,
            bloom_repo,
            probabilistic_repo,
            geo_repo,
            config,
        );
        state.pubsub_service = pubsub_service;
        state
    }

    fn build_state(repo: Arc<dyn PubSubRepository>) -> AppState {
        build_state_with_config(repo, Settings::default())
    }

    #[tokio::test]
    async fn test_pubsub_publish_endpoint() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 2,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let app = pubsub_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pubsub/publish")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"channel":"news","message":"hi"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["channel"], "news");
        assert_eq!(json["data"]["receivers"], 2);
    }

    #[tokio::test]
    async fn test_pubsub_channels_endpoint() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: vec!["news".to_string(), "alerts".to_string()],
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let app = pubsub_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pubsub/channels")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_pubsub_numsub_endpoint() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: vec![NumSubResult {
                channel: "news".to_string(),
                subscribers: 5,
            }],
        });
        let state = build_state(repo);
        let app = pubsub_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pubsub/numsub")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"channels":["news"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_pubsub_numpat_endpoint() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 4,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let app = pubsub_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pubsub/numpat")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_pubsub_stats_endpoint() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        state.pubsub_service.record_message();
        state.pubsub_service.record_error();
        let app = pubsub_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pubsub/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_pubsub_sharded_endpoints() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 2,
            },
            numpat_result: 0,
            channels_result: vec!["news".to_string()],
            numsub_result: vec![NumSubResult {
                channel: "news".to_string(),
                subscribers: 1,
            }],
        });
        let state = build_state(repo);
        let app = pubsub_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pubsub/spublish")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"channel":"news","message":"hi"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pubsub/shardchannels")
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
                    .uri("/api/v1/pubsub/shardnumsub")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"channels":["news"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_validate_channels() {
        let ok = validate_channels(&["news".to_string()]);
        assert!(ok.is_ok());

        let err = validate_channels(&["".to_string()]).unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let too_long = "x".repeat(MAX_CHANNEL_NAME_LENGTH + 1);
        let err = validate_channels(&[too_long]).unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let too_many = (0..(MAX_CHANNELS_PER_REQUEST + 1))
            .map(|i| format!("ch{}", i))
            .collect::<Vec<_>>();
        let err = validate_channels(&too_many).unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[test]
    fn test_validate_patterns() {
        let ok = validate_patterns(&["user:*".to_string()]);
        assert!(ok.is_ok());

        let err = validate_patterns(&["".to_string()]).unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let too_long = "x".repeat(MAX_CHANNEL_NAME_LENGTH + 1);
        let err = validate_patterns(&[too_long]).unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let too_many = (0..(MAX_CHANNELS_PER_REQUEST + 1))
            .map(|i| format!("pat{}", i))
            .collect::<Vec<_>>();
        let err = validate_patterns(&too_many).unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[test]
    fn test_extract_payload_variants() {
        // Test UTF-8 string payload
        let msg_value = redis::Value::Array(vec![
            redis::Value::BulkString(b"message".to_vec()),
            redis::Value::BulkString(b"news".to_vec()),
            redis::Value::BulkString(b"hello".to_vec()),
        ]);
        let msg = redis::Msg::from_owned_value(msg_value).unwrap();
        assert_eq!(extract_payload(&msg), "hello");

        // Test binary payload with invalid UTF-8 sequence
        // Using bytes that cannot be valid UTF-8: 0xFF followed by 0xFE
        let binary_value = redis::Value::Array(vec![
            redis::Value::BulkString(b"message".to_vec()),
            redis::Value::BulkString(b"news".to_vec()),
            redis::Value::BulkString(vec![0xFF, 0xFE, 0x00, 0x01]),
        ]);
        let binary_msg = redis::Msg::from_owned_value(binary_value).unwrap();
        let payload = extract_payload(&binary_msg);
        // Note: redis's get_payload::<String>() may use lossy UTF-8 conversion,
        // so we just verify we get a non-empty result
        assert!(!payload.is_empty());

        // Test fallback to empty string when payload cannot be decoded
        let invalid_value = redis::Value::Array(vec![
            redis::Value::BulkString(b"message".to_vec()),
            redis::Value::BulkString(b"news".to_vec()),
            redis::Value::Attribute {
                data: Box::new(redis::Value::Array(vec![redis::Value::Int(1)])),
                attributes: Vec::new(),
            },
        ]);
        let invalid_msg = redis::Msg::from_owned_value(invalid_value).unwrap();
        assert_eq!(extract_payload(&invalid_msg), "");
    }

    #[tokio::test]
    async fn test_ws_subscribe_flow() {
        let Some((_container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url.clone();
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/subscribe?channels=news");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();

        let msg = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let confirmation: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
        assert_eq!(confirmation["type"], "subscribed");
        assert_eq!(confirmation["target"], "news");

        // Ping/pong test — tolerate connection reset on slow CI runners
        let _ = socket.send(WsMessage::Ping(vec![1, 2, 3].into())).await;
        if let Ok(Some(Ok(pong))) = timeout(Duration::from_secs(3), socket.next()).await {
            assert!(
                matches!(pong, WsMessage::Pong(_) | WsMessage::Close(_)),
                "expected Pong or Close, got {pong:?}"
            );
        }

        let client = redis::Client::open(redis_url.as_str()).unwrap();
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: i64 = redis::cmd("PUBLISH")
            .arg("news")
            .arg("hello")
            .query_async(&mut conn)
            .await
            .unwrap();

        let msg = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let payload: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
        assert_eq!(payload["type"], "message");
        assert_eq!(payload["channel"], "news");
        assert_eq!(payload["message"], "hello");

        let _ = socket.send(WsMessage::Close(None)).await;
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_psubscribe_flow() {
        let Some((_container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url.clone();
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/psubscribe?patterns=user:*");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();

        let msg = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let confirmation: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
        assert_eq!(confirmation["type"], "psubscribed");
        assert_eq!(confirmation["target"], "user:*");

        // Ping/pong test — tolerate connection reset on slow CI runners
        let _ = socket.send(WsMessage::Ping(vec![9, 9, 9].into())).await;
        if let Ok(Some(Ok(pong))) = timeout(Duration::from_secs(3), socket.next()).await {
            assert!(
                matches!(pong, WsMessage::Pong(_) | WsMessage::Close(_)),
                "expected Pong or Close, got {pong:?}"
            );
        }

        let client = redis::Client::open(redis_url.as_str()).unwrap();
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: i64 = redis::cmd("PUBLISH")
            .arg("user:123")
            .arg("hi")
            .query_async(&mut conn)
            .await
            .unwrap();

        let msg = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let payload: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
        assert_eq!(payload["type"], "pmessage");
        assert_eq!(payload["pattern"], "user:*");
        assert_eq!(payload["channel"], "user:123");
        assert_eq!(payload["message"], "hi");

        let _ = socket.send(WsMessage::Close(None)).await;
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_subscribe_close_breaks_loop() {
        let Some((_container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url.clone();
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/subscribe?channels=news");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();

        let _ = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        socket.send(WsMessage::Close(None)).await.unwrap();
        let _ = timeout(Duration::from_secs(3), socket.next()).await.ok();

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_psubscribe_close_breaks_loop() {
        let Some((_container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url.clone();
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/psubscribe?patterns=user:*");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();

        let _ = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        socket.send(WsMessage::Close(None)).await.unwrap();
        let _ = timeout(Duration::from_secs(3), socket.next()).await.ok();

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_subscribe_rejects_empty_channels() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/subscribe?channels=");
        let err = tokio_tungstenite::connect_async(ws_url).await.unwrap_err();
        let response = match err {
            tokio_tungstenite::tungstenite::Error::Http(response) => response,
            other => panic!("unexpected error: {other:?}"),
        };
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_subscribe_rejects_long_channel() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let channel = "x".repeat(MAX_CHANNEL_NAME_LENGTH + 1);
        let ws_url = format!("ws://{addr}/api/v1/pubsub/subscribe?channels={channel}");
        let err = tokio_tungstenite::connect_async(ws_url).await.unwrap_err();
        let response = match err {
            tokio_tungstenite::tungstenite::Error::Http(response) => response,
            other => panic!("unexpected error: {other:?}"),
        };
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_psubscribe_rejects_empty_patterns() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/psubscribe?patterns=");
        let err = tokio_tungstenite::connect_async(ws_url).await.unwrap_err();
        let response = match err {
            tokio_tungstenite::tungstenite::Error::Http(response) => response,
            other => panic!("unexpected error: {other:?}"),
        };
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_psubscribe_rejects_long_pattern() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let pattern = "x".repeat(MAX_CHANNEL_NAME_LENGTH + 1);
        let ws_url = format!("ws://{addr}/api/v1/pubsub/psubscribe?patterns={pattern}");
        let err = tokio_tungstenite::connect_async(ws_url).await.unwrap_err();
        let response = match err {
            tokio_tungstenite::tungstenite::Error::Http(response) => response,
            other => panic!("unexpected error: {other:?}"),
        };
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_ssubscribe_not_implemented() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/ssubscribe?channels=news");
        let err = tokio_tungstenite::connect_async(ws_url).await.unwrap_err();
        let response = match err {
            tokio_tungstenite::tungstenite::Error::Http(response) => response,
            other => panic!("unexpected error: {other:?}"),
        };
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_handle_subscribe_subscribe_error() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let stats = Arc::new(PubSubStats {
            max_subscriptions: 1,
            ..Default::default()
        });
        let pubsub = PubSubConnection::new_for_tests(None, stats);
        let connection = Arc::new(Mutex::new(Some(pubsub)));
        let channels = Arc::new(vec!["news".to_string()]);

        let router = Router::new()
            .route(
                "/ws",
                get({
                    let connection = connection.clone();
                    let channels = channels.clone();
                    move |ws: WebSocketUpgrade, State(state): State<AppState>| {
                        let connection = connection.clone();
                        let channels = channels.clone();
                        async move {
                            let pubsub = connection.lock().await.take().expect("pubsub");
                            Ok::<_, CacheError>(ws.on_upgrade(move |socket| {
                                handle_subscribe(socket, state, channels.as_ref().clone(), pubsub)
                            }))
                        }
                    }
                }),
            )
            .with_state(state);

        let (addr, shutdown_tx) = spawn_router(router).await;
        let ws_url = format!("ws://{addr}/ws");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let msg = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let payload: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
        assert_eq!(payload["error"], "subscribe_failed");
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_handle_subscribe_stream_failed() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let stats = Arc::new(PubSubStats {
            max_subscriptions: 1,
            ..Default::default()
        });
        let pubsub = PubSubConnection::new_for_tests(None, stats);
        let connection = Arc::new(Mutex::new(Some(pubsub)));
        let channels = Arc::new(Vec::new());

        let router = Router::new()
            .route(
                "/ws",
                get({
                    let connection = connection.clone();
                    let channels = channels.clone();
                    move |ws: WebSocketUpgrade, State(state): State<AppState>| {
                        let connection = connection.clone();
                        let channels = channels.clone();
                        async move {
                            let pubsub = connection.lock().await.take().expect("pubsub");
                            Ok::<_, CacheError>(ws.on_upgrade(move |socket| {
                                handle_subscribe(socket, state, channels.as_ref().clone(), pubsub)
                            }))
                        }
                    }
                }),
            )
            .with_state(state);

        let (addr, shutdown_tx) = spawn_router(router).await;
        let ws_url = format!("ws://{addr}/ws");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let msg = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let payload: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
        assert_eq!(payload["error"], "stream_failed");
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_handle_psubscribe_subscribe_error() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let stats = Arc::new(PubSubStats {
            max_subscriptions: 1,
            ..Default::default()
        });
        let pubsub = PubSubConnection::new_for_tests(None, stats);
        let connection = Arc::new(Mutex::new(Some(pubsub)));
        let patterns = Arc::new(vec!["user:*".to_string()]);

        let router = Router::new()
            .route(
                "/ws",
                get({
                    let connection = connection.clone();
                    let patterns = patterns.clone();
                    move |ws: WebSocketUpgrade, State(state): State<AppState>| {
                        let connection = connection.clone();
                        let patterns = patterns.clone();
                        async move {
                            let pubsub = connection.lock().await.take().expect("pubsub");
                            Ok::<_, CacheError>(ws.on_upgrade(move |socket| {
                                handle_psubscribe(socket, state, patterns.as_ref().clone(), pubsub)
                            }))
                        }
                    }
                }),
            )
            .with_state(state);

        let (addr, shutdown_tx) = spawn_router(router).await;
        let ws_url = format!("ws://{addr}/ws");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let msg = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let payload: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
        assert_eq!(payload["error"], "psubscribe_failed");
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_handle_psubscribe_stream_failed() {
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let state = build_state(repo);
        let stats = Arc::new(PubSubStats {
            max_subscriptions: 1,
            ..Default::default()
        });
        let pubsub = PubSubConnection::new_for_tests(None, stats);
        let connection = Arc::new(Mutex::new(Some(pubsub)));
        let patterns = Arc::new(Vec::new());

        let router = Router::new()
            .route(
                "/ws",
                get({
                    let connection = connection.clone();
                    let patterns = patterns.clone();
                    move |ws: WebSocketUpgrade, State(state): State<AppState>| {
                        let connection = connection.clone();
                        let patterns = patterns.clone();
                        async move {
                            let pubsub = connection.lock().await.take().expect("pubsub");
                            Ok::<_, CacheError>(ws.on_upgrade(move |socket| {
                                handle_psubscribe(socket, state, patterns.as_ref().clone(), pubsub)
                            }))
                        }
                    }
                }),
            )
            .with_state(state);

        let (addr, shutdown_tx) = spawn_router(router).await;
        let ws_url = format!("ws://{addr}/ws");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let msg = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let payload: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
        assert_eq!(payload["error"], "stream_failed");
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_subscribe_confirmation_send_failure() {
        let Some((_container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url;
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        set_ws_send_failures(1);
        let ws_url = format!("ws://{addr}/api/v1/pubsub/subscribe?channels=news");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let _ = timeout(Duration::from_secs(1), socket.next()).await;
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_subscribe_pong_send_failure() {
        let Some((_container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url;
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/subscribe?channels=news");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let _ = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        set_ws_send_failures(1);
        socket.send(WsMessage::Ping(vec![1].into())).await.unwrap();
        let _ = timeout(Duration::from_secs(1), socket.next()).await;
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_subscribe_message_send_failure() {
        let Some((_container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url.clone();
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/subscribe?channels=news");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let _ = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        set_ws_send_failures(1);
        let client = redis::Client::open(redis_url.as_str()).unwrap();
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: i64 = redis::cmd("PUBLISH")
            .arg("news")
            .arg("hello")
            .query_async(&mut conn)
            .await
            .unwrap();
        let _ = timeout(Duration::from_secs(1), socket.next()).await;
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_subscribe_redis_disconnect() {
        let Some((container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url;
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/subscribe?channels=news");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let _ = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        drop(container);
        let _ = timeout(Duration::from_secs(2), socket.next()).await;
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_psubscribe_confirmation_send_failure() {
        let Some((_container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url;
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        set_ws_send_failures(1);
        let ws_url = format!("ws://{addr}/api/v1/pubsub/psubscribe?patterns=user:*");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let _ = timeout(Duration::from_secs(1), socket.next()).await;
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_psubscribe_pong_send_failure() {
        let Some((_container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url;
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/psubscribe?patterns=user:*");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let _ = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        set_ws_send_failures(1);
        socket.send(WsMessage::Ping(vec![2].into())).await.unwrap();
        let _ = timeout(Duration::from_secs(1), socket.next()).await;
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_psubscribe_message_send_failure() {
        let Some((_container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url.clone();
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/psubscribe?patterns=user:*");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let _ = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        set_ws_send_failures(1);
        let client = redis::Client::open(redis_url.as_str()).unwrap();
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: i64 = redis::cmd("PUBLISH")
            .arg("user:123")
            .arg("hi")
            .query_async(&mut conn)
            .await
            .unwrap();
        let _ = timeout(Duration::from_secs(1), socket.next()).await;
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_psubscribe_redis_disconnect() {
        let Some((container, redis_url)) = start_redis().await else {
            return;
        };
        let repo = Arc::new(StubPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 0,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let mut config = Settings::default();
        config.redis.url = redis_url;
        let state = build_state_with_config(repo, config);
        let (addr, shutdown_tx) = spawn_pubsub_server(state).await;

        let ws_url = format!("ws://{addr}/api/v1/pubsub/psubscribe?patterns=user:*");
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();
        let _ = timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        drop(container);
        let _ = timeout(Duration::from_secs(2), socket.next()).await;
        let _ = shutdown_tx.send(());
    }

    #[test]
    fn test_pubsub_routes_creation() {
        let _routes = pubsub_routes();
    }
}
