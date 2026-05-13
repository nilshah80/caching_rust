//! Cluster API Schemas
//!
//! Request and response types for Redis Cluster endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    ClusterNode, ClusterSlotStatsFilter, SlotStats, SlotStatsMetric, SlotStatsOrder,
};

/// Response for CLUSTER KEYSLOT
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct KeySlotResponse {
    pub key: String,
    pub slot: u16,
}

/// Response for CLUSTER MYID
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterIdResponse {
    pub id: String,
}

/// Response for CLUSTER MYSHARDID
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterShardIdResponse {
    pub shard_id: String,
}

/// Response for CLUSTER LINKS
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterLinksResponse {
    pub links: serde_json::Value,
}

/// Response for CLUSTER REPLICAS
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterReplicasResponse {
    pub replicas: Vec<ClusterNode>,
}

/// Response for CLUSTER COUNTKEYSINSLOT
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterCountKeysInSlotResponse {
    pub slot: u16,
    pub count: u64,
}

/// Query string for CLUSTER GETKEYSINSLOT
#[derive(Debug, Deserialize, ToSchema)]
pub struct ClusterGetKeysInSlotQuery {
    /// Maximum number of key names Redis should return.
    pub count: u64,
}

/// Response for CLUSTER GETKEYSINSLOT
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterGetKeysInSlotResponse {
    pub slot: u16,
    pub count: u64,
    pub keys: Vec<String>,
}

/// Query string for `GET /api/v1/cluster/slot-stats`.
///
/// Redis 8.2 requires either a slot range or an order specifier — bare
/// invocations are rejected. The handler enforces "exactly one mode" via
/// [`SlotStatsQuery::into_filter`].
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct SlotStatsQuery {
    /// Inclusive slot start for SLOTSRANGE.
    #[serde(default)]
    pub slot_start: Option<u16>,
    /// Inclusive slot end for SLOTSRANGE.
    #[serde(default)]
    pub slot_end: Option<u16>,
    /// Sort metric. Accepts snake_case (`key_count`), hyphenated (`key-count`),
    /// or run-together (`keycount`); converted to the Redis wire token before
    /// dispatch.
    #[serde(default)]
    pub order_by: Option<String>,
    /// Optional row cap when paired with `order_by`.
    #[serde(default)]
    pub limit: Option<i64>,
    /// Sort direction (`asc` / `desc`). Defaults to `asc`.
    #[serde(default)]
    pub order: Option<String>,
}

impl SlotStatsQuery {
    /// Convert the query string into a typed filter, rejecting empty queries
    /// and mixed-mode queries before they reach Redis.
    pub fn into_filter(self) -> Result<ClusterSlotStatsFilter, CacheError> {
        let has_range = self.slot_start.is_some() || self.slot_end.is_some();
        let has_order = self.order_by.is_some();

        match (has_range, has_order) {
            (true, true) => Err(CacheError::InvalidInput(
                "slot range and order_by are mutually exclusive".to_string(),
            )),
            (false, false) => Err(CacheError::InvalidInput(
                "CLUSTER SLOT-STATS requires either a slot range or order_by".to_string(),
            )),
            (true, false) => {
                let start = self.slot_start.ok_or_else(|| {
                    CacheError::InvalidInput(
                        "slot_start is required when slot_end is supplied".to_string(),
                    )
                })?;
                let end = self.slot_end.ok_or_else(|| {
                    CacheError::InvalidInput(
                        "slot_end is required when slot_start is supplied".to_string(),
                    )
                })?;
                Ok(ClusterSlotStatsFilter::Range { start, end })
            }
            (false, true) => {
                let raw = self.order_by.unwrap_or_default();
                let metric = SlotStatsMetric::parse(&raw).ok_or_else(|| {
                    CacheError::InvalidInput(format!(
                        "Unsupported order_by metric '{raw}'. Allowed: key_count, cpu_usec, \
                         memory_bytes, network_bytes_in, network_bytes_out"
                    ))
                })?;
                let order = match self.order.as_deref() {
                    None => SlotStatsOrder::Asc,
                    Some(raw) => match raw.to_ascii_lowercase().as_str() {
                        "asc" => SlotStatsOrder::Asc,
                        "desc" => SlotStatsOrder::Desc,
                        other => {
                            return Err(CacheError::InvalidInput(format!(
                                "Unsupported order '{other}'. Use 'asc' or 'desc'."
                            )));
                        }
                    },
                };
                Ok(ClusterSlotStatsFilter::OrderBy {
                    metric,
                    limit: self.limit,
                    order,
                })
            }
        }
    }
}

/// Per-slot record in the response body.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SlotStatsSchema {
    pub slot: u16,
    pub key_count: i64,
    pub cpu_usec: i64,
    pub memory_bytes: u64,
    pub network_bytes_in: i64,
    pub network_bytes_out: i64,
}

impl From<SlotStats> for SlotStatsSchema {
    fn from(s: SlotStats) -> Self {
        Self {
            slot: s.slot,
            key_count: s.key_count,
            cpu_usec: s.cpu_usec,
            memory_bytes: s.memory_bytes,
            network_bytes_in: s.network_bytes_in,
            network_bytes_out: s.network_bytes_out,
        }
    }
}

