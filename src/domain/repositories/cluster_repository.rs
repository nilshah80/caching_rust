//! Cluster Repository Trait
//!
//! Defines the interface for Redis Cluster operations.

use crate::domain::errors::CacheError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Parsed CLUSTER INFO response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterInfo {
    pub cluster_state: String,
    pub cluster_slots_assigned: u64,
    pub cluster_slots_ok: u64,
    pub cluster_slots_pfail: u64,
    pub cluster_slots_fail: u64,
    pub cluster_known_nodes: u64,
    pub cluster_size: u64,
    pub cluster_current_epoch: u64,
    pub cluster_my_epoch: u64,
}

/// A single node from CLUSTER NODES
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterNode {
    pub id: String,
    pub address: String,
    pub flags: String,
    pub master_id: Option<String>,
    pub ping_sent: u64,
    pub pong_recv: u64,
    pub config_epoch: u64,
    pub link_state: String,
    pub slots: Vec<String>,
}

/// A slot range entry from CLUSTER SLOTS
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterSlotRange {
    pub start: u64,
    pub end: u64,
    pub master: ClusterEndpoint,
    pub replicas: Vec<ClusterEndpoint>,
}

/// A node endpoint (host + port)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterEndpoint {
    pub host: String,
    pub port: u16,
    pub node_id: Option<String>,
}

#[async_trait]
pub trait ClusterRepository: Send + Sync {
    /// Get cluster info (CLUSTER INFO)
    async fn cluster_info(&self) -> Result<ClusterInfo, CacheError>;

    /// Get cluster nodes (CLUSTER NODES)
    async fn cluster_nodes(&self) -> Result<Vec<ClusterNode>, CacheError>;

    /// Get cluster slot mapping (CLUSTER SLOTS)
    async fn cluster_slots(&self) -> Result<Vec<ClusterSlotRange>, CacheError>;

    /// Get cluster shards (CLUSTER SHARDS, Redis 7.0+)
    async fn cluster_shards(&self) -> Result<redis::Value, CacheError>;

    /// Get the hash slot for a key (CLUSTER KEYSLOT)
    async fn cluster_keyslot(&self, key: &str) -> Result<u16, CacheError>;
}
