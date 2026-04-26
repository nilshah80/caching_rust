//! Redis Cluster Repository Implementation
//!
//! Implements cluster info operations using direct Redis commands.
//! Uses the standalone pool connection (CLUSTER INFO works on any node).

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    ClusterEndpoint, ClusterInfo, ClusterNode, ClusterRepository, ClusterSlotRange,
    ClusterSlotStatsFilter, SlotStats,
};
use crate::infrastructure::redis::connection::InstrumentedPool;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RedisClusterRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisClusterRepository {
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ClusterRepository for RedisClusterRepository {
    async fn cluster_info(&self) -> Result<ClusterInfo, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let info: String = redis::cmd("CLUSTER")
            .arg("INFO")
            .query_async(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;

        parse_cluster_info(&info)
    }

    async fn cluster_nodes(&self) -> Result<Vec<ClusterNode>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let nodes: String = redis::cmd("CLUSTER")
            .arg("NODES")
            .query_async(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;

        Ok(parse_cluster_nodes(&nodes))
    }

    async fn cluster_slots(&self) -> Result<Vec<ClusterSlotRange>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let slots: redis::Value = redis::cmd("CLUSTER")
            .arg("SLOTS")
            .query_async(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;

        Ok(parse_cluster_slots(&slots))
    }

    async fn cluster_shards(&self) -> Result<redis::Value, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let shards: redis::Value = redis::cmd("CLUSTER")
            .arg("SHARDS")
            .query_async(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;

        Ok(shards)
    }

    async fn cluster_keyslot(&self, key: &str) -> Result<u16, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let slot: u16 = redis::cmd("CLUSTER")
            .arg("KEYSLOT")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;

        Ok(slot)
    }

    async fn cluster_slot_stats(
        &self,
        filter: ClusterSlotStatsFilter,
    ) -> Result<Vec<SlotStats>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let mut cmd = redis::cmd("CLUSTER");
        cmd.arg("SLOT-STATS");
        match filter {
            ClusterSlotStatsFilter::Range { start, end } => {
                cmd.arg("SLOTSRANGE").arg(start).arg(end);
            }
            ClusterSlotStatsFilter::OrderBy {
                metric,
                limit,
                order,
            } => {
                cmd.arg("ORDERBY").arg(metric.as_str());
                if let Some(n) = limit {
                    cmd.arg("LIMIT").arg(n);
                }
                cmd.arg(order.as_str());
            }
        }
        let raw: redis::Value = cmd
            .query_async(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;
        parse_slot_stats(raw)
    }
}

/// Parse the CLUSTER SLOT-STATS reply into typed slot records.
///
/// Outer array contains entries shaped as `[slot_int, [name, value, name, value, ...]]`.
/// Metric names use hyphens (`key-count`, `cpu-usec`, `memory-bytes`,
/// `network-bytes-in`, `network-bytes-out`); unknown fields are ignored so the
/// parser stays forward-compatible.
fn parse_slot_stats(value: redis::Value) -> Result<Vec<SlotStats>, CacheError> {
    let outer = match value {
        redis::Value::Array(a) => a,
        _ => {
            return Err(CacheError::Internal(
                "Unexpected reply shape from CLUSTER SLOT-STATS".to_string(),
            ));
        }
    };
    let mut out = Vec::with_capacity(outer.len());
    for entry in outer {
        let parts = match entry {
            redis::Value::Array(p) if p.len() >= 2 => p,
            _ => continue,
        };
        let slot = match &parts[0] {
            redis::Value::Int(n) => *n as u16,
            _ => continue,
        };
        let mut stats = SlotStats {
            slot,
            key_count: 0,
            cpu_usec: 0,
            memory_bytes: 0,
            network_bytes_in: 0,
            network_bytes_out: 0,
        };
        if let redis::Value::Array(kv) = &parts[1] {
            let mut iter = kv.iter();
            while let (Some(name_v), Some(value_v)) = (iter.next(), iter.next()) {
                let name = match name_v {
                    redis::Value::BulkString(b) => String::from_utf8_lossy(b).into_owned(),
                    redis::Value::SimpleString(s) => s.clone(),
                    _ => continue,
                };
                let n = match value_v {
                    redis::Value::Int(n) => *n,
                    _ => continue,
                };
                // Redis docs spell the metric labels in uppercase
                // (KEY-COUNT, CPU-USEC, MEMORY-BYTES, …); some builds may
                // ship them lowercase. Normalize before matching so the
                // parser is case-insensitive.
                match name.to_ascii_lowercase().as_str() {
                    "key-count" => stats.key_count = n,
                    "cpu-usec" => stats.cpu_usec = n,
                    "memory-bytes" => stats.memory_bytes = n.max(0) as u64,
                    "network-bytes-in" => stats.network_bytes_in = n,
                    "network-bytes-out" => stats.network_bytes_out = n,
                    _ => {}
                }
            }
        }
        out.push(stats);
    }
    Ok(out)
}

