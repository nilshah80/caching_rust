//! Sorted Set Schemas
//!
//! Request and response types for sorted set (ZSET) API endpoints.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// A member with its score
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScoredMemberDto {
    /// The member value
    pub member: String,
    /// The score associated with the member
    pub score: f64,
}

impl ScoredMemberDto {
    pub fn new(member: String, score: f64) -> Self {
        Self { member, score }
    }
}

/// Options for ZADD command
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ZAddOptionsDto {
    /// NX: Only add new elements (don't update existing)
    #[serde(default)]
    pub nx: bool,
    /// XX: Only update existing elements (don't add new)
    #[serde(default)]
    pub xx: bool,
    /// GT: Only update when new score > current score
    #[serde(default)]
    pub gt: bool,
    /// LT: Only update when new score < current score
    #[serde(default)]
    pub lt: bool,
    /// CH: Return number of changed elements (added + updated) instead of just added
    #[serde(default)]
    pub ch: bool,
}

/// Request to add members with scores to a sorted set
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZAddRequest {
    /// Members with their scores to add
    #[validate(length(min = 1, message = "At least one member is required"))]
    pub members: Vec<ScoredMemberDto>,
    /// Options for the ZADD operation
    #[serde(default)]
    pub options: Option<ZAddOptionsDto>,
}

/// Request for ZADD with INCR option
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZAddIncrRequest {
    /// Member to increment
    pub member: String,
    /// Score to add (can be negative)
    pub score: f64,
    /// Options for the operation
    #[serde(default)]
    pub options: Option<ZAddOptionsDto>,
}

/// Request to remove members from a sorted set
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZRemRequest {
    /// Members to remove
    #[validate(length(min = 1, message = "At least one member is required"))]
    pub members: Vec<String>,
}

/// Request to get scores of multiple members
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZMScoreRequest {
    /// Members to get scores for
    #[validate(length(min = 1, message = "At least one member is required"))]
    pub members: Vec<String>,
}

/// Request for ZINCRBY
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZIncrByRequest {
    /// Member to increment
    pub member: String,
    /// Amount to increment by (can be negative)
    pub increment: f64,
}

/// Score range for queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScoreRangeDto {
    /// Minimum score (use "-inf" for negative infinity)
    pub min: String,
    /// Maximum score (use "+inf" for positive infinity)
    pub max: String,
}

impl ScoreRangeDto {
    pub fn all() -> Self {
        Self {
            min: "-inf".to_string(),
            max: "+inf".to_string(),
        }
    }
}

/// Lexicographical range for queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LexRangeDto {
    /// Minimum value ("-" for unbounded, "[value" for inclusive, "(value" for exclusive)
    pub min: String,
    /// Maximum value ("+" for unbounded, "[value" for inclusive, "(value" for exclusive)
    pub max: String,
}

/// Query parameters for ZRANGE
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZRangeQuery {
    /// Start index (0-based, negative values count from end)
    pub start: i64,
    /// Stop index (inclusive, negative values count from end)
    pub stop: i64,
    /// Include scores in response
    #[serde(default)]
    pub with_scores: bool,
    /// Reverse order
    #[serde(default)]
    pub rev: bool,
}

/// Request for ZRANGEBYSCORE
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZRangeByScoreRequest {
    /// Score range
    pub range: ScoreRangeDto,
    /// Include scores in response
    #[serde(default)]
    pub with_scores: bool,
    /// Reverse order
    #[serde(default)]
    pub rev: bool,
    /// Offset for pagination
    pub offset: Option<i64>,
    /// Count for pagination
    pub count: Option<i64>,
}

/// Request for ZRANGEBYLEX
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZRangeByLexRequest {
    /// Lexicographical range
    pub range: LexRangeDto,
    /// Reverse order
    #[serde(default)]
    pub rev: bool,
    /// Offset for pagination
    pub offset: Option<i64>,
    /// Count for pagination
    pub count: Option<i64>,
}

/// Request for ZRANGESTORE
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZRangeStoreRequest {
    /// Destination key
    #[validate(length(min = 1))]
    pub destination: String,
    /// Start index
    pub start: i64,
    /// Stop index
    pub stop: i64,
    /// Include scores in result
    #[serde(default)]
    pub with_scores: bool,
    /// Reverse order
    #[serde(default)]
    pub rev: bool,
}

