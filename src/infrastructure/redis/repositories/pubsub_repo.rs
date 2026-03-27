//! Redis Pub/Sub Repository Implementation
//!
//! Implements Pub/Sub command operations using the command pool.
//! SUBSCRIBE operations are handled by PubSubManager with dedicated connections.

use async_trait::async_trait;
use redis::AsyncCommands;
use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{NumSubResult, PubSubRepository, PublishResult};
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Redis implementation of PubSubRepository
pub struct RedisPubSubRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisPubSubRepository {
    /// Create a new RedisPubSubRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PubSubRepository for RedisPubSubRepository {
    async fn publish(&self, channel: &str, message: &str) -> Result<PublishResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let receivers: i64 = conn.publish(channel, message).await?;

        Ok(PublishResult {
            channel: channel.to_string(),
            receivers,
        })
    }

    async fn spublish(&self, channel: &str, message: &str) -> Result<PublishResult, CacheError> {
        let mut conn = self.pool.get().await?;

        // SPUBLISH is for Redis Cluster sharded pub/sub (Redis 7.0+)
        let receivers: i64 = redis::cmd("SPUBLISH")
            .arg(channel)
            .arg(message)
            .query_async(&mut *conn)
            .await?;

        Ok(PublishResult {
            channel: channel.to_string(),
            receivers,
        })
    }

    async fn pubsub_channels(&self, pattern: Option<&str>) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("PUBSUB");
        cmd.arg("CHANNELS");
        if let Some(p) = pattern {
            cmd.arg(p);
        }

        let channels: Vec<String> = cmd.query_async(&mut *conn).await?;
        Ok(channels)
    }

    async fn pubsub_numsub(&self, channels: &[String]) -> Result<Vec<NumSubResult>, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("PUBSUB");
        cmd.arg("NUMSUB");
        for channel in channels {
            cmd.arg(channel);
        }

        // NUMSUB returns flat array: [channel1, count1, channel2, count2, ...]
        let result: Vec<redis::Value> = cmd.query_async(&mut *conn).await?;

        let mut results = Vec::new();
        let mut iter = result.into_iter();
        while let Some(channel_val) = iter.next() {
            let channel = match channel_val {
                redis::Value::BulkString(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                redis::Value::SimpleString(s) => s,
                _ => continue,
            };

            if let Some(count_val) = iter.next() {
                let subscribers = match count_val {
                    redis::Value::Int(n) => n,
                    _ => 0,
                };
                results.push(NumSubResult {
                    channel,
                    subscribers,
                });
            }
        }

        Ok(results)
    }

    async fn pubsub_numpat(&self) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;

        let count: i64 = redis::cmd("PUBSUB")
            .arg("NUMPAT")
            .query_async(&mut *conn)
            .await?;

        Ok(count)
    }

    async fn pubsub_shardchannels(&self, pattern: Option<&str>) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;

        // PUBSUB SHARDCHANNELS [pattern] - Redis 7.0+ for cluster
        let mut cmd = redis::cmd("PUBSUB");
        cmd.arg("SHARDCHANNELS");
        if let Some(p) = pattern {
            cmd.arg(p);
        }

        let channels: Vec<String> = cmd.query_async(&mut *conn).await?;
        Ok(channels)
    }

    async fn pubsub_shardnumsub(
        &self,
        channels: &[String],
    ) -> Result<Vec<NumSubResult>, CacheError> {
        let mut conn = self.pool.get().await?;

        // PUBSUB SHARDNUMSUB [channel ...] - Redis 7.0+ for cluster
        let mut cmd = redis::cmd("PUBSUB");
        cmd.arg("SHARDNUMSUB");
        for channel in channels {
            cmd.arg(channel);
        }

        // Returns flat array like NUMSUB
        let result: Vec<redis::Value> = cmd.query_async(&mut *conn).await?;

        let mut results = Vec::new();
        let mut iter = result.into_iter();
        while let Some(channel_val) = iter.next() {
            let channel = match channel_val {
                redis::Value::BulkString(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                redis::Value::SimpleString(s) => s,
                _ => continue,
            };

            if let Some(count_val) = iter.next() {
                let subscribers = match count_val {
                    redis::Value::Int(n) => n,
                    _ => 0,
                };
                results.push(NumSubResult {
                    channel,
                    subscribers,
                });
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publish_result() {
        let result = PublishResult {
            channel: "test-channel".to_string(),
            receivers: 5,
        };
        assert_eq!(result.channel, "test-channel");
        assert_eq!(result.receivers, 5);
    }

    #[test]
    fn test_numsub_result() {
        let result = NumSubResult {
            channel: "my-channel".to_string(),
            subscribers: 10,
        };
        assert_eq!(result.channel, "my-channel");
        assert_eq!(result.subscribers, 10);
    }
}