fn parse_cluster_info(info: &str) -> Result<ClusterInfo, CacheError> {
    let mut result = ClusterInfo {
        cluster_state: String::new(),
        cluster_slots_assigned: 0,
        cluster_slots_ok: 0,
        cluster_slots_pfail: 0,
        cluster_slots_fail: 0,
        cluster_known_nodes: 0,
        cluster_size: 0,
        cluster_current_epoch: 0,
        cluster_my_epoch: 0,
    };

    for line in info.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            match key {
                "cluster_state" => result.cluster_state = value.to_string(),
                "cluster_slots_assigned" => {
                    result.cluster_slots_assigned = value.parse().unwrap_or(0);
                }
                "cluster_slots_ok" => result.cluster_slots_ok = value.parse().unwrap_or(0),
                "cluster_slots_pfail" => result.cluster_slots_pfail = value.parse().unwrap_or(0),
                "cluster_slots_fail" => result.cluster_slots_fail = value.parse().unwrap_or(0),
                "cluster_known_nodes" => result.cluster_known_nodes = value.parse().unwrap_or(0),
                "cluster_size" => result.cluster_size = value.parse().unwrap_or(0),
                "cluster_current_epoch" => {
                    result.cluster_current_epoch = value.parse().unwrap_or(0);
                }
                "cluster_my_epoch" => result.cluster_my_epoch = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }

    Ok(result)
}

fn parse_cluster_nodes(nodes_str: &str) -> Vec<ClusterNode> {
    nodes_str
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 8 {
                return None;
            }
            Some(ClusterNode {
                id: parts[0].to_string(),
                address: parts[1].to_string(),
                flags: parts[2].to_string(),
                master_id: if parts[3] == "-" {
                    None
                } else {
                    Some(parts[3].to_string())
                },
                ping_sent: parts[4].parse().unwrap_or(0),
                pong_recv: parts[5].parse().unwrap_or(0),
                config_epoch: parts[6].parse().unwrap_or(0),
                link_state: parts[7].to_string(),
                slots: parts[8..].iter().map(|s| s.to_string()).collect(),
            })
        })
        .collect()
}

fn parse_cluster_slots(value: &redis::Value) -> Vec<ClusterSlotRange> {
    let mut ranges = Vec::new();

    if let redis::Value::Array(slots) = value {
        for slot_entry in slots {
            if let redis::Value::Array(parts) = slot_entry
                && parts.len() >= 3
            {
                let start = extract_u64(&parts[0]);
                let end = extract_u64(&parts[1]);
                let master = extract_endpoint(&parts[2]);

                let replicas: Vec<ClusterEndpoint> =
                    parts[3..].iter().map(extract_endpoint).collect();

                ranges.push(ClusterSlotRange {
                    start,
                    end,
                    master,
                    replicas,
                });
            }
        }
    }

    ranges
}

fn extract_u64(value: &redis::Value) -> u64 {
    match value {
        redis::Value::Int(n) => *n as u64,
        _ => 0,
    }
}

