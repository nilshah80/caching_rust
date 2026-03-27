//! Stream Schemas
//!
//! Request and response types for stream API endpoints.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::domain::entities::{
    AutoClaimResult, ClaimResult, ConsumerGroupInfo, ConsumerInfo, PendingEntry, PendingSummary,
    StreamEntry, StreamInfo, StreamReadResult, XAddOptions, XAutoClaimOptions, XClaimOptions,
    XGroupCreateOptions, XPendingOptions, XReadGroupOptions, XReadOptions, XTrimStrategy,
};

// ========== XADD Schemas ==========

/// Request to add entry to a stream
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct StreamAddRequest {
    /// Entry fields as key-value pairs
    #[validate(length(min = 1, message = "At least one field is required"))]
    pub fields: HashMap<String, String>,

    /// Specific entry ID (optional, auto-generated if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Maximum stream length (approximate trimming with ~)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maxlen: Option<i64>,

    /// Minimum ID to keep (approximate trimming with ~)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minid: Option<String>,

    /// Use approximate trimming (~) for maxlen/minid
    #[serde(default = "default_true")]
    pub approximate: bool,

    /// Only add if stream already exists
    #[serde(default)]
    pub no_mkstream: bool,

    /// Limit trimming operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
}

fn default_true() -> bool {
    true
}

impl From<StreamAddRequest> for XAddOptions {
    fn from(req: StreamAddRequest) -> Self {
        XAddOptions {
            id: req.id,
            maxlen: req.maxlen,
            minid: req.minid,
            approximate: req.approximate,
            no_mkstream: req.no_mkstream,
            limit: req.limit,
        }
    }
}

/// Response from XADD
#[derive(Debug, Serialize, ToSchema)]
pub struct StreamAddResponse {
    /// ID of the added entry
    pub id: String,
}

// ========== XRANGE/XREVRANGE Schemas ==========

/// Query parameters for XRANGE/XREVRANGE
#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct StreamRangeQuery {
    /// Start ID (use "-" for minimum, default)
    #[serde(default = "default_start")]
    pub start: String,

    /// End ID (use "+" for maximum, default)
    #[serde(default = "default_end")]
    pub end: String,

    /// Maximum number of entries to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
}

fn default_start() -> String {
    "-".to_string()
}

fn default_end() -> String {
    "+".to_string()
}

/// Response containing stream entries
#[derive(Debug, Serialize, ToSchema)]
pub struct StreamEntriesResponse {
    /// Stream entries
    pub entries: Vec<StreamEntry>,
}

// ========== XLEN Schemas ==========

/// Response from XLEN
#[derive(Debug, Serialize, ToSchema)]
pub struct StreamLengthResponse {
    /// Number of entries in the stream
    pub length: i64,
}

// ========== XDEL Schemas ==========

/// Request to delete entries from stream
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct StreamDeleteRequest {
    /// Entry IDs to delete
    #[validate(length(min = 1, message = "At least one ID is required"))]
    pub ids: Vec<String>,
}

/// Response from XDEL
#[derive(Debug, Serialize, ToSchema)]
pub struct StreamDeleteResponse {
    /// Number of entries deleted
    pub deleted: i64,
}

// ========== XTRIM Schemas ==========

/// Trim strategy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "strategy", rename_all = "lowercase")]
pub enum TrimStrategyParam {
    /// Keep at most N entries
    Maxlen {
        /// Maximum number of entries to keep
        count: i64,
        /// Use approximate trimming (~)
        #[serde(default = "default_true")]
        approximate: bool,
    },
    /// Remove entries older than the given ID
    Minid {
        /// Minimum ID to keep
        id: String,
        /// Use approximate trimming (~)
        #[serde(default = "default_true")]
        approximate: bool,
    },
}

impl From<TrimStrategyParam> for XTrimStrategy {
    fn from(param: TrimStrategyParam) -> Self {
        match param {
            TrimStrategyParam::Maxlen { count, approximate } => {
                XTrimStrategy::MaxLen { count, approximate }
            }
            TrimStrategyParam::Minid { id, approximate } => {
                XTrimStrategy::MinId { id, approximate }
            }
        }
    }
}

