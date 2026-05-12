//! Redis Cluster Connection
//!
//! Wraps the `redis` crate's async cluster client for multi-node Redis Cluster deployments.
//! The cluster client automatically handles slot mapping, MOVED/ASK redirects, and node discovery.
//!
//! Each call to `get()` creates a new `ClusterConnection` from the `ClusterClient`.
//! The `ClusterClient` internally manages connection pooling to cluster nodes, so
//! creating connections is cheap and does not require external locking.

use crate::infrastructure::config::RedisConfig;
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
    /// Create a new cluster pool from config.
    ///
    /// # Errors
    ///
    /// Returns an error if the cluster client cannot be created from the provided node URLs.
    pub fn new(config: &RedisConfig) -> Result<Self, redis::RedisError> {
        let nodes = config.cluster_node_urls();

        let mut builder = ClusterClient::builder(nodes);

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
}
