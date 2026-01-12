//! Pub/Sub Service
//!
//! Business logic layer for Pub/Sub operations.
//! This service combines:
//! - Command pool operations (PUBLISH, PUBSUB info commands) via PubSubRepository
//! - Dedicated connection operations (SUBSCRIBE) via PubSubManager

use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{NumSubResult, PubSubRepository, PublishResult};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::pubsub_manager::{PubSubConnection, PubSubManager, PubSubStatsSnapshot};
use crate::infrastructure::redis::repositories::RedisPubSubRepository;

/// Service for Pub/Sub operations
///
/// Combines command pool operations with dedicated subscription connections.
pub struct PubSubService {
    /// Repository for PUBLISH and info commands (uses command pool)
    repository: Arc<dyn PubSubRepository>,
    /// Manager for subscription connections (dedicated connections)
    pubsub_manager: Arc<PubSubManager>,
}

impl PubSubService {
    /// Create a new PubSubService
    pub fn new(pool: Arc<InstrumentedPool>, pubsub_manager: Arc<PubSubManager>) -> Self {
        Self::new_with_repository(
            Arc::new(RedisPubSubRepository::new(pool)),
            pubsub_manager,
        )
    }

    /// Create a PubSubService with a custom repository (useful for testing)
    pub fn new_with_repository(
        repository: Arc<dyn PubSubRepository>,
        pubsub_manager: Arc<PubSubManager>,
    ) -> Self {
        Self {
            repository,
            pubsub_manager,
        }
    }

    // ========== Command Pool Operations (PUBLISH, INFO) ==========

    /// Publish a message to a channel (PUBLISH)
    ///
    /// Uses the command pool for short-lived connection.
    pub async fn publish(&self, channel: &str, message: &str) -> Result<PublishResult, CacheError> {
        self.validate_channel(channel)?;
        self.repository.publish(channel, message).await
    }

    /// Publish a message to a sharded channel (SPUBLISH)
    ///
    /// For Redis Cluster sharded pub/sub (Redis 7.0+).
    pub async fn spublish(&self, channel: &str, message: &str) -> Result<PublishResult, CacheError> {
        self.validate_channel(channel)?;
        self.repository.spublish(channel, message).await
    }

    /// List active channels (PUBSUB CHANNELS)
    pub async fn channels(&self, pattern: Option<&str>) -> Result<Vec<String>, CacheError> {
        if let Some(p) = pattern {
            self.validate_pattern(p)?;
        }
        self.repository.pubsub_channels(pattern).await
    }

    /// Get subscriber count for channels (PUBSUB NUMSUB)
    pub async fn numsub(&self, channels: &[String]) -> Result<Vec<NumSubResult>, CacheError> {
        for channel in channels {
            self.validate_channel(channel)?;
        }
        self.repository.pubsub_numsub(channels).await
    }

    /// Get number of pattern subscriptions (PUBSUB NUMPAT)
    pub async fn numpat(&self) -> Result<i64, CacheError> {
        self.repository.pubsub_numpat().await
    }

    /// List active sharded channels (PUBSUB SHARDCHANNELS)
    pub async fn shardchannels(&self, pattern: Option<&str>) -> Result<Vec<String>, CacheError> {
        if let Some(p) = pattern {
            self.validate_pattern(p)?;
        }
        self.repository.pubsub_shardchannels(pattern).await
    }

    /// Get subscriber count for sharded channels (PUBSUB SHARDNUMSUB)
    pub async fn shardnumsub(&self, channels: &[String]) -> Result<Vec<NumSubResult>, CacheError> {
        for channel in channels {
            self.validate_channel(channel)?;
        }
        self.repository.pubsub_shardnumsub(channels).await
    }

    // ========== Dedicated Connection Operations (SUBSCRIBE) ==========