/// Request for ZCOUNT
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZCountRequest {
    /// Score range
    pub range: ScoreRangeDto,
}

/// Request for ZLEXCOUNT
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZLexCountRequest {
    /// Lexicographical range
    pub range: LexRangeDto,
}

/// Request for ZREMRANGEBYRANK
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZRemRangeByRankRequest {
    /// Start rank
    pub start: i64,
    /// Stop rank
    pub stop: i64,
}

/// Request for ZREMRANGEBYSCORE
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZRemRangeByScoreRequest {
    /// Score range
    pub range: ScoreRangeDto,
}

/// Request for ZREMRANGEBYLEX
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZRemRangeByLexRequest {
    /// Lexicographical range
    pub range: LexRangeDto,
}

/// Query parameters for ZPOPMIN/ZPOPMAX
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZPopQuery {
    /// Number of elements to pop (default: 1)
    #[serde(default)]
    pub count: Option<i64>,
}

/// Request for blocking pop operations (BZPOPMIN, BZPOPMAX)
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZBPopRequest {
    /// Keys to pop from
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
    /// Timeout in seconds (server-enforced max from configuration)
    #[validate(range(min = 1, message = "Timeout must be at least 1 second"))]
    pub timeout_seconds: u32,
}

/// Request for ZMPOP
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZMPopRequest {
    /// Keys to pop from
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
    /// Direction: "min" or "max"
    pub direction: String,
    /// Number of elements to pop
    pub count: Option<i64>,
}

/// Request for BZMPOP
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZBMPopRequest {
    /// Keys to pop from
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
    /// Direction: "min" or "max"
    pub direction: String,
    /// Timeout in seconds (server-enforced max from configuration)
    #[validate(range(min = 1, message = "Timeout must be at least 1 second"))]
    pub timeout_seconds: u32,
    /// Number of elements to pop
    pub count: Option<i64>,
}

/// Query parameters for ZRANDMEMBER
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZRandMemberQuery {
    /// Number of members to return (positive = distinct, negative = may repeat)
    pub count: Option<i64>,
    /// Include scores in response
    #[serde(default)]
    pub with_scores: bool,
}

/// Aggregate function for set operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ZAggregateDto {
    #[default]
    Sum,
    Min,
    Max,
}

/// Options for set algebra operations (ZUNION, ZINTER)
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ZSetAlgebraOptionsDto {
    /// Weights for each key
    pub weights: Option<Vec<f64>>,
    /// Aggregate function
    #[serde(default)]
    pub aggregate: ZAggregateDto,
    /// Include scores in result
    #[serde(default)]
    pub with_scores: bool,
}

/// Request for ZUNION/ZINTER
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZSetAlgebraRequest {
    /// Keys to operate on
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
    /// Options for the operation
    #[serde(default)]
    pub options: Option<ZSetAlgebraOptionsDto>,
}

/// Request for ZUNIONSTORE/ZINTERSTORE
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZSetAlgebraStoreRequest {
    /// Destination key
    #[validate(length(min = 1))]
    pub destination: String,
    /// Keys to operate on
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
    /// Options for the operation
    #[serde(default)]
    pub options: Option<ZSetAlgebraOptionsDto>,
}

/// Request for ZINTERCARD
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZInterCardRequest {
    /// Keys to operate on
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
    /// Optional limit to stop early
    pub limit: Option<u64>,
}

/// Request for ZDIFF
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZDiffRequest {
    /// Keys to operate on
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
    /// Include scores in result
    #[serde(default)]
    pub with_scores: bool,
}

/// Request for ZDIFFSTORE
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ZDiffStoreRequest {
    /// Destination key
    #[validate(length(min = 1))]
    pub destination: String,
    /// Keys to operate on
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
}

/// Query parameters for ZSCAN
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZScanQuery {
    /// Cursor position (0 to start)
    #[serde(default)]
    pub cursor: u64,
    /// Pattern to match members
    pub pattern: Option<String>,
    /// Hint for number of members to return per call
    pub count: Option<u64>,
}

// ========== Response types ==========

/// Response from ZADD
#[derive(Debug, Serialize, ToSchema)]
pub struct ZAddResponse {
    /// Number of elements added (or changed if CH option was used)
    pub count: i64,
    /// New score if INCR option was used
    pub new_score: Option<f64>,
}

