//! Stream Domain Entities
//!
//! Core business objects for Redis Streams.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// A single entry in a Redis Stream
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StreamEntry {
    /// Entry ID (format: timestamp-sequence, e.g., "1704000001234-0")
    pub id: String,

    /// Entry fields as key-value pairs
    pub fields: HashMap<String, String>,
}

/// Information about a Redis Stream
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StreamInfo {
    /// Number of entries in the stream
    pub length: i64,

    /// Number of consumer groups
    pub groups: i64,

    /// ID of the first entry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_entry_id: Option<String>,

    /// ID of the last entry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_entry_id: Option<String>,

    /// First entry (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_entry: Option<StreamEntry>,

    /// Last entry (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_entry: Option<StreamEntry>,

    /// Maximum deleted entry ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_deleted_entry_id: Option<String>,

    /// Number of entries added to the stream
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries_added: Option<i64>,

    /// Radix tree keys
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radix_tree_keys: Option<i64>,

    /// Radix tree nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radix_tree_nodes: Option<i64>,
}

/// Information about a consumer group
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsumerGroupInfo {
    /// Name of the consumer group
    pub name: String,

    /// Number of consumers in the group
    pub consumers: i64,

    /// Number of pending entries (delivered but not acknowledged)
    pub pending: i64,

    /// Last delivered ID
    pub last_delivered_id: String,

    /// Number of entries read by this group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries_read: Option<i64>,

    /// Lag behind the stream
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lag: Option<i64>,
}

/// Information about a consumer in a group
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsumerInfo {
    /// Name of the consumer
    pub name: String,

    /// Number of pending entries for this consumer
    pub pending: i64,

    /// Idle time in milliseconds
    pub idle_ms: i64,

    /// Time since last interaction in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inactive_ms: Option<i64>,
}

/// A pending entry that was delivered but not acknowledged
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingEntry {
    /// Entry ID
    pub id: String,

    /// Consumer name that owns this pending entry
    pub consumer: String,

    /// Time since delivery in milliseconds
    pub idle_time_ms: i64,

    /// Number of times this entry was delivered
    pub delivery_count: i64,
}

/// Summary of pending entries for a consumer group
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingSummary {
    /// Total number of pending entries
    pub count: i64,

    /// Smallest pending entry ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_id: Option<String>,

    /// Largest pending entry ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_id: Option<String>,

    /// Pending count per consumer
    pub consumers: HashMap<String, i64>,
}

/// Result of XREAD/XREADGROUP operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StreamReadResult {
    /// Stream key
    pub key: String,

    /// Entries read from this stream
    pub entries: Vec<StreamEntry>,
}

/// Result of XCLAIM operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaimResult {
    /// Claimed entries
    pub entries: Vec<StreamEntry>,
}

/// Result of XAUTOCLAIM operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutoClaimResult {
    /// Next ID to use for subsequent XAUTOCLAIM calls
    pub next_id: String,

    /// Claimed entries
    pub entries: Vec<StreamEntry>,

    /// IDs that no longer exist in the stream (deleted entries)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub deleted_ids: Vec<String>,
}

/// Options for XADD command
#[derive(Debug, Clone, Default)]
pub struct XAddOptions {
    /// Use specific ID instead of auto-generated
    pub id: Option<String>,

    /// Approximate trimming with MAXLEN
    pub maxlen: Option<i64>,

    /// Approximate trimming with MINID
    pub minid: Option<String>,

    /// Whether to use approximate trimming (~)
    pub approximate: bool,

    /// Only add if stream already exists (NOMKSTREAM)
    pub no_mkstream: bool,

    /// Limit the number of entries to trim
    pub limit: Option<i64>,
}

/// Options for XTRIM command
#[derive(Debug, Clone)]
pub enum XTrimStrategy {
    /// Keep at most N entries
    MaxLen { count: i64, approximate: bool },

    /// Remove entries older than the given ID
    MinId { id: String, approximate: bool },
}