    /// Create a new subscription connection
    ///
    /// This creates a dedicated connection (not from pool) for subscription use.
    /// The caller is responsible for subscribing to channels and handling messages.
    pub async fn create_subscription(&self) -> Result<PubSubConnection, CacheError> {
        self.pubsub_manager.create_subscription().await
    }

    /// Check if we can create a new subscription
    pub fn can_subscribe(&self) -> bool {
        self.pubsub_manager.can_subscribe()
    }

    /// Get current number of active subscriptions
    pub fn active_subscriptions(&self) -> usize {
        self.pubsub_manager.active_subscriptions()
    }

    /// Get maximum allowed subscriptions
    pub fn max_subscriptions(&self) -> usize {
        self.pubsub_manager.max_subscriptions()
    }

    /// Get Pub/Sub statistics
    pub fn get_stats(&self) -> PubSubStatsSnapshot {
        self.pubsub_manager.get_stats()
    }

    /// Record a message received (for stats)
    pub fn record_message(&self) {
        self.pubsub_manager.record_message();
    }

    /// Record an error (for stats)
    pub fn record_error(&self) {
        self.pubsub_manager.record_error();
    }

    // ========== Validation ==========

    fn validate_channel(&self, channel: &str) -> Result<(), CacheError> {
        if channel.is_empty() {
            return Err(CacheError::InvalidInput("Channel name cannot be empty".to_string()));
        }
        if channel.len() > 1024 {
            return Err(CacheError::InvalidInput("Channel name too long (max 1024 characters)".to_string()));
        }
        Ok(())
    }

