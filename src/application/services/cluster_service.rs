//! Cluster Service
//!
//! Application service for Redis Cluster operations.

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    ClusterInfo, ClusterNode, ClusterRepository, ClusterSlotRange, ClusterSlotStatsFilter,
    SlotStats,
};
use std::sync::Arc;

const MAX_CLUSTER_SLOT: u16 = 16_383;

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

    /// Get this node's cluster ID
    pub async fn cluster_myid(&self) -> Result<String, CacheError> {
        self.repository.cluster_myid().await
    }

    /// Get this node's shard ID
    pub async fn cluster_myshardid(&self) -> Result<String, CacheError> {
        self.repository.cluster_myshardid().await
    }

    /// Get cluster bus links
    pub async fn cluster_links(&self) -> Result<redis::Value, CacheError> {
        self.repository.cluster_links().await
    }

    /// List replicas for a master node
    pub async fn cluster_replicas(&self, node_id: &str) -> Result<Vec<ClusterNode>, CacheError> {
        if node_id.trim().is_empty() {
            return Err(CacheError::InvalidInput(
                "node_id cannot be empty".to_string(),
            ));
        }
        self.repository.cluster_replicas(node_id).await
    }

    /// Get hash slot for a key
    pub async fn cluster_keyslot(&self, key: &str) -> Result<u16, CacheError> {
        self.repository.cluster_keyslot(key).await
    }

    /// Count keys in a hash slot
    pub async fn cluster_countkeysinslot(&self, slot: u16) -> Result<u64, CacheError> {
        validate_cluster_slot(slot)?;
        self.repository.cluster_countkeysinslot(slot).await
    }

    /// Get key names from a hash slot
    pub async fn cluster_getkeysinslot(
        &self,
        slot: u16,
        count: u64,
    ) -> Result<Vec<String>, CacheError> {
        validate_cluster_slot(slot)?;
        if count == 0 {
            return Err(CacheError::InvalidInput(
                "count must be a positive integer".to_string(),
            ));
        }
        self.repository.cluster_getkeysinslot(slot, count).await
    }

    /// Per-slot usage statistics for slots assigned to the connected node
    /// (CLUSTER SLOT-STATS, Redis 8.2+).
    ///
    /// Validates the filter contents before dispatch — Redis rejects bare
    /// invocations, and a SLOTSRANGE with `start > end` or `end > 16383` is
    /// rejected upstream.
    pub async fn cluster_slot_stats(
        &self,
        filter: ClusterSlotStatsFilter,
    ) -> Result<Vec<SlotStats>, CacheError> {
        if let ClusterSlotStatsFilter::Range { start, end } = &filter {
            if start > end {
                return Err(CacheError::InvalidInput(
                    "slot_start must be <= slot_end".to_string(),
                ));
            }
            if *end > MAX_CLUSTER_SLOT {
                return Err(CacheError::InvalidInput(
                    "slot range exceeds the maximum slot index 16383".to_string(),
                ));
            }
        }
        if let ClusterSlotStatsFilter::OrderBy { limit, .. } = &filter
            && let Some(n) = limit
            && *n <= 0
        {
            return Err(CacheError::InvalidInput(
                "limit must be a positive integer".to_string(),
            ));
        }
        self.repository.cluster_slot_stats(filter).await
    }
}

fn validate_cluster_slot(slot: u16) -> Result<(), CacheError> {
    if slot > MAX_CLUSTER_SLOT {
        return Err(CacheError::InvalidInput(
            "slot exceeds the maximum slot index 16383".to_string(),
        ));
    }
    Ok(())
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

        async fn cluster_myid(&self) -> Result<String, CacheError> {
            Ok("node-1".to_string())
        }

        async fn cluster_myshardid(&self) -> Result<String, CacheError> {
            Ok("shard-1".to_string())
        }

        async fn cluster_links(&self) -> Result<redis::Value, CacheError> {
            Ok(redis::Value::Array(vec![]))
        }

        async fn cluster_replicas(&self, _node_id: &str) -> Result<Vec<ClusterNode>, CacheError> {
            Ok(vec![ClusterNode {
                id: "replica-1".to_string(),
                address: "127.0.0.1:7002".to_string(),
                flags: "slave".to_string(),
                master_id: Some("node-1".to_string()),
                ping_sent: 0,
                pong_recv: 1000,
                config_epoch: 1,
                link_state: "connected".to_string(),
                slots: vec![],
            }])
        }

        async fn cluster_keyslot(&self, _key: &str) -> Result<u16, CacheError> {
            Ok(12539)
        }

        async fn cluster_countkeysinslot(&self, _slot: u16) -> Result<u64, CacheError> {
            Ok(2)
        }

        async fn cluster_getkeysinslot(
            &self,
            _slot: u16,
            _count: u64,
        ) -> Result<Vec<String>, CacheError> {
            Ok(vec!["key:1".to_string(), "key:2".to_string()])
        }

        async fn cluster_slot_stats(
            &self,
            _filter: ClusterSlotStatsFilter,
        ) -> Result<Vec<SlotStats>, CacheError> {
            Ok(vec![SlotStats {
                slot: 0,
                key_count: 1,
                cpu_usec: 0,
                memory_bytes: 64,
                network_bytes_in: 0,
                network_bytes_out: 0,
            }])
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

    #[tokio::test]
    async fn test_cluster_identity_and_slot_introspection() {
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        assert_eq!(service.cluster_myid().await.unwrap(), "node-1");
        assert_eq!(service.cluster_myshardid().await.unwrap(), "shard-1");
        assert!(matches!(
            service.cluster_links().await.unwrap(),
            redis::Value::Array(v) if v.is_empty()
        ));
        assert_eq!(service.cluster_replicas("node-1").await.unwrap().len(), 1);
        assert_eq!(service.cluster_countkeysinslot(42).await.unwrap(), 2);
        assert_eq!(
            service.cluster_getkeysinslot(42, 2).await.unwrap(),
            vec!["key:1".to_string(), "key:2".to_string()]
        );
    }

    #[tokio::test]
    async fn test_cluster_replicas_rejects_empty_node_id() {
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        let err = service.cluster_replicas("  ").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_cluster_slot_introspection_rejects_invalid_inputs() {
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        let err = service.cluster_countkeysinslot(16_384).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.cluster_getkeysinslot(16_384, 1).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.cluster_getkeysinslot(42, 0).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_slot_stats_range_dispatches_to_repo() {
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        let stats = service
            .cluster_slot_stats(ClusterSlotStatsFilter::Range { start: 0, end: 100 })
            .await
            .expect("range succeeds");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].slot, 0);
    }

    #[tokio::test]
    async fn test_slot_stats_rejects_inverted_range() {
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        let err = service
            .cluster_slot_stats(ClusterSlotStatsFilter::Range { start: 50, end: 10 })
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_slot_stats_rejects_out_of_range_end() {
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        let err = service
            .cluster_slot_stats(ClusterSlotStatsFilter::Range {
                start: 0,
                end: 16384,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_slot_stats_rejects_zero_limit() {
        use crate::domain::repositories::{SlotStatsMetric, SlotStatsOrder};
        let service = ClusterService::new(Arc::new(MockClusterRepo));
        let err = service
            .cluster_slot_stats(ClusterSlotStatsFilter::OrderBy {
                metric: SlotStatsMetric::KeyCount,
                limit: Some(0),
                order: SlotStatsOrder::Desc,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }
}
