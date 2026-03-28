//! Pub/Sub Connection Manager
//!
//! Manages dedicated connections for Redis Pub/Sub subscriptions.
//! These connections are NOT from the command pool - each subscription
//! gets its own dedicated connection to support the blocking nature of SUBSCRIBE.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use redis::aio::PubSub;
use redis::{Client, RedisResult};

use crate::domain::errors::CacheError;
use crate::infrastructure::config::PubSubConfig;
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Statistics for Pub/Sub connections
#[derive(Debug, Default)]
pub struct PubSubStats {
    /// Current number of active subscriptions
    pub active_subscriptions: AtomicUsize,
    /// Maximum allowed subscriptions
    pub max_subscriptions: usize,
    /// Total subscriptions created (lifetime)
    pub total_created: AtomicU64,
    /// Total messages received (lifetime)
    pub total_messages: AtomicU64,
    /// Total errors encountered
    pub errors: AtomicU64,
}

impl PubSubStats {
    fn new(max_subscriptions: usize) -> Self {
        Self {
            max_subscriptions,
            ..Default::default()
        }
    }
}

/// Source of the Redis URL for creating new pub/sub connections.
enum UrlSource {
    /// Read from the pool's resolved URL (follows sentinel failover)
    Pool(Arc<InstrumentedPool>),
    /// Fixed URL (standalone mode or tests)
    Static(String),
}

impl UrlSource {
    fn url(&self) -> String {
        match self {
            Self::Pool(pool) => pool.resolved_url(),
            Self::Static(url) => url.clone(),
        }
    }
}

/// Manager for Pub/Sub dedicated connections
///
/// This manager creates dedicated Redis connections for subscriptions,
/// separate from the command pool. This prevents subscription operations
/// from blocking or exhausting the main connection pool.
///
/// In sentinel mode, each new subscription reads the current master URL
/// from the pool, so failover is picked up automatically for new connections.
///
/// `connection_timeout_ms` is enforced when creating new subscription connections.
/// Idle cleanup is handled by WebSocket disconnection which drops the [`PubSubConnection`].
pub struct PubSubManager {
    /// Where to read the Redis URL from
    url_source: UrlSource,
    /// Configuration
    config: PubSubConfig,
    /// Connection statistics
    stats: Arc<PubSubStats>,
}

impl PubSubManager {
    /// Create a new PubSubManager backed by the given pool.
    /// The pool's `resolved_url()` is read on each new subscription,
    /// so sentinel failover propagates to new pub/sub connections.
    pub fn new_with_pool(pool: Arc<InstrumentedPool>, config: PubSubConfig) -> Self {
        let stats = Arc::new(PubSubStats::new(config.max_subscriptions));
        Self {
            url_source: UrlSource::Pool(pool),
            config,
            stats,
        }
    }

    /// Create a new PubSubManager from a fixed URL string (tests or standalone).
    pub fn new(redis_url: &str, config: PubSubConfig) -> Result<Self, CacheError> {
        // Verify the URL is valid
        Client::open(redis_url).map_err(|e| {
            CacheError::ConnectionFailed(format!("Failed to create Redis client for Pub/Sub: {e}",))
        })?;

        let stats = Arc::new(PubSubStats::new(config.max_subscriptions));
        Ok(Self {
            url_source: UrlSource::Static(redis_url.to_string()),
            config,
            stats,
        })
    }

    /// Get current number of active subscriptions
    pub fn active_subscriptions(&self) -> usize {
        self.stats.active_subscriptions.load(Ordering::Relaxed)
    }

    /// Get the maximum allowed subscriptions
    pub fn max_subscriptions(&self) -> usize {
        self.config.max_subscriptions
    }

    /// Check if we can create a new subscription
    pub fn can_subscribe(&self) -> bool {
        self.active_subscriptions() < self.config.max_subscriptions
    }

    /// Get statistics
    pub fn get_stats(&self) -> PubSubStatsSnapshot {
        PubSubStatsSnapshot {
            active_subscriptions: self.stats.active_subscriptions.load(Ordering::Relaxed),
            max_subscriptions: self.stats.max_subscriptions,
            total_created: self.stats.total_created.load(Ordering::Relaxed),
            total_messages: self.stats.total_messages.load(Ordering::Relaxed),
            errors: self.stats.errors.load(Ordering::Relaxed),
        }
    }

