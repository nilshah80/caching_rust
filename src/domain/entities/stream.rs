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
    /// Use specific ID instead of auto-generated. **Mutually exclusive with
    /// `idmp`** — Redis 8.6 IDMP/IDMPAUTO require an auto-generated `*` ID.
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

    /// Reference policy for the trim sub-clause (Redis 8.2+). Only meaningful
    /// when `maxlen` or `minid` is set; service-layer rejects otherwise.
    pub reference_policy: Option<XAckDelMode>,

    /// Idempotent producer mode (Redis 8.6+). Mutually exclusive with `id`.
    pub idmp: Option<XAddIdmp>,
}

/// XADD idempotent-producer mode (Redis 8.6+).
///
/// On the wire, encodes as either `IDMPAUTO producer-id` (auto-derived
/// idempotent id) or `IDMP producer-id idempotent-id` (explicit id). Both
/// modes require the message be added with an auto-generated `*` stream ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XAddIdmp {
    /// `IDMP producer-id idempotent-id` — slightly faster, caller manages
    /// uniqueness of the idempotent id.
    Manual {
        producer_id: String,
        idempotent_id: String,
    },
    /// `IDMPAUTO producer-id` — Redis derives the idempotent id from the
    /// message body via hash.
    Auto { producer_id: String },
}

/// Options for XTRIM command
#[derive(Debug, Clone)]
pub enum XTrimStrategy {
    /// Keep at most N entries
    MaxLen { count: i64, approximate: bool },

    /// Remove entries older than the given ID
    MinId { id: String, approximate: bool },
}

/// Options for XTRIM (Redis 8.2+ surface).
///
/// `XTrimStrategy` carries the threshold; this wrapper adds the optional
/// `LIMIT count` and `KEEPREF | DELREF | ACKED` reference-policy flag
/// introduced in Redis 8.2. Wire order is `XTRIM key STRATEGY [LIMIT n] [POLICY]`.
#[derive(Debug, Clone)]
pub struct XTrimOptions {
    pub strategy: XTrimStrategy,
    /// `LIMIT count` — soft cap on the number of entries trimmed in one call.
    pub limit: Option<i64>,
    /// `KEEPREF | DELREF | ACKED` (Redis 8.2+).
    pub reference_policy: Option<XAckDelMode>,
}

/// XCFGSET configuration parameters (Redis 8.6+ stream IDMP config).
///
/// At least one of `idmp_duration_seconds` / `idmp_max_size` must be set —
/// service-layer guards reject empty payloads. Calling XCFGSET clears all
/// existing producer IDMP maps for the stream.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XCfgSetOptions {
    /// `IDMP-DURATION` in seconds, valid range `1..=86400` (default 100).
    pub idmp_duration_seconds: Option<u64>,
    /// `IDMP-MAXSIZE` in entries, valid range `1..=10000`.
    pub idmp_max_size: Option<u64>,
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
#[derive(Debug, Clone, Default)]
pub struct XReadGroupOptions {
    /// Maximum number of entries to return per stream
    pub count: Option<i64>,

    /// Block for this many milliseconds (0 = block forever)
    pub block_ms: Option<i64>,

    /// Don't update last-delivered-id when acknowledging
    pub no_ack: bool,
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
#[derive(Debug, Clone, Default)]
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

/// Options for XAUTOCLAIM command
#[derive(Debug, Clone, Default)]
pub struct XAutoClaimOptions {
    /// Maximum number of entries to claim
    pub count: Option<i64>,

    /// Only return IDs, not full entries
    pub just_id: bool,
}

/// Stream reference policy — controls how consumer-group references are
/// handled when an entry is deleted or trimmed (Redis 8.2+).
///
/// Used by `XACKDEL`, `XDELEX`, `XTRIM` (8.2 reference-policy flag), and
/// `XADD`'s trim sub-clause. The variant name retains the `XAckDel` prefix
/// for backward compatibility with the public `XAckDelModeSchema` JSON
/// shape; renaming to `StreamRefPolicy` is tracked as a future cleanup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum XAckDelMode {
    /// Acknowledge in the current group, delete from the stream, but keep
    /// existing references in *other* groups' PEL. Redis default.
    #[default]
    KeepRef,
    /// Acknowledge + delete + remove all references from every group's PEL.
    DelRef,
    /// Only delete the entry if every group has already acknowledged it.
    /// Otherwise leave it intact and return the `dangling` status.
    Acked,
}

impl XAckDelMode {
    /// Wire token for the optional `KEEPREF | DELREF | ACKED` flag.
    pub fn as_str(&self) -> &'static str {
        match self {
            XAckDelMode::KeepRef => "KEEPREF",
            XAckDelMode::DelRef => "DELREF",
            XAckDelMode::Acked => "ACKED",
        }
    }
}

/// Per-entry result for XACKDEL.
///
/// `status` is forwarded verbatim from Redis (1 = deleted, -1 = missing,
/// 2 = dangling, any future code surfaced as-is). `status_label` is a
/// human-readable rendering — handy for clients that don't want to interpret
/// integers, while still keeping the int around for forward compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XAckDelEntryResult {
    pub id: String,
    pub status: i64,
    pub status_label: String,
}

impl XAckDelEntryResult {
    /// Build a result, deriving the human label from the numeric status.
    pub fn new(id: String, status: i64) -> Self {
        let status_label = match status {
            1 => "deleted",
            -1 => "missing",
            2 => "dangling",
            _ => "unknown",
        }
        .to_string();
        Self {
            id,
            status,
            status_label,
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

    #[test]
    fn test_xackdel_mode_wire_tokens() {
        assert_eq!(XAckDelMode::KeepRef.as_str(), "KEEPREF");
        assert_eq!(XAckDelMode::DelRef.as_str(), "DELREF");
        assert_eq!(XAckDelMode::Acked.as_str(), "ACKED");
        assert_eq!(XAckDelMode::default(), XAckDelMode::KeepRef);
    }

    #[test]
    fn test_xackdel_entry_result_status_labels() {
        let cases = [
            (1, "deleted"),
            (-1, "missing"),
            (2, "dangling"),
            (99, "unknown"),
            (0, "unknown"),
        ];
        for (status, label) in cases {
            let entry = XAckDelEntryResult::new("1-0".to_string(), status);
            assert_eq!(entry.status, status);
            assert_eq!(entry.status_label, label, "status code {status}");
        }
    }
}