fn extract_endpoint(value: &redis::Value) -> ClusterEndpoint {
    if let redis::Value::Array(parts) = value {
        let host = match parts.first() {
            Some(redis::Value::BulkString(b)) => String::from_utf8_lossy(b).to_string(),
            _ => String::new(),
        };
        let port = parts.get(1).map(|v| extract_u64(v) as u16).unwrap_or(0);
        let node_id = parts.get(2).and_then(|v| match v {
            redis::Value::BulkString(b) => {
                let s = String::from_utf8_lossy(b).to_string();
                if s.is_empty() { None } else { Some(s) }
            }
            _ => None,
        });
        ClusterEndpoint {
            host,
            port,
            node_id,
        }
    } else {
        ClusterEndpoint {
            host: String::new(),
            port: 0,
            node_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cluster_info() {
        let info = "\
cluster_state:ok\r
cluster_slots_assigned:16384\r
cluster_slots_ok:16384\r
cluster_slots_pfail:0\r
cluster_slots_fail:0\r
cluster_known_nodes:6\r
cluster_size:3\r
cluster_current_epoch:6\r
cluster_my_epoch:1\r
";
        let result = parse_cluster_info(info).unwrap();
        assert_eq!(result.cluster_state, "ok");
        assert_eq!(result.cluster_slots_assigned, 16384);
        assert_eq!(result.cluster_known_nodes, 6);
        assert_eq!(result.cluster_size, 3);
    }

    #[test]
    fn test_parse_cluster_nodes() {
        let nodes = "abc123 127.0.0.1:7001@17001 master - 0 1000 1 connected 0-5460\n\
                     def456 127.0.0.1:7002@17002 master - 0 1000 2 connected 5461-10922\n";
        let result = parse_cluster_nodes(nodes);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "abc123");
        assert_eq!(result[0].address, "127.0.0.1:7001@17001");
        assert!(result[0].flags.contains("master"));
        assert!(result[0].master_id.is_none());
        assert_eq!(result[0].slots, vec!["0-5460"]);
        assert_eq!(result[1].id, "def456");
    }

    #[test]
    fn test_parse_cluster_nodes_with_replica() {
        let nodes = "abc123 127.0.0.1:7001@17001 slave parent123 0 1000 1 connected\n";
        let result = parse_cluster_nodes(nodes);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].master_id, Some("parent123".to_string()));
        assert!(result[0].slots.is_empty());
    }

    #[test]
    fn test_parse_cluster_info_empty() {
        let result = parse_cluster_info("").unwrap();
        assert!(result.cluster_state.is_empty());
        assert_eq!(result.cluster_slots_assigned, 0);
    }

    /// Build one CLUSTER SLOT-STATS slot entry with the given metric label casing.
    fn slot_entry(slot: i64, label_case: fn(&str) -> String) -> redis::Value {
        let label = |s: &str| redis::Value::BulkString(label_case(s).into_bytes());
        redis::Value::Array(vec![
            redis::Value::Int(slot),
            redis::Value::Array(vec![
                label("key-count"),
                redis::Value::Int(11),
                label("cpu-usec"),
                redis::Value::Int(22),
                label("memory-bytes"),
                redis::Value::Int(33),
                label("network-bytes-in"),
                redis::Value::Int(44),
                label("network-bytes-out"),
                redis::Value::Int(55),
            ]),
        ])
    }

    #[test]
    fn test_parse_slot_stats_uppercase_labels() {
        // Redis docs spell metric labels in UPPERCASE; the parser must
        // accept that wire form even though the match arms are lowercase.
        let reply = redis::Value::Array(vec![slot_entry(7, |s| s.to_ascii_uppercase())]);
        let stats = parse_slot_stats(reply).expect("parse");
        assert_eq!(stats.len(), 1);
        let s = &stats[0];
        assert_eq!(s.slot, 7);
        assert_eq!(s.key_count, 11);
        assert_eq!(s.cpu_usec, 22);
        assert_eq!(s.memory_bytes, 33);
        assert_eq!(s.network_bytes_in, 44);
        assert_eq!(s.network_bytes_out, 55);
    }

    #[test]
    fn test_parse_slot_stats_lowercase_labels() {
        let reply = redis::Value::Array(vec![slot_entry(0, str::to_string)]);
        let stats = parse_slot_stats(reply).expect("parse");
        assert_eq!(stats.len(), 1);
        let s = &stats[0];
        assert_eq!(s.slot, 0);
        assert_eq!(s.key_count, 11);
        assert_eq!(s.memory_bytes, 33);
        assert_eq!(s.network_bytes_out, 55);
    }

    #[test]
    fn test_parse_slot_stats_ignores_unknown_metric() {
        // Forward-compat: unknown metric names are silently ignored, not
        // treated as parse errors.
        let reply = redis::Value::Array(vec![redis::Value::Array(vec![
            redis::Value::Int(3),
            redis::Value::Array(vec![
                redis::Value::BulkString(b"key-count".to_vec()),
                redis::Value::Int(9),
                redis::Value::BulkString(b"FUTURE-METRIC".to_vec()),
                redis::Value::Int(123_456),
            ]),
        ])]);
        let stats = parse_slot_stats(reply).expect("parse");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].slot, 3);
        assert_eq!(stats[0].key_count, 9);
        assert_eq!(stats[0].cpu_usec, 0);
    }

    #[test]
    fn test_parse_slot_stats_rejects_non_array_top_level() {
        let err = parse_slot_stats(redis::Value::Int(5)).unwrap_err();
        assert!(matches!(
            err,
            crate::domain::errors::CacheError::Internal(_)
        ));
    }
}