    /// Create a new dedicated Pub/Sub connection
    ///
    /// This creates a NEW connection (not from pool) for subscription use.
    /// Returns a PubSubConnection wrapper that decrements the counter on drop.
    pub async fn create_subscription(&self) -> Result<PubSubConnection, CacheError> {
        // Check limit before creating connection
        let current = self
            .stats
            .active_subscriptions
            .fetch_add(1, Ordering::SeqCst);
        if current >= self.config.max_subscriptions {
            // Revert the increment
            self.stats
                .active_subscriptions
                .fetch_sub(1, Ordering::SeqCst);
            return Err(CacheError::SubscriptionLimitReached);
        }

        // Create a fresh client from the current resolved URL.
        // In sentinel mode this picks up the new master after failover.
        let current_url = self.url_source.url();
        let client = Client::open(current_url.as_str()).map_err(|e| {
            self.stats
                .active_subscriptions
                .fetch_sub(1, Ordering::SeqCst);
            self.stats.errors.fetch_add(1, Ordering::Relaxed);
            CacheError::ConnectionFailed(format!("Failed to create Pub/Sub client: {e}"))
        })?;

        let connect_timeout = Duration::from_millis(self.config.connection_timeout_ms);
        let pubsub = tokio::time::timeout(connect_timeout, client.get_async_pubsub())
            .await
            .map_err(|_| {
                self.stats
                    .active_subscriptions
                    .fetch_sub(1, Ordering::SeqCst);
                self.stats.errors.fetch_add(1, Ordering::Relaxed);
                CacheError::ConnectionFailed(format!(
                    "Pub/Sub connection timed out after {}ms",
                    self.config.connection_timeout_ms
                ))
            })?
            .map_err(|e| {
                self.stats
                    .active_subscriptions
                    .fetch_sub(1, Ordering::SeqCst);
                self.stats.errors.fetch_add(1, Ordering::Relaxed);
                CacheError::ConnectionFailed(format!("Failed to create Pub/Sub connection: {}", e))
            })?;

        self.stats.total_created.fetch_add(1, Ordering::Relaxed);

        Ok(PubSubConnection {
            inner: Some(pubsub),
            stats: self.stats.clone(),
        })
    }

    /// Increment message counter
    pub fn record_message(&self) {
        self.stats.total_messages.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment error counter
    pub fn record_error(&self) {
        self.stats.errors.fetch_add(1, Ordering::Relaxed);
    }
}

/// Snapshot of Pub/Sub statistics (for API responses)
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PubSubStatsSnapshot {
    pub active_subscriptions: usize,
    pub max_subscriptions: usize,
    pub total_created: u64,
    pub total_messages: u64,
    pub errors: u64,
}

/// Wrapper around a Pub/Sub connection that tracks lifecycle
///
/// When dropped, this automatically decrements the active subscription counter.
pub struct PubSubConnection {
    inner: Option<PubSub>,
    stats: Arc<PubSubStats>,
}

impl std::fmt::Debug for PubSubConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PubSubConnection")
            .field("inner", &self.inner.as_ref().map(|_| "PubSub<active>"))
            .field("stats", &self.stats)
            .finish()
    }
}