/// Response from ZADD with INCR
#[derive(Debug, Serialize, ToSchema)]
pub struct ZAddIncrResponse {
    /// The new score of the member
    pub new_score: Option<f64>,
}

/// Response from ZREM
#[derive(Debug, Serialize, ToSchema)]
pub struct ZRemResponse {
    /// Number of members removed
    pub removed: i64,
}

/// Response from ZSCORE
#[derive(Debug, Serialize, ToSchema)]
pub struct ZScoreResponse {
    /// The score of the member (null if member doesn't exist)
    pub score: Option<f64>,
}

/// Response from ZMSCORE
#[derive(Debug, Serialize, ToSchema)]
pub struct ZMScoreResponse {
    /// Scores of the members (null for non-existent members)
    pub scores: Vec<Option<f64>>,
}

/// Response from ZINCRBY
#[derive(Debug, Serialize, ToSchema)]
pub struct ZIncrByResponse {
    /// The new score after increment
    pub new_score: f64,
}

/// Response from ZCARD
#[derive(Debug, Serialize, ToSchema)]
pub struct ZCardResponse {
    /// Number of members in the sorted set
    pub cardinality: i64,
}

/// Response from ZCOUNT/ZLEXCOUNT
#[derive(Debug, Serialize, ToSchema)]
pub struct ZCountResponse {
    /// Number of members in the specified range
    pub count: i64,
}

/// Response from ZRANK/ZREVRANK
#[derive(Debug, Serialize, ToSchema)]
pub struct ZRankResponse {
    /// The rank of the member (null if member doesn't exist)
    pub rank: Option<i64>,
}

/// Response from range operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ZRangeResponse {
    /// Members with their scores
    pub members: Vec<ScoredMemberDto>,
}

/// Response from ZRANGEBYLEX
#[derive(Debug, Serialize, ToSchema)]
pub struct ZRangeByLexResponse {
    /// Members in the lexicographical range
    pub members: Vec<String>,
}

/// Response from ZRANGESTORE
#[derive(Debug, Serialize, ToSchema)]
pub struct ZRangeStoreResponse {
    /// Number of members stored
    pub count: i64,
}

/// Response from range remove operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ZRemRangeResponse {
    /// Number of members removed
    pub removed: i64,
}

/// Response from pop operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ZPopResponse {
    /// Popped members with their scores
    pub members: Vec<ScoredMemberDto>,
}

/// Response from blocking pop operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ZBPopResponse {
    /// The key from which the element was popped (null if timed out)
    pub key: Option<String>,
    /// Popped members with their scores
    pub members: Vec<ScoredMemberDto>,
}

/// Response from ZMPOP/BZMPOP
#[derive(Debug, Serialize, ToSchema)]
pub struct ZMPopResponse {
    /// The key from which elements were popped (null if none found or timed out)
    pub key: Option<String>,
    /// Popped members with their scores
    pub members: Vec<ScoredMemberDto>,
}

/// Response from ZRANDMEMBER
#[derive(Debug, Serialize, ToSchema)]
pub struct ZRandMemberResponse {
    /// Random members with their scores
    pub members: Vec<ScoredMemberDto>,
}

/// Response from set algebra operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ZSetAlgebraResponse {
    /// Resulting members with their scores
    pub members: Vec<ScoredMemberDto>,
}

/// Response from set algebra store operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ZSetAlgebraStoreResponse {
    /// Number of members in the resulting set
    pub count: i64,
}

/// Response from ZINTERCARD
#[derive(Debug, Serialize, ToSchema)]
pub struct ZInterCardResponse {
    /// Cardinality of the intersection
    pub cardinality: i64,
}

/// Response from ZSCAN
#[derive(Debug, Serialize, ToSchema)]
pub struct ZScanResponse {
    /// Cursor for next iteration (0 = complete)
    pub cursor: u64,
    /// Members with their scores returned in this batch
    pub members: Vec<ScoredMemberDto>,
}

/// Query parameters for BZPOPMIN/BZPOPMAX SSE streaming endpoints
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ZBPopStreamQuery {
    /// Number of seconds between polls (default: 5, max: 30)
    #[serde(default = "default_zpop_poll_seconds")]
    pub poll_seconds: Option<u32>,
}

