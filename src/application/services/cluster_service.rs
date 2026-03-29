//! Cluster Service
//!
//! Application service for Redis Cluster operations.

use crate::domain::errors::CacheError;
use crate::domain::repositories::{ClusterInfo, ClusterNode, ClusterRepository, ClusterSlotRange};
use std::sync::Arc;

pub struct ClusterService {
    repository: Arc<dyn ClusterRepository>,
}

impl ClusterService {
    pub fn new(repository: Arc<dyn ClusterRepository>) -> Self {
        Self { repository }
    }

    /// Get cluster info
    pub async fn cluster_info(&self) -> Result<ClusterInfo, CacheError> {
        self.repository.cluster_info().await
    }

    /// Get cluster nodes
    pub async fn cluster_nodes(&self) -> Result<Vec<ClusterNode>, CacheError> {
        self.repository.cluster_nodes().await
    }

    /// Get cluster slot mapping
    pub async fn cluster_slots(&self) -> Result<Vec<ClusterSlotRange>, CacheError> {
        self.repository.cluster_slots().await
    }

    /// Get cluster shards (Redis 7.0+)
    pub async fn cluster_shards(&self) -> Result<redis::Value, CacheError> {
        self.repository.cluster_shards().await
    }

    /// Get hash slot for a key
    pub async fn cluster_keyslot(&self, key: &str) -> Result<u16, CacheError> {
        self.repository.cluster_keyslot(key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockClusterRepo;

    #[async_trait]
    impl ClusterRepository for MockClusterRepo {
        async fn cluster_info(&self) -> Result<ClusterInfo, CacheError> {
            Ok(ClusterInfo {
                cluster_state: "ok".to_string(),
                cluster_slots_assigned: 16384,
                cluster_slots_ok: 16384,
                cluster_slots_pfail: 0,
                cluster_slots_fail: 0,
                cluster_known_nodes: 3,
                cluster_size: 3,
                cluster_current_epoch: 3,
                cluster_my_epoch: 1,
            })
        }

        async fn cluster_nodes(&self) -> Result<Vec<ClusterNode>, CacheError> {
            Ok(vec![ClusterNode {
                id: "abc123".to_string(),
                address: "127.0.0.1:7001".to_string(),
                flags: "master".to_string(),
                master_id: None,
                ping_sent: 0,
                pong_recv: 1000,
                config_epoch: 1,
                link_state: "connected".to_string(),
                slots: vec!["0-5460".to_string()],
            }])
        }

        async fn cluster_slots(&self) -> Result<Vec<ClusterSlotRange>, CacheError> {
            Ok(vec![])
        }

        async fn cluster_shards(&self) -> Result<redis::Value, CacheError> {
            Ok(redis::Value::Array(vec![]))
        }

        async fn cluster_keyslot(&self, _key: &str) -> Result<u16, CacheError> {
            Ok(12539)
        }
    }

    #[tokio::test]
    async fn test_cluster_info() {
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        let info = service.cluster_info().await.unwrap();
        assert_eq!(info.cluster_state, "ok");
        assert_eq!(info.cluster_slots_assigned, 16384);
    }

    #[tokio::test]
    async fn test_cluster_nodes() {
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        let nodes = service.cluster_nodes().await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].flags, "master");
    }

    #[tokio::test]
    async fn test_cluster_keyslot() {
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        let slot = service.cluster_keyslot("test").await.unwrap();
        assert_eq!(slot, 12539);
    }

    #[tokio::test]
    async fn test_cluster_slots() {
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        let slots = service.cluster_slots().await.unwrap();
        assert!(slots.is_empty());
    }

    #[tokio::test]
    async fn test_cluster_shards() {
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        let shards = service.cluster_shards().await.unwrap();
        assert!(matches!(shards, redis::Value::Array(v) if v.is_empty()));
    }
}