/// Response body for `GET /api/v1/cluster/slot-stats`.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SlotStatsResponse {
    pub slots: Vec<SlotStatsSchema>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyslot_response_serialization() {
        let resp = KeySlotResponse {
            key: "test".to_string(),
            slot: 12539,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("12539"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_cluster_identity_and_slot_schemas_serialize() {
        let id = ClusterIdResponse {
            id: "node-1".to_string(),
        };
        assert!(serde_json::to_string(&id).unwrap().contains("node-1"));

        let shard = ClusterShardIdResponse {
            shard_id: "shard-1".to_string(),
        };
        assert!(serde_json::to_string(&shard).unwrap().contains("shard-1"));

        let count = ClusterCountKeysInSlotResponse { slot: 42, count: 3 };
        assert!(
            serde_json::to_string(&count)
                .unwrap()
                .contains("\"count\":3")
        );

        let keys = ClusterGetKeysInSlotResponse {
            slot: 42,
            count: 2,
            keys: vec!["a".to_string(), "b".to_string()],
        };
        assert!(serde_json::to_string(&keys).unwrap().contains("\"keys\""));
    }

    #[test]
    fn test_slot_stats_query_rejects_empty() {
        let q = SlotStatsQuery::default();
        assert!(matches!(q.into_filter(), Err(CacheError::InvalidInput(_))));
    }

    #[test]
    fn test_slot_stats_query_rejects_mixed_modes() {
        let q = SlotStatsQuery {
            slot_start: Some(0),
            slot_end: Some(10),
            order_by: Some("key_count".into()),
            ..Default::default()
        };
        assert!(matches!(q.into_filter(), Err(CacheError::InvalidInput(_))));
    }

    #[test]
    fn test_slot_stats_query_range_mode() {
        let q = SlotStatsQuery {
            slot_start: Some(0),
            slot_end: Some(100),
            ..Default::default()
        };
        match q.into_filter().expect("filter") {
            ClusterSlotStatsFilter::Range { start, end } => {
                assert_eq!(start, 0);
                assert_eq!(end, 100);
            }
            other => panic!("expected Range, got {other:?}"),
        }
    }

    #[test]
    fn test_slot_stats_query_rejects_partial_ranges() {
        let q = SlotStatsQuery {
            slot_start: Some(0),
            ..Default::default()
        };
        assert!(matches!(q.into_filter(), Err(CacheError::InvalidInput(_))));

        let q = SlotStatsQuery {
            slot_end: Some(100),
            ..Default::default()
        };
        assert!(matches!(q.into_filter(), Err(CacheError::InvalidInput(_))));
    }

    #[test]
    fn test_slot_stats_query_orderby_default_asc() {
        let q = SlotStatsQuery {
            order_by: Some("memory_bytes".into()),
            limit: Some(10),
            ..Default::default()
        };
        match q.into_filter().expect("filter") {
            ClusterSlotStatsFilter::OrderBy {
                metric,
                limit,
                order,
            } => {
                assert_eq!(metric, SlotStatsMetric::MemoryBytes);
                assert_eq!(metric.as_str(), "MEMORY-BYTES");
                assert_eq!(limit, Some(10));
                assert_eq!(order, SlotStatsOrder::Asc);
            }
            other => panic!("expected OrderBy, got {other:?}"),
        }
    }

    #[test]
    fn test_slot_stats_query_orderby_desc() {
        let q = SlotStatsQuery {
            order_by: Some("CPU-USEC".into()),
            order: Some("DESC".into()),
            ..Default::default()
        };
        match q.into_filter().expect("filter") {
            ClusterSlotStatsFilter::OrderBy { metric, order, .. } => {
                assert_eq!(metric, SlotStatsMetric::CpuUsec);
                assert_eq!(order, SlotStatsOrder::Desc);
            }
            other => panic!("expected OrderBy, got {other:?}"),
        }
    }

    #[test]
    fn test_slot_stats_query_unknown_metric() {
        let q = SlotStatsQuery {
            order_by: Some("does_not_exist".into()),
            ..Default::default()
        };
        assert!(matches!(q.into_filter(), Err(CacheError::InvalidInput(_))));
    }

    #[test]
    fn test_slot_stats_query_unknown_order() {
        let q = SlotStatsQuery {
            order_by: Some("key_count".into()),
            order: Some("sideways".into()),
            ..Default::default()
        };
        assert!(matches!(q.into_filter(), Err(CacheError::InvalidInput(_))));
    }

    #[test]
    fn test_slot_stats_schema_from_entity() {
        let s = SlotStats {
            slot: 5,
            key_count: 10,
            cpu_usec: 20,
            memory_bytes: 30,
            network_bytes_in: 40,
            network_bytes_out: 50,
        };
        let schema: SlotStatsSchema = s.into();
        assert_eq!(schema.slot, 5);
        assert_eq!(schema.memory_bytes, 30);
    }
}
