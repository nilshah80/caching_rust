//! Pub/Sub Repository Trait
//!
//! Repository trait for Redis Pub/Sub operations.
//! Note: SUBSCRIBE operations are handled by PubSubManager with dedicated connections.
//! This repository handles PUBLISH and PUBSUB info commands that use the command pool.

use async_trait::async_trait;

use crate::domain::errors::CacheError;

/// Result from PUBLISH command
#[derive(Debug, Clone)]
pub struct PublishResult {
    /// Channel the message was published to
    pub channel: String,
    /// Number of clients that received the message
    pub receivers: i64,
}

/// Result from PUBSUB NUMSUB command
#[derive(Debug, Clone)]
pub struct NumSubResult {
    /// Channel name
    pub channel: String,
    /// Number of subscribers
    pub subscribers: i64,
}

/// Repository trait for Pub/Sub command operations
///
/// This handles PUBLISH and PUBSUB info commands that use the command pool.
/// SUBSCRIBE/PSUBSCRIBE operations are NOT in this trait - they use
/// dedicated connections via PubSubManager.
#[async_trait]
pub trait PubSubRepository: Send + Sync {
    /// Publish a message to a channel (PUBLISH)
    ///
    /// Returns the number of clients that received the message.
    async fn publish(&self, channel: &str, message: &str) -> Result<PublishResult, CacheError>;

    /// Publish a message to a sharded channel (SPUBLISH)
    ///
    /// For Redis Cluster sharded pub/sub (Redis 7.0+).
    async fn spublish(&self, channel: &str, message: &str) -> Result<PublishResult, CacheError>;

    /// List active channels (PUBSUB CHANNELS)
    ///
    /// Returns list of channels with at least one subscriber.
    /// If pattern is provided, only channels matching the pattern are returned.
    async fn pubsub_channels(&self, pattern: Option<&str>) -> Result<Vec<String>, CacheError>;

    /// Get subscriber count for channels (PUBSUB NUMSUB)
    ///
    /// Returns number of subscribers for each specified channel.
    async fn pubsub_numsub(&self, channels: &[String]) -> Result<Vec<NumSubResult>, CacheError>;

    /// Get number of pattern subscriptions (PUBSUB NUMPAT)
    ///
    /// Returns total number of pattern subscriptions across all clients.
    async fn pubsub_numpat(&self) -> Result<i64, CacheError>;

    /// List active sharded channels (PUBSUB SHARDCHANNELS)
    ///
    /// For Redis Cluster (Redis 7.0+).
    async fn pubsub_shardchannels(&self, pattern: Option<&str>) -> Result<Vec<String>, CacheError>;

    /// Get subscriber count for sharded channels (PUBSUB SHARDNUMSUB)
    ///
    /// For Redis Cluster (Redis 7.0+).
    async fn pubsub_shardnumsub(
        &self,
        channels: &[String],
    ) -> Result<Vec<NumSubResult>, CacheError>;
}