impl PubSubConnection {
    /// Subscribe to a channel
    pub async fn subscribe(&mut self, channel: &str) -> RedisResult<()> {
        if let Some(ref mut pubsub) = self.inner {
            pubsub.subscribe(channel).await
        } else {
            Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "PubSub connection already consumed",
            )))
        }
    }

    /// Subscribe to multiple channels
    pub async fn subscribe_many<'a>(
        &mut self,
        channels: impl IntoIterator<Item = &'a str>,
    ) -> RedisResult<()> {
        if let Some(ref mut pubsub) = self.inner {
            for channel in channels {
                pubsub.subscribe(channel).await?;
            }
            Ok(())
        } else {
            Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "PubSub connection already consumed",
            )))
        }
    }

    /// Subscribe to a pattern
    pub async fn psubscribe(&mut self, pattern: &str) -> RedisResult<()> {
        if let Some(ref mut pubsub) = self.inner {
            pubsub.psubscribe(pattern).await
        } else {
            Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "PubSub connection already consumed",
            )))
        }
    }

    /// Subscribe to multiple patterns
    pub async fn psubscribe_many<'a>(
        &mut self,
        patterns: impl IntoIterator<Item = &'a str>,
    ) -> RedisResult<()> {
        if let Some(ref mut pubsub) = self.inner {
            for pattern in patterns {
                pubsub.psubscribe(pattern).await?;
            }
            Ok(())
        } else {
            Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "PubSub connection already consumed",
            )))
        }
    }

    /// Unsubscribe from a channel
    pub async fn unsubscribe(&mut self, channel: &str) -> RedisResult<()> {
        if let Some(ref mut pubsub) = self.inner {
            pubsub.unsubscribe(channel).await
        } else {
            Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "PubSub connection already consumed",
            )))
        }
    }

    /// Unsubscribe from a pattern
    pub async fn punsubscribe(&mut self, pattern: &str) -> RedisResult<()> {
        if let Some(ref mut pubsub) = self.inner {
            pubsub.punsubscribe(pattern).await
        } else {
            Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "PubSub connection already consumed",
            )))
        }
    }

    /// Take the message stream, consuming this connection wrapper
    ///
    /// Returns a stream that yields messages. The stats counter will be
    /// decremented when the returned stream is dropped.
    pub fn into_on_message(mut self) -> Option<PubSubMessageStream> {
        self.inner.take().map(|pubsub| PubSubMessageStream {
            inner: pubsub.into_on_message(),
            stats: self.stats.clone(),
        })
    }

    /// Get access to the inner PubSub for advanced operations
    pub fn inner_mut(&mut self) -> Option<&mut PubSub> {
        self.inner.as_mut()
    }
}

#[cfg(test)]
impl PubSubConnection {
    pub(crate) fn new_for_tests(inner: Option<PubSub>, stats: Arc<PubSubStats>) -> Self {
        Self { inner, stats }
    }
}

impl Drop for PubSubConnection {
    fn drop(&mut self) {
        // Only decrement if inner is still present (not consumed by into_on_message)
        if self.inner.is_some() {
            self.stats
                .active_subscriptions
                .fetch_sub(1, Ordering::SeqCst);
        }
    }
}

/// Stream wrapper that decrements counter when dropped
pub struct PubSubMessageStream {
    inner: redis::aio::PubSubStream,
    stats: Arc<PubSubStats>,
}