/// Request to trim stream
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StreamTrimRequest {
    /// Trim strategy
    #[serde(flatten)]
    pub strategy: TrimStrategyParam,
}

/// Response from XTRIM
#[derive(Debug, Serialize, ToSchema)]
pub struct StreamTrimResponse {
    /// Number of entries removed
    pub trimmed: i64,
}

// ========== XINFO Schemas ==========

/// Query parameters for XINFO STREAM
#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct StreamInfoQuery {
    /// Return full stream info including entries and groups
    #[serde(default)]
    pub full: bool,
}

/// Response from XINFO STREAM (re-export domain type)
pub type StreamInfoResponse = StreamInfo;

/// Response from XINFO GROUPS (re-export domain type)
pub type ConsumerGroupInfoResponse = Vec<ConsumerGroupInfo>;

/// Response from XINFO CONSUMERS (re-export domain type)
pub type ConsumerInfoResponse = Vec<ConsumerInfo>;

// ========== XREAD Schemas ==========

/// Stream and last ID pair for XREAD
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct StreamIdPair {
    /// Stream key
    #[validate(length(min = 1))]
    pub key: String,

    /// Last ID read (use "0" for all, "$" for only new entries)
    pub id: String,
}

/// Request to read from streams
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct StreamReadRequest {
    /// Streams and their last IDs
    #[validate(length(min = 1, message = "At least one stream is required"), nested)]
    pub streams: Vec<StreamIdPair>,

    /// Maximum number of entries to return per stream
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,

    /// Block for this many milliseconds (max 30000)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(max = 30000))]
    pub block_ms: Option<i64>,
}

impl From<&StreamReadRequest> for Vec<(String, String)> {
    fn from(req: &StreamReadRequest) -> Self {
        req.streams
            .iter()
            .map(|s| (s.key.clone(), s.id.clone()))
            .collect()
    }
}

impl StreamReadRequest {
    pub fn to_options(&self) -> XReadOptions {
        XReadOptions {
            count: self.count,
            block_ms: self.block_ms,
        }
    }
}

/// Request for blocking XREAD
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct StreamReadBlockingRequest {
    /// Streams and their last IDs
    #[validate(length(min = 1, message = "At least one stream is required"), nested)]
    pub streams: Vec<StreamIdPair>,

    /// Maximum number of entries to return per stream
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,

    /// Timeout in seconds (max 30)
    #[validate(range(min = 1, max = 30))]
    pub timeout_seconds: u32,
}

/// Response from XREAD (re-export domain type)
pub type StreamReadResponse = Vec<StreamReadResult>;

// ========== Consumer Group Schemas (Admin Protected) ==========

/// Request to create a consumer group
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ConsumerGroupCreateRequest {
    /// Consumer group name
    #[validate(length(min = 1, message = "Group name is required"))]
    pub group: String,

    /// Starting ID (use "0" for all entries, "$" for only new entries)
    #[serde(default = "default_group_id")]
    pub id: String,

    /// Create stream if it doesn't exist
    #[serde(default)]
    pub mkstream: bool,

    /// Number of entries initially read by the group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries_read: Option<i64>,
}

fn default_group_id() -> String {
    "$".to_string()
}

impl ConsumerGroupCreateRequest {
    pub fn to_options(&self) -> XGroupCreateOptions {
        XGroupCreateOptions {
            mkstream: self.mkstream,
            entries_read: self.entries_read,
        }
    }
}

/// Response from group creation
#[derive(Debug, Serialize, ToSchema)]
pub struct ConsumerGroupCreateResponse {
    /// Whether the group was created
    pub created: bool,
}

/// Request to set consumer group ID
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ConsumerGroupSetIdRequest {
    /// New last delivered ID
    #[validate(length(min = 1))]
    pub id: String,

    /// Number of entries read
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries_read: Option<i64>,
}