fn default_zpop_poll_seconds() -> Option<u32> {
    Some(5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_zpop_poll_seconds() {
        assert_eq!(default_zpop_poll_seconds(), Some(5));
    }

    #[test]
    fn test_zadd_request() {
        let json = r#"{"members": [{"member": "a", "score": 1.0}, {"member": "b", "score": 2.0}]}"#;
        let req: ZAddRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.members.len(), 2);
        assert_eq!(req.members[0].member, "a");
        assert_eq!(req.members[0].score, 1.0);
    }

    #[test]
    fn test_zadd_request_with_options() {
        let json =
            r#"{"members": [{"member": "a", "score": 1.0}], "options": {"nx": true, "ch": true}}"#;
        let req: ZAddRequest = serde_json::from_str(json).unwrap();
        let opts = req.options.unwrap();
        assert!(opts.nx);
        assert!(opts.ch);
        assert!(!opts.xx);
    }

    #[test]
    fn test_score_range() {
        let json = r#"{"min": "-inf", "max": "+inf"}"#;
        let range: ScoreRangeDto = serde_json::from_str(json).unwrap();
        assert_eq!(range.min, "-inf");
        assert_eq!(range.max, "+inf");
    }

    #[test]
    fn test_lex_range() {
        let json = r#"{"min": "[a", "max": "[z"}"#;
        let range: LexRangeDto = serde_json::from_str(json).unwrap();
        assert_eq!(range.min, "[a");
        assert_eq!(range.max, "[z");
    }

    #[test]
    fn test_zset_algebra_request() {
        let json = r#"{"keys": ["zset1", "zset2"], "options": {"weights": [1.0, 2.0], "aggregate": "max"}}"#;
        let req: ZSetAlgebraRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.keys.len(), 2);
        let opts = req.options.unwrap();
        assert_eq!(opts.weights.unwrap(), vec![1.0, 2.0]);
    }

    #[test]
    fn test_zscan_query_defaults() {
        let query: ZScanQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.cursor, 0);
        assert!(query.pattern.is_none());
        assert!(query.count.is_none());
    }

    #[test]
    fn test_zmpop_request() {
        let json = r#"{"keys": ["zset1", "zset2"], "direction": "min", "count": 5}"#;
        let req: ZMPopRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.keys.len(), 2);
        assert_eq!(req.direction, "min");
        assert_eq!(req.count, Some(5));
    }

    #[test]
    fn test_scored_member_dto_new() {
        let member = ScoredMemberDto::new("value".to_string(), 1.5);
        assert_eq!(member.member, "value");
        assert_eq!(member.score, 1.5);
    }

    #[test]
    fn test_score_range_all() {
        let range = ScoreRangeDto::all();
        assert_eq!(range.min, "-inf");
        assert_eq!(range.max, "+inf");
    }

    #[test]
    fn test_aggregate_default() {
        let agg = ZAggregateDto::default();
        assert!(matches!(agg, ZAggregateDto::Sum));
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn zbpop_empty_keys_fails() {
        let req = ZBPopRequest {
            keys: vec![],
            timeout_seconds: 5,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn zbpop_timeout_zero_fails() {
        let req = ZBPopRequest {
            keys: vec!["k1".into()],
            timeout_seconds: 0,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn zbpop_timeout_31_passes_server_clamps_later() {
        let req = ZBPopRequest {
            keys: vec!["k1".into()],
            timeout_seconds: 31,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn zbpop_valid_passes() {
        let req = ZBPopRequest {
            keys: vec!["k1".into()],
            timeout_seconds: 5,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn zbmpop_empty_keys_fails() {
        let req = ZBMPopRequest {
            keys: vec![],
            direction: "min".into(),
            timeout_seconds: 5,
            count: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn zbmpop_timeout_zero_fails() {
        let req = ZBMPopRequest {
            keys: vec!["k1".into()],
            direction: "min".into(),
            timeout_seconds: 0,
            count: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn zbmpop_timeout_31_passes_server_clamps_later() {
        let req = ZBMPopRequest {
            keys: vec!["k1".into()],
            direction: "min".into(),
            timeout_seconds: 31,
            count: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn zbmpop_valid_passes() {
        let req = ZBMPopRequest {
            keys: vec!["k1".into()],
            direction: "min".into(),
            timeout_seconds: 5,
            count: None,
        };
        assert!(req.validate().is_ok());
    }
}