/// Options for XREAD command
#[derive(Debug, Clone, Default)]
pub struct XReadOptions {
    /// Maximum number of entries to return per stream
    pub count: Option<i64>,

    /// Block for this many milliseconds (0 = block forever)
    pub block_ms: Option<i64>,
}

/// Options for XREADGROUP command
#[derive(Debug, Clone)]
pub struct XReadGroupOptions {
    /// Maximum number of entries to return per stream
    pub count: Option<i64>,

    /// Block for this many milliseconds (0 = block forever)
    pub block_ms: Option<i64>,

    /// Don't update last-delivered-id when acknowledging
    pub no_ack: bool,
}

impl Default for XReadGroupOptions {
    fn default() -> Self {
        Self {
            count: None,
            block_ms: None,
            no_ack: false,
        }
    }
}

/// Options for XGROUP CREATE command
#[derive(Debug, Clone, Default)]
pub struct XGroupCreateOptions {
    /// Create stream if it doesn't exist
    pub mkstream: bool,

    /// Number of entries to read from the stream for the group
    pub entries_read: Option<i64>,
}

/// Options for XCLAIM command
#[derive(Debug, Clone)]
pub struct XClaimOptions {
    /// Minimum idle time in milliseconds to claim
    pub min_idle_time_ms: i64,

    /// Set idle time to this value
    pub idle_ms: Option<i64>,

    /// Set delivery count
    pub time_ms: Option<i64>,

    /// Retry count
    pub retry_count: Option<i64>,

    /// Force claim even if ID is not pending
    pub force: bool,

    /// Only return IDs, not full entries
    pub just_id: bool,

    /// Claim only if last delivery ID matches
    pub last_id: Option<String>,
}

impl Default for XClaimOptions {
    fn default() -> Self {
        Self {
            min_idle_time_ms: 0,
            idle_ms: None,
            time_ms: None,
            retry_count: None,
            force: false,
            just_id: false,
            last_id: None,
        }
    }
}

/// Options for XAUTOCLAIM command
#[derive(Debug, Clone)]
pub struct XAutoClaimOptions {
    /// Maximum number of entries to claim
    pub count: Option<i64>,

    /// Only return IDs, not full entries
    pub just_id: bool,
}

impl Default for XAutoClaimOptions {
    fn default() -> Self {
        Self {
            count: None,
            just_id: false,
        }
    }
}

/// Options for XPENDING command
#[derive(Debug, Clone, Default)]
pub struct XPendingOptions {
    /// Minimum ID to return
    pub start: Option<String>,

    /// Maximum ID to return
    pub end: Option<String>,

    /// Maximum number of entries to return
    pub count: Option<i64>,

    /// Filter by consumer name
    pub consumer: Option<String>,

    /// Minimum idle time filter
    pub idle_ms: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_entry_serialization() {
        let mut fields = HashMap::new();
        fields.insert("user".to_string(), "alice".to_string());
        fields.insert("action".to_string(), "login".to_string());

        let entry = StreamEntry {
            id: "1704000001234-0".to_string(),
            fields,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("1704000001234-0"));
        assert!(json.contains("alice"));
    }

    #[test]
    fn test_pending_entry() {
        let pending = PendingEntry {
            id: "1704000001234-0".to_string(),
            consumer: "worker-1".to_string(),
            idle_time_ms: 5000,
            delivery_count: 2,
        };

        assert_eq!(pending.consumer, "worker-1");
        assert_eq!(pending.delivery_count, 2);
    }

    #[test]
    fn test_xautoclaim_options_default() {
        let options = XAutoClaimOptions::default();
        assert!(options.count.is_none());
        assert!(!options.just_id);
    }

    #[test]
    fn test_xadd_options_default() {
        let opts = XAddOptions::default();
        assert!(opts.id.is_none());
        assert!(opts.maxlen.is_none());
        assert!(!opts.approximate);
        assert!(!opts.no_mkstream);
    }
}