/// Request to create a consumer
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ConsumerCreateRequest {
    /// Consumer name
    #[validate(length(min = 1, message = "Consumer name is required"))]
    pub consumer: String,
}

/// Response from consumer operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ConsumerOperationResponse {
    /// Result of the operation
    pub result: i64,
}

// ========== XREADGROUP Schemas ==========

/// Request to read from streams as a consumer group member
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct StreamReadGroupRequest {
    /// Consumer name
    #[validate(length(min = 1, message = "Consumer name is required"))]
    pub consumer: String,

    /// Streams and their IDs (use ">" for never-delivered entries)
    #[validate(length(min = 1, message = "At least one stream is required"), nested)]
    pub streams: Vec<StreamIdPair>,

    /// Maximum number of entries to return per stream
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,

    /// Block for this many milliseconds (max 30000)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(max = 30000))]
    pub block_ms: Option<i64>,

    /// Don't add entries to pending list
    #[serde(default)]
    pub no_ack: bool,
}

impl StreamReadGroupRequest {
    pub fn to_options(&self) -> XReadGroupOptions {
        XReadGroupOptions {
            count: self.count,
            block_ms: self.block_ms,
            no_ack: self.no_ack,
        }
    }
}

/// Request for blocking XREADGROUP
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct StreamReadGroupBlockingRequest {
    /// Consumer name
    #[validate(length(min = 1, message = "Consumer name is required"))]
    pub consumer: String,

    /// Streams and their IDs (use ">" for never-delivered entries)
    #[validate(length(min = 1, message = "At least one stream is required"), nested)]
    pub streams: Vec<StreamIdPair>,

    /// Maximum number of entries to return per stream
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,

    /// Timeout in seconds (max 30)
    #[validate(range(min = 1, max = 30))]
    pub timeout_seconds: u32,

    /// Don't add entries to pending list
    #[serde(default)]
    pub no_ack: bool,
}

// ========== XACK Schemas ==========

/// Request to acknowledge entries
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct StreamAckRequest {
    /// Entry IDs to acknowledge
    #[validate(length(min = 1, message = "At least one ID is required"))]
    pub ids: Vec<String>,
}

/// Response from XACK
#[derive(Debug, Serialize, ToSchema)]
pub struct StreamAckResponse {
    /// Number of entries acknowledged
    pub acknowledged: i64,
}

// ========== XPENDING Schemas ==========

/// Query parameters for XPENDING detail
#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct PendingQuery {
    /// Minimum ID (use "-" for first)
    #[serde(default = "default_pending_start")]
    pub start: Option<String>,

    /// Maximum ID (use "+" for last)
    #[serde(default = "default_pending_end")]
    pub end: Option<String>,

    /// Maximum entries to return
    #[serde(default = "default_pending_count")]
    pub count: Option<i64>,

    /// Filter by consumer name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumer: Option<String>,

    /// Minimum idle time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_ms: Option<i64>,
}

fn default_pending_start() -> Option<String> {
    Some("-".to_string())
}

fn default_pending_end() -> Option<String> {
    Some("+".to_string())
}

fn default_pending_count() -> Option<i64> {
    Some(100)
}

impl From<PendingQuery> for XPendingOptions {
    fn from(query: PendingQuery) -> Self {
        XPendingOptions {
            start: query.start,
            end: query.end,
            count: query.count,
            consumer: query.consumer,
            idle_ms: query.idle_ms,
        }
    }
}

/// Response from XPENDING summary (re-export domain type)
pub type PendingSummaryResponse = PendingSummary;

/// Response from XPENDING detail (re-export domain type)
pub type PendingEntriesResponse = Vec<PendingEntry>;

// ========== XCLAIM Schemas ==========

