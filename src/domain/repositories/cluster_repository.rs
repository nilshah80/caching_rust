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

/// Per-slot usage statistics returned by CLUSTER SLOT-STATS (Redis 8.2+).
///
/// Field names follow the Redis reply: KEY-COUNT, CPU-USEC, MEMORY-BYTES,
/// NETWORK-BYTES-IN, NETWORK-BYTES-OUT.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SlotStats {
    /// Slot number (0–16383)
    pub slot: u16,
    /// Number of keys assigned to this slot
    pub key_count: i64,
    /// Cumulative CPU time spent on this slot, microseconds
    pub cpu_usec: i64,
    /// Memory used by keys assigned to this slot, bytes
    pub memory_bytes: u64,
    /// Network bytes ingested for this slot
    pub network_bytes_in: i64,
    /// Network bytes emitted for this slot
    pub network_bytes_out: i64,
}

/// Metrics that CLUSTER SLOT-STATS ORDERBY accepts. Each variant maps to the
/// exact wire token Redis expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotStatsMetric {
    KeyCount,
    CpuUsec,
    MemoryBytes,
    NetworkBytesIn,
    NetworkBytesOut,
}

impl SlotStatsMetric {
    /// Redis wire token for this metric.
    pub fn as_str(&self) -> &'static str {
        match self {
            SlotStatsMetric::KeyCount => "KEY-COUNT",
            SlotStatsMetric::CpuUsec => "CPU-USEC",
            SlotStatsMetric::MemoryBytes => "MEMORY-BYTES",
            SlotStatsMetric::NetworkBytesIn => "NETWORK-BYTES-IN",
            SlotStatsMetric::NetworkBytesOut => "NETWORK-BYTES-OUT",
        }
    }

    /// Parse a snake_case or hyphenated form (case-insensitive) into a metric.
    pub fn parse(input: &str) -> Option<Self> {
        match input.to_ascii_lowercase().as_str() {
            "key_count" | "key-count" | "keycount" => Some(Self::KeyCount),
            "cpu_usec" | "cpu-usec" | "cpuusec" => Some(Self::CpuUsec),
            "memory_bytes" | "memory-bytes" | "memorybytes" => Some(Self::MemoryBytes),
            "network_bytes_in" | "network-bytes-in" | "networkbytesin" => {
                Some(Self::NetworkBytesIn)
            }
            "network_bytes_out" | "network-bytes-out" | "networkbytesout" => {
                Some(Self::NetworkBytesOut)
            }
            _ => None,
        }
    }
}

/// Sort direction for the CLUSTER SLOT-STATS ORDERBY clause.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SlotStatsOrder {
    #[default]
    Asc,
    Desc,
}

impl SlotStatsOrder {
    pub fn as_str(&self) -> &'static str {
        match self {
            SlotStatsOrder::Asc => "ASC",
            SlotStatsOrder::Desc => "DESC",
        }
    }
}

/// Required filter for CLUSTER SLOT-STATS. Redis rejects bare invocations, so
/// the API forces the caller to pick one mode.
#[derive(Debug, Clone)]
pub enum ClusterSlotStatsFilter {
    /// SLOTSRANGE start end — inclusive on both bounds, 0..=16383
    Range { start: u16, end: u16 },
    /// ORDERBY metric [LIMIT n] [ASC|DESC]
    OrderBy {
        metric: SlotStatsMetric,
        limit: Option<i64>,
        order: SlotStatsOrder,
    },
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

    /// Get per-slot usage statistics for slots assigned to the connected node
    /// (CLUSTER SLOT-STATS, Redis 8.2+).
    async fn cluster_slot_stats(
        &self,
        filter: ClusterSlotStatsFilter,
    ) -> Result<Vec<SlotStats>, CacheError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_stats_metric_wire_tokens() {
        assert_eq!(SlotStatsMetric::KeyCount.as_str(), "KEY-COUNT");
        assert_eq!(SlotStatsMetric::CpuUsec.as_str(), "CPU-USEC");
        assert_eq!(SlotStatsMetric::MemoryBytes.as_str(), "MEMORY-BYTES");
        assert_eq!(SlotStatsMetric::NetworkBytesIn.as_str(), "NETWORK-BYTES-IN");
        assert_eq!(
            SlotStatsMetric::NetworkBytesOut.as_str(),
            "NETWORK-BYTES-OUT"
        );
    }

    #[test]
    fn test_slot_stats_metric_parse_variants() {
        assert_eq!(
            SlotStatsMetric::parse("key_count"),
            Some(SlotStatsMetric::KeyCount)
        );
        assert_eq!(
            SlotStatsMetric::parse("cpu-usec"),
            Some(SlotStatsMetric::CpuUsec)
        );
        assert_eq!(
            SlotStatsMetric::parse("MEMORYBYTES"),
            Some(SlotStatsMetric::MemoryBytes)
        );
        assert_eq!(
            SlotStatsMetric::parse("network_bytes_in"),
            Some(SlotStatsMetric::NetworkBytesIn)
        );
        assert_eq!(
            SlotStatsMetric::parse("network-bytes-out"),
            Some(SlotStatsMetric::NetworkBytesOut)
        );
        assert_eq!(SlotStatsMetric::parse("unknown"), None);
    }

    #[test]
    fn test_slot_stats_order_wire_tokens() {
        assert_eq!(SlotStatsOrder::default(), SlotStatsOrder::Asc);
        assert_eq!(SlotStatsOrder::Asc.as_str(), "ASC");
        assert_eq!(SlotStatsOrder::Desc.as_str(), "DESC");
    }
}