impl futures::Stream for PubSubMessageStream {
    type Item = redis::Msg;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl Drop for PubSubMessageStream {
    fn drop(&mut self) {
        self.stats
            .active_subscriptions
            .fetch_sub(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use testcontainers::ContainerAsync;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::redis::{REDIS_PORT, Redis};
    use tokio::time::{Duration, timeout};

    async fn start_redis() -> (ContainerAsync<Redis>, String) {
        let container = Redis::default().start().await.unwrap();
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
        let url = format!("redis://{host}:{port}");
        (container, url)
    }

    #[test]
    fn test_pubsub_stats_new() {
        let stats = PubSubStats::new(100);
        assert_eq!(stats.max_subscriptions, 100);
        assert_eq!(stats.active_subscriptions.load(Ordering::Relaxed), 0);
        assert_eq!(stats.total_created.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_pubsub_stats_snapshot() {
        let stats = Arc::new(PubSubStats::new(50));
        stats.active_subscriptions.store(10, Ordering::Relaxed);
        stats.total_created.store(100, Ordering::Relaxed);
        stats.total_messages.store(1000, Ordering::Relaxed);
        stats.errors.store(5, Ordering::Relaxed);

        let snapshot = PubSubStatsSnapshot {
            active_subscriptions: stats.active_subscriptions.load(Ordering::Relaxed),
            max_subscriptions: stats.max_subscriptions,
            total_created: stats.total_created.load(Ordering::Relaxed),
            total_messages: stats.total_messages.load(Ordering::Relaxed),
            errors: stats.errors.load(Ordering::Relaxed),
        };

        assert_eq!(snapshot.active_subscriptions, 10);
        assert_eq!(snapshot.max_subscriptions, 50);
        assert_eq!(snapshot.total_created, 100);
        assert_eq!(snapshot.total_messages, 1000);
        assert_eq!(snapshot.errors, 5);
    }

    #[test]
    fn test_manager_stats_and_counters() {
        let config = PubSubConfig::default();
        let manager = PubSubManager::new("redis://127.0.0.1/", config).unwrap();

        manager.record_message();
        manager.record_error();

        let stats = manager.get_stats();
        assert_eq!(stats.total_messages, 1);
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.max_subscriptions, manager.max_subscriptions());
    }

    #[test]
    fn test_manager_can_subscribe_limits() {
        let config = PubSubConfig {
            max_subscriptions: 1,
            ..PubSubConfig::default()
        };
        let manager = PubSubManager::new("redis://127.0.0.1/", config).unwrap();

        manager
            .stats
            .active_subscriptions
            .store(1, Ordering::Relaxed);
        assert_eq!(manager.active_subscriptions(), 1);
        assert!(!manager.can_subscribe());
    }

    #[tokio::test]
    async fn test_create_subscription_limit_reached() {
        let config = PubSubConfig {
            max_subscriptions: 1,
            ..PubSubConfig::default()
        };
        let manager = PubSubManager::new("redis://127.0.0.1/", config).unwrap();

        manager
            .stats
            .active_subscriptions
            .store(1, Ordering::SeqCst);

        let err = manager.create_subscription().await.unwrap_err();
        assert!(matches!(err, CacheError::SubscriptionLimitReached));
        assert_eq!(manager.active_subscriptions(), 1);
    }

    #[tokio::test]
    async fn test_pubsub_connection_errors_without_inner() {
        let stats = Arc::new(PubSubStats::new(1));
        let mut conn = PubSubConnection { inner: None, stats };

        let err = conn.subscribe("chan").await.unwrap_err();
        assert_eq!(err.kind(), redis::ErrorKind::IoError);

        let err = conn.subscribe_many(["a", "b"]).await.unwrap_err();
        assert_eq!(err.kind(), redis::ErrorKind::IoError);

        let err = conn.psubscribe("pat").await.unwrap_err();
        assert_eq!(err.kind(), redis::ErrorKind::IoError);

        let err = conn.psubscribe_many(["pat1", "pat2"]).await.unwrap_err();
        assert_eq!(err.kind(), redis::ErrorKind::IoError);

        let err = conn.unsubscribe("chan").await.unwrap_err();
        assert_eq!(err.kind(), redis::ErrorKind::IoError);

        let err = conn.punsubscribe("pat").await.unwrap_err();
        assert_eq!(err.kind(), redis::ErrorKind::IoError);

        assert!(conn.into_on_message().is_none());
    }

    #[tokio::test]
    async fn test_create_subscription_connection_failure_updates_stats() {
        let config = PubSubConfig::default();
        let manager = PubSubManager::new("redis://127.0.0.1:1", config).unwrap();

        let result = timeout(Duration::from_secs(2), manager.create_subscription()).await;
        let err = result.expect("timeout").unwrap_err();
        assert!(matches!(err, CacheError::ConnectionFailed(_)));

        let stats = manager.get_stats();
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.active_subscriptions, 0);
    }

    #[tokio::test]
    async fn test_pubsub_connection_with_real_redis() {
        let (_container, redis_url) = start_redis().await;
        let config = PubSubConfig {
            max_subscriptions: 4,
            ..PubSubConfig::default()
        };
        let manager = PubSubManager::new(&redis_url, config).unwrap();

        let mut conn = manager.create_subscription().await.unwrap();
        assert_eq!(manager.active_subscriptions(), 1);
        let _debug = format!("{:?}", conn);

        conn.subscribe_many(["chan1", "chan2"]).await.unwrap();

        let mut pconn = manager.create_subscription().await.unwrap();
        pconn.psubscribe_many(["pat*"]).await.unwrap();
        assert!(pconn.inner_mut().is_some());

        let mut stream = conn.into_on_message().expect("stream");
        let client = redis::Client::open(redis_url.as_str()).unwrap();
        let mut publish_conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: i64 = redis::cmd("PUBLISH")
            .arg("chan1")
            .arg("hello")
            .query_async(&mut publish_conn)
            .await
            .unwrap();

        let msg = timeout(Duration::from_secs(2), stream.next())
            .await
            .unwrap()
            .expect("stream item");
        assert_eq!(msg.get_channel_name(), "chan1");

        drop(stream);
        assert_eq!(manager.active_subscriptions(), 1);

        let conn_drop = manager.create_subscription().await.unwrap();
        drop(conn_drop);
        assert_eq!(manager.active_subscriptions(), 1);
    }
}