/// Request to claim pending entries
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct StreamClaimRequest {
    /// Consumer to claim entries for
    #[validate(length(min = 1, message = "Consumer name is required"))]
    pub consumer: String,

    /// Entry IDs to claim
    #[validate(length(min = 1, message = "At least one ID is required"))]
    pub ids: Vec<String>,

    /// Minimum idle time in milliseconds
    #[serde(default)]
    pub min_idle_time_ms: i64,

    /// Set idle time to this value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_ms: Option<i64>,

    /// Set time to this value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_ms: Option<i64>,

    /// Set retry count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<i64>,

    /// Force claim even if not pending
    #[serde(default)]
    pub force: bool,

    /// Only return IDs, not full entries
    #[serde(default)]
    pub just_id: bool,

    /// Last ID for optimistic locking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_id: Option<String>,
}

impl StreamClaimRequest {
    pub fn to_options(&self) -> XClaimOptions {
        XClaimOptions {
            min_idle_time_ms: self.min_idle_time_ms,
            idle_ms: self.idle_ms,
            time_ms: self.time_ms,
            retry_count: self.retry_count,
            force: self.force,
            just_id: self.just_id,
            last_id: self.last_id.clone(),
        }
    }
}

/// Response from XCLAIM (re-export domain type)
pub type StreamClaimResponse = ClaimResult;

// ========== XAUTOCLAIM Schemas ==========

/// Request to auto-claim pending entries
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct StreamAutoClaimRequest {
    /// Consumer to claim entries for
    #[validate(length(min = 1, message = "Consumer name is required"))]
    pub consumer: String,

    /// Minimum idle time in milliseconds
    pub min_idle_time_ms: i64,

    /// Starting ID for scanning
    #[serde(default = "default_autoclaim_start")]
    pub start: String,

    /// Maximum entries to claim
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,

    /// Only return IDs, not full entries
    #[serde(default)]
    pub just_id: bool,
}

fn default_autoclaim_start() -> String {
    "0-0".to_string()
}

impl StreamAutoClaimRequest {
    pub fn to_options(&self) -> XAutoClaimOptions {
        XAutoClaimOptions {
            count: self.count,
            just_id: self.just_id,
        }
    }
}

/// Response from XAUTOCLAIM (re-export domain type)
pub type StreamAutoClaimResponse = AutoClaimResult;

// ========== SSE Streaming Schemas ==========

/// Query parameters for SSE stream subscription
#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct StreamSubscribeQuery {
    /// Last ID to start from (use "0" for all, "$" for only new)
    #[serde(default = "default_subscribe_id")]
    pub last_id: String,

    /// Maximum entries per batch
    #[serde(default = "default_subscribe_count")]
    pub count: Option<i64>,
}

fn default_subscribe_id() -> String {
    "$".to_string()
}

fn default_subscribe_count() -> Option<i64> {
    Some(10)
}

/// Query parameters for SSE consumer group subscription
#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams, Validate)]
pub struct StreamGroupSubscribeQuery {
    /// Consumer name
    #[validate(length(min = 1))]
    pub consumer: String,

    /// Maximum entries per batch
    #[serde(default = "default_subscribe_count")]
    pub count: Option<i64>,

    /// Don't add to pending list
    #[serde(default)]
    pub no_ack: bool,
}

// ========== XSETID Schemas ==========

