//! Redis Cluster Connection
//!
//! Wraps the `redis` crate's async cluster client for multi-node Redis Cluster deployments.
//! The cluster client automatically handles slot mapping, MOVED/ASK redirects, and node discovery.
//!
//! Each call to `get()` creates a new `ClusterConnection` from the `ClusterClient`.
//! The `ClusterClient` internally manages connection pooling to cluster nodes, so
//! creating connections is cheap and does not require external locking.

use std::time::Duration;

use crate::infrastructure::config::{BlockingConfig, PoolConfig, RedisConfig};
use crate::infrastructure::redis::blocking::BLOCKING_RESPONSE_TIMEOUT_GRACE;
use redis::cluster::ClusterClient;
use redis::cluster_async::ClusterConnection;

/// Cluster connection pool.
///
/// Wraps `ClusterClient` which maintains internal connections to all cluster nodes.
/// Each `get()` call returns a new `ClusterConnection` that routes commands to the
/// correct node based on key hash slot, handling MOVED/ASK redirects automatically.
#[derive(Clone)]
pub struct ClusterPool {
    client: ClusterClient,
}

impl ClusterPool {
    /// Create a new cluster pool from config using default timeout settings.
    ///
    /// Test-only helper. Production code should use `with_timeout_config` so
    /// cluster response timeouts reflect the configured blocking-command cap.
    ///
    /// # Errors
    ///
    /// Returns an error if the cluster client cannot be created from the provided node URLs.
    #[cfg(test)]
    pub fn new(config: &RedisConfig) -> Result<Self, redis::RedisError> {
        Self::with_timeout_config(config, &PoolConfig::default(), &BlockingConfig::default())
    }

    /// Create a new cluster pool using the service timeout settings.
    ///
    /// Cluster connections do not expose per-borrow response timeout mutation,
    /// so the client-level timeout must be large enough for the longest
    /// configured blocking command.
    ///
    /// # Errors
    ///
    /// Returns an error if the cluster client cannot be created from the provided node URLs.
    pub fn with_timeout_config(
        config: &RedisConfig,
        pool_config: &PoolConfig,
        blocking_config: &BlockingConfig,
    ) -> Result<Self, redis::RedisError> {
        let nodes = config.cluster_node_urls();

        let response_timeout = cluster_response_timeout(pool_config, blocking_config);
        tracing::info!(
            response_timeout_ms = response_timeout.as_millis() as u64,
            command_timeout_ms = pool_config.command_timeout_ms,
            max_blocking_timeout_seconds = blocking_config.max_timeout_seconds,
            "configured Redis cluster response timeout"
        );
        let mut builder = ClusterClient::builder(nodes)
            .connection_timeout(Duration::from_millis(pool_config.connect_timeout_ms))
            .response_timeout(response_timeout)
            .overall_response_timeout(Some(response_timeout));

        if let Some(ref password) = config.password {
            builder = builder.password(password.clone());
        }

        if config.cluster_read_from_replicas {
            // `read_from_replicas()` was deprecated in redis 1.2 in favour of
            // a strategy-based API. `RandomReplicaStrategy` preserves the
            // original "any replica" routing behavior.
            builder =
                builder.read_routing_strategy(redis::cluster_read_routing::RandomReplicaStrategy);
        }

        let client = builder.build()?;

        Ok(Self { client })
    }

    /// Get a new cluster connection.
    ///
    /// Each connection is independently routed — no shared mutex.
    /// The underlying `ClusterClient` pools TCP connections to individual nodes.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection to the cluster cannot be established.
    pub async fn get(&self) -> Result<ClusterConnection, redis::RedisError> {
        self.client.get_async_connection().await
    }

    /// Execute a raw Redis command on the cluster.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails or the cluster is unreachable.
    pub async fn execute_command<T: redis::FromRedisValue>(
        &self,
        cmd: &mut redis::Cmd,
    ) -> Result<T, redis::RedisError> {
        let mut conn = self.get().await?;
        cmd.query_async(&mut conn).await
    }
}

fn cluster_response_timeout(
    pool_config: &PoolConfig,
    blocking_config: &BlockingConfig,
) -> Duration {
    let command_timeout = Duration::from_millis(pool_config.command_timeout_ms);
    let blocking_timeout = Duration::from_secs(blocking_config.max_timeout_seconds as u64)
        .saturating_add(BLOCKING_RESPONSE_TIMEOUT_GRACE);
    // Cluster connections cannot widen response timeouts per command. The client
    // timeout is therefore the wider of normal command timeout and the maximum
    // blocking-command timeout, plus grace, for every cluster command.
    command_timeout.max(blocking_timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_pool_creation_fails_with_empty_nodes() {
        let config = RedisConfig {
            cluster_enabled: true,
            cluster_nodes: String::new(),
            ..RedisConfig::default()
        };
        let result = ClusterPool::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_cluster_pool_creation_with_valid_nodes() {
        let config = RedisConfig {
            cluster_enabled: true,
            cluster_nodes: "redis://127.0.0.1:7001,redis://127.0.0.1:7002".to_string(),
            ..RedisConfig::default()
        };
        let result = ClusterPool::new(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cluster_pool_with_password() {
        let config = RedisConfig {
            cluster_enabled: true,
            cluster_nodes: "redis://127.0.0.1:7001".to_string(),
            password: Some("secret".to_string()),
            ..RedisConfig::default()
        };
        let result = ClusterPool::new(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cluster_pool_with_replica_read_routing() {
        let config = RedisConfig {
            cluster_enabled: true,
            cluster_nodes: "redis://127.0.0.1:7001".to_string(),
            cluster_read_from_replicas: true,
            ..RedisConfig::default()
        };
        let result = ClusterPool::new(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cluster_response_timeout_covers_max_blocking_timeout() {
        let pool_config = PoolConfig {
            command_timeout_ms: 1_000,
            ..PoolConfig::default()
        };
        let blocking_config = BlockingConfig {
            max_timeout_seconds: 6,
            ..BlockingConfig::default()
        };

        assert_eq!(
            cluster_response_timeout(&pool_config, &blocking_config),
            Duration::from_secs(7)
        );
    }

    #[test]
    fn test_cluster_response_timeout_preserves_longer_command_timeout() {
        let pool_config = PoolConfig {
            command_timeout_ms: 45_000,
            ..PoolConfig::default()
        };
        let blocking_config = BlockingConfig {
            max_timeout_seconds: 6,
            ..BlockingConfig::default()
        };

        assert_eq!(
            cluster_response_timeout(&pool_config, &blocking_config),
            Duration::from_secs(45)
        );
    }

    #[test]
    fn test_cluster_pool_creation_with_explicit_timeouts() {
        let config = RedisConfig {
            cluster_enabled: true,
            cluster_nodes: "redis://127.0.0.1:7001,redis://127.0.0.1:7002".to_string(),
            ..RedisConfig::default()
        };
        let pool_config = PoolConfig {
            connect_timeout_ms: 250,
            command_timeout_ms: 1_000,
            ..PoolConfig::default()
        };
        let blocking_config = BlockingConfig {
            max_timeout_seconds: 6,
            ..BlockingConfig::default()
        };

        let result = ClusterPool::with_timeout_config(&config, &pool_config, &blocking_config);
        assert!(result.is_ok());
    }
}