    fn validate_pattern(&self, pattern: &str) -> Result<(), CacheError> {
        if pattern.is_empty() {
            return Err(CacheError::InvalidInput("Pattern cannot be empty".to_string()));
        }
        if pattern.len() > 1024 {
            return Err(CacheError::InvalidInput("Pattern too long (max 1024 characters)".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;
    use crate::infrastructure::config::PubSubConfig;

    struct StaticPubSubRepository {
        publish_result: PublishResult,
        numpat_result: i64,
        channels_result: Vec<String>,
        numsub_result: Vec<NumSubResult>,
    }

    #[async_trait]
    impl PubSubRepository for StaticPubSubRepository {
        async fn publish(&self, _channel: &str, _message: &str) -> Result<PublishResult, CacheError> {
            Ok(self.publish_result.clone())
        }

        async fn spublish(&self, _channel: &str, _message: &str) -> Result<PublishResult, CacheError> {
            Ok(self.publish_result.clone())
        }

        async fn pubsub_channels(&self, _pattern: Option<&str>) -> Result<Vec<String>, CacheError> {
            Ok(self.channels_result.clone())
        }

        async fn pubsub_numsub(&self, _channels: &[String]) -> Result<Vec<NumSubResult>, CacheError> {
            Ok(self.numsub_result.clone())
        }

        async fn pubsub_numpat(&self) -> Result<i64, CacheError> {
            Ok(self.numpat_result)
        }

        async fn pubsub_shardchannels(&self, _pattern: Option<&str>) -> Result<Vec<String>, CacheError> {
            Ok(self.channels_result.clone())
        }

        async fn pubsub_shardnumsub(&self, _channels: &[String]) -> Result<Vec<NumSubResult>, CacheError> {
            Ok(self.numsub_result.clone())
        }
    }

    fn service_with_repo(repo: Arc<dyn PubSubRepository>) -> PubSubService {
        let config = PubSubConfig::default();
        let manager = Arc::new(PubSubManager::new("redis://127.0.0.1/", config).unwrap());
        PubSubService::new_with_repository(repo, manager)
    }

    #[tokio::test]
    async fn test_publish_validation_errors() {
        let repo = Arc::new(StaticPubSubRepository {
            publish_result: PublishResult {
                channel: "chan".to_string(),
                receivers: 1,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let service = service_with_repo(repo);

        let err = service.publish("", "msg").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let long_channel = "x".repeat(1025);
        let err = service.publish(&long_channel, "msg").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_channels_validation() {
        let repo = Arc::new(StaticPubSubRepository {
            publish_result: PublishResult {
                channel: "chan".to_string(),
                receivers: 1,
            },
            numpat_result: 0,
            channels_result: vec!["alpha".to_string()],
            numsub_result: Vec::new(),
        });
        let service = service_with_repo(repo);

        let err = service.channels(Some("")).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_numsub_validation() {
        let repo = Arc::new(StaticPubSubRepository {
            publish_result: PublishResult {
                channel: "chan".to_string(),
                receivers: 1,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let service = service_with_repo(repo);

        let err = service.numsub(&vec!["".to_string()]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_sharded_validation() {
        let repo = Arc::new(StaticPubSubRepository {
            publish_result: PublishResult {
                channel: "chan".to_string(),
                receivers: 1,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let service = service_with_repo(repo);

        let err = service.shardchannels(Some("")).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.shardnumsub(&vec!["".to_string()]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_pattern_length_validation() {
        let repo = Arc::new(StaticPubSubRepository {
            publish_result: PublishResult {
                channel: "chan".to_string(),
                receivers: 1,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let service = service_with_repo(repo);

        let long_pattern = "x".repeat(1025);
        let err = service.channels(Some(&long_pattern)).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_publish_and_numpat_success() {
        let repo = Arc::new(StaticPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 3,
            },
            numpat_result: 7,
            channels_result: vec!["news".to_string()],
            numsub_result: vec![NumSubResult {
                channel: "news".to_string(),
                subscribers: 2,
            }],
        });
        let service = service_with_repo(repo);

        let result = service.publish("news", "hi").await.unwrap();
        assert_eq!(result.channel, "news");
        assert_eq!(result.receivers, 3);

        let numpat = service.numpat().await.unwrap();
        assert_eq!(numpat, 7);
    }

    #[test]
    fn test_stats_and_counters() {
        let repo = Arc::new(StaticPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 3,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let service = service_with_repo(repo);

        let stats = service.get_stats();
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.errors, 0);

        service.record_message();
        service.record_error();
        let stats = service.get_stats();
        assert_eq!(stats.total_messages, 1);
        assert_eq!(stats.errors, 1);
    }

    #[tokio::test]
    async fn test_create_subscription_limit_reached() {
        let repo = Arc::new(StaticPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 3,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let config = PubSubConfig {
            max_subscriptions: 0,
            ..PubSubConfig::default()
        };
        let manager = Arc::new(PubSubManager::new("redis://127.0.0.1/", config).unwrap());
        let service = PubSubService::new_with_repository(repo, manager);

        let err = service.create_subscription().await.unwrap_err();
        assert!(matches!(err, CacheError::SubscriptionLimitReached));
    }

    #[test]
    fn test_subscription_limits_accessors() {
        let repo = Arc::new(StaticPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 3,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let config = PubSubConfig {
            max_subscriptions: 2,
            ..PubSubConfig::default()
        };
        let manager = Arc::new(PubSubManager::new("redis://127.0.0.1/", config).unwrap());
        let service = PubSubService::new_with_repository(repo, manager);

        assert_eq!(service.active_subscriptions(), 0);
        assert_eq!(service.max_subscriptions(), 2);
    }

    #[test]
    fn test_can_subscribe_with_zero_limit() {
        let repo = Arc::new(StaticPubSubRepository {
            publish_result: PublishResult {
                channel: "news".to_string(),
                receivers: 3,
            },
            numpat_result: 0,
            channels_result: Vec::new(),
            numsub_result: Vec::new(),
        });
        let config = PubSubConfig {
            max_subscriptions: 0,
            ..PubSubConfig::default()
        };
        let manager = Arc::new(PubSubManager::new("redis://127.0.0.1/", config).unwrap());
        let service = PubSubService::new_with_repository(repo, manager);

        assert!(!service.can_subscribe());
    }
}