/// Request to set stream last ID (admin operation)
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct StreamSetIdRequest {
    /// New last ID
    #[validate(length(min = 1))]
    pub last_id: String,

    /// Number of entries added
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries_added: Option<i64>,

    /// Maximum deleted entry ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_deleted_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_add_request_defaults() {
        let json = r#"{"fields":{"key":"value"}}"#;
        let req: StreamAddRequest = serde_json::from_str(json).unwrap();
        assert!(req.approximate);
        assert!(!req.no_mkstream);
    }

    #[test]
    fn test_stream_range_query_defaults() {
        let query: StreamRangeQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.start, "-");
        assert_eq!(query.end, "+");
    }

    #[test]
    fn test_trim_strategy_maxlen() {
        let json = r#"{"strategy":"maxlen","count":1000,"approximate":true}"#;
        let strategy: TrimStrategyParam = serde_json::from_str(json).unwrap();
        match strategy {
            TrimStrategyParam::Maxlen { count, approximate } => {
                assert_eq!(count, 1000);
                assert!(approximate);
            }
            _ => panic!("Wrong strategy type"),
        }
    }

    #[test]
    fn test_trim_strategy_minid() {
        let json = r#"{"strategy":"minid","id":"1704000001234-0","approximate":false}"#;
        let strategy: TrimStrategyParam = serde_json::from_str(json).unwrap();
        match strategy {
            TrimStrategyParam::Minid { id, approximate } => {
                assert_eq!(id, "1704000001234-0");
                assert!(!approximate);
            }
            _ => panic!("Wrong strategy type"),
        }
    }

    #[test]
    fn test_consumer_group_create_defaults() {
        let json = r#"{"group":"mygroup"}"#;
        let req: ConsumerGroupCreateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, "$");
        assert!(!req.mkstream);
    }

    #[test]
    fn test_stream_read_request() {
        let req = StreamReadRequest {
            streams: vec![StreamIdPair {
                key: "stream1".to_string(),
                id: "0".to_string(),
            }],
            count: Some(10),
            block_ms: Some(5000),
        };
        let pairs: Vec<(String, String)> = (&req).into();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "stream1");
        assert_eq!(pairs[0].1, "0");
    }

    #[test]
    fn test_stream_add_into_options() {
        let mut fields = HashMap::new();
        fields.insert("field".to_string(), "value".to_string());
        let req = StreamAddRequest {
            fields,
            id: Some("1-0".to_string()),
            maxlen: Some(10),
            minid: Some("0-0".to_string()),
            approximate: false,
            no_mkstream: true,
            limit: Some(5),
        };
        let opts: XAddOptions = req.into();
        assert_eq!(opts.id.as_deref(), Some("1-0"));
        assert_eq!(opts.maxlen, Some(10));
        assert_eq!(opts.minid.as_deref(), Some("0-0"));
        assert!(!opts.approximate);
        assert!(opts.no_mkstream);
        assert_eq!(opts.limit, Some(5));
    }

    #[test]
    fn test_trim_strategy_into_options() {
        let maxlen = TrimStrategyParam::Maxlen {
            count: 42,
            approximate: true,
        };
        match XTrimStrategy::from(maxlen) {
            XTrimStrategy::MaxLen { count, approximate } => {
                assert_eq!(count, 42);
                assert!(approximate);
            }
            _ => panic!("expected maxlen strategy"),
        }

        let minid = TrimStrategyParam::Minid {
            id: "1-0".to_string(),
            approximate: false,
        };
        match XTrimStrategy::from(minid) {
            XTrimStrategy::MinId { id, approximate } => {
                assert_eq!(id, "1-0");
                assert!(!approximate);
            }
            _ => panic!("expected minid strategy"),
        }
    }

    #[test]
    fn test_stream_read_request_options() {
        let req = StreamReadRequest {
            streams: vec![StreamIdPair {
                key: "stream1".to_string(),
                id: "0".to_string(),
            }],
            count: Some(2),
            block_ms: Some(1500),
        };
        let opts = req.to_options();
        assert_eq!(opts.count, Some(2));
        assert_eq!(opts.block_ms, Some(1500));
    }

    #[test]
    fn test_consumer_group_create_to_options() {
        let req = ConsumerGroupCreateRequest {
            group: "group".to_string(),
            id: "0".to_string(),
            mkstream: true,
            entries_read: Some(3),
        };
        let opts = req.to_options();
        assert!(opts.mkstream);
        assert_eq!(opts.entries_read, Some(3));
    }

    #[test]
    fn test_stream_read_group_to_options() {
        let req = StreamReadGroupRequest {
            consumer: "consumer".to_string(),
            streams: vec![StreamIdPair {
                key: "stream".to_string(),
                id: ">".to_string(),
            }],
            count: Some(4),
            block_ms: Some(2500),
            no_ack: true,
        };
        let opts = req.to_options();
        assert_eq!(opts.count, Some(4));
        assert_eq!(opts.block_ms, Some(2500));
        assert!(opts.no_ack);
    }

    #[test]
    fn test_pending_query_defaults_and_options() {
        let query: PendingQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.start.as_deref(), Some("-"));
        assert_eq!(query.end.as_deref(), Some("+"));
        assert_eq!(query.count, Some(100));

        let opts: XPendingOptions = query.into();
        assert_eq!(opts.start.as_deref(), Some("-"));
        assert_eq!(opts.end.as_deref(), Some("+"));
        assert_eq!(opts.count, Some(100));
    }

    #[test]
    fn test_stream_claim_to_options() {
        let req = StreamClaimRequest {
            consumer: "c1".to_string(),
            ids: vec!["1-0".to_string()],
            min_idle_time_ms: 50,
            idle_ms: Some(10),
            time_ms: Some(20),
            retry_count: Some(1),
            force: true,
            just_id: true,
            last_id: Some("0-0".to_string()),
        };
        let opts = req.to_options();
        assert_eq!(opts.min_idle_time_ms, 50);
        assert_eq!(opts.idle_ms, Some(10));
        assert_eq!(opts.time_ms, Some(20));
        assert_eq!(opts.retry_count, Some(1));
        assert!(opts.force);
        assert!(opts.just_id);
        assert_eq!(opts.last_id.as_deref(), Some("0-0"));
    }

    #[test]
    fn test_autoclaim_defaults_and_options() {
        let json = r#"{"consumer":"c1","min_idle_time_ms":100}"#;
        let req: StreamAutoClaimRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.start, "0-0");
        assert!(!req.just_id);

        let opts = req.to_options();
        assert!(opts.count.is_none());
        assert!(!opts.just_id);
    }

    #[test]
    fn test_stream_subscribe_defaults() {
        let query: StreamSubscribeQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.last_id, "$");
        assert_eq!(query.count, Some(10));
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn stream_read_blocking_empty_streams_fails() {
        let req = StreamReadBlockingRequest {
            streams: vec![],
            count: None,
            timeout_seconds: 5,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn stream_read_blocking_timeout_zero_fails() {
        let req = StreamReadBlockingRequest {
            streams: vec![StreamIdPair {
                key: "s1".into(),
                id: "0".into(),
            }],
            count: None,
            timeout_seconds: 0,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn stream_read_blocking_timeout_31_fails() {
        let req = StreamReadBlockingRequest {
            streams: vec![StreamIdPair {
                key: "s1".into(),
                id: "0".into(),
            }],
            count: None,
            timeout_seconds: 31,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn stream_read_blocking_valid_passes() {
        let req = StreamReadBlockingRequest {
            streams: vec![StreamIdPair {
                key: "s1".into(),
                id: "0".into(),
            }],
            count: None,
            timeout_seconds: 5,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn stream_read_group_blocking_empty_streams_fails() {
        let req = StreamReadGroupBlockingRequest {
            consumer: "c1".into(),
            streams: vec![],
            count: None,
            timeout_seconds: 5,
            no_ack: false,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn stream_read_group_blocking_timeout_zero_fails() {
        let req = StreamReadGroupBlockingRequest {
            consumer: "c1".into(),
            streams: vec![StreamIdPair {
                key: "s1".into(),
                id: ">".into(),
            }],
            count: None,
            timeout_seconds: 0,
            no_ack: false,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn stream_read_group_blocking_timeout_31_fails() {
        let req = StreamReadGroupBlockingRequest {
            consumer: "c1".into(),
            streams: vec![StreamIdPair {
                key: "s1".into(),
                id: ">".into(),
            }],
            count: None,
            timeout_seconds: 31,
            no_ack: false,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn stream_read_group_blocking_valid_passes() {
        let req = StreamReadGroupBlockingRequest {
            consumer: "c1".into(),
            streams: vec![StreamIdPair {
                key: "s1".into(),
                id: ">".into(),
            }],
            count: None,
            timeout_seconds: 5,
            no_ack: false,
        };
        assert!(req.validate().is_ok());
    }
}
