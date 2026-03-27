//! String Schemas
//!
//! Request/response schemas for string operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use validator::Validate;

/// Request to set a string value
#[allow(clippy::struct_excessive_bools)] // Mirrors Redis SET command options
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SetStringRequest {
    /// The value to set
    #[validate(length(min = 0))]
    pub value: String,

    /// TTL in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,

    /// TTL in milliseconds (takes precedence over `ttl_seconds`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,

    /// Only set if key does not exist (NX)
    #[serde(default)]
    pub nx: bool,

    /// Only set if key exists (XX)
    #[serde(default)]
    pub xx: bool,

    /// Return the previous value (GET)
    #[serde(default)]
    pub get: bool,

    /// Keep the existing TTL (KEEPTTL)
    #[serde(default)]
    pub keep_ttl: bool,
}

/// Response after setting a string
#[derive(Debug, Serialize, ToSchema)]
pub struct SetStringResponse {
    /// The key that was set
    pub key: String,

    /// Whether the operation was successful
    pub success: bool,

    /// Previous value if GET option was used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_value: Option<String>,
}

/// Request for MGET operation
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MGetRequest {
    /// Keys to retrieve
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
}

/// Response for MGET operation
#[derive(Debug, Serialize, ToSchema)]
pub struct MGetResponse {
    /// Keys that were found with their values
    pub found: HashMap<String, String>,

    /// Keys that were not found
    pub missing: Vec<String>,

    /// Total count of keys requested
    pub total_requested: usize,

    /// Count of keys found
    pub found_count: usize,
}

/// Request for MSET operation
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MSetRequest {
    /// Key-value pairs to set
    #[validate(length(min = 1, message = "At least one pair is required"))]
    pub pairs: HashMap<String, String>,

    /// Only set if none of the keys exist (NX mode)
    #[serde(default)]
    pub nx: bool,
}

/// Response for MSET operation
#[derive(Debug, Serialize, ToSchema)]
pub struct MSetResponse {
    /// Number of keys set
    pub count: usize,

    /// Keys that were set
    pub keys: Vec<String>,

    /// Whether operation succeeded (for MSETNX)
    pub success: bool,
}

/// Request to increment a value
#[derive(Debug, Deserialize, ToSchema)]
pub struct IncrementRequest {
    /// Amount to increment by (default: 1)
    #[serde(default = "default_increment")]
    pub delta: i64,

    /// Whether to use float increment
    #[serde(default)]
    pub float: bool,

    /// Float delta (used when float is true)
    pub float_delta: Option<f64>,
}

const fn default_increment() -> i64 {
    1
}

/// Response for increment operation
#[derive(Debug, Serialize, ToSchema)]
pub struct IncrementResponse {
    /// The key that was incremented
    pub key: String,

    /// New value after increment
    pub new_value: String,
}

/// Request to append to a string
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AppendRequest {
    /// Value to append
    #[validate(length(min = 1))]
    pub value: String,
}

/// Response for append operation
#[derive(Debug, Serialize, ToSchema)]
pub struct AppendResponse {
    /// The key that was appended to
    pub key: String,

    /// New length of the string
    pub new_length: i64,
}

/// Query parameters for GETRANGE
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetRangeParams {
    /// Start index (can be negative)
    pub start: i64,

    /// End index (can be negative)
    pub end: i64,
}

/// Request for LCS (Longest Common Subsequence) operation
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct LcsRequest {
    /// First key
    #[validate(length(min = 1, message = "key1 is required"))]
    pub key1: String,
    /// Second key
    #[validate(length(min = 1, message = "key2 is required"))]
    pub key2: String,
    /// Return just the length
    #[serde(default)]
    pub len: bool,
    /// Return match positions
    #[serde(default)]
    pub idx: bool,
    /// Minimum match length (used with IDX)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_match_len: Option<u64>,
    /// Include match lengths in IDX output
    #[serde(default)]
    pub with_match_len: bool,
}

/// Response for LCS operation
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(untagged)]
pub enum LcsResponse {
    /// The LCS string itself
    String {
        /// The longest common subsequence
        lcs: String,
    },
    /// Just the length
    Length {
        /// Length of the LCS
        length: i64,
    },
    /// Match positions
    Matches {
        /// List of matches
        matches: Vec<LcsMatchSchema>,
        /// Total LCS length
        len: i64,
    },
}

/// A single match in the LCS result
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LcsMatchSchema {
    /// Start position in key1
    pub key1_start: i64,
    /// End position in key1
    pub key1_end: i64,
    /// Start position in key2
    pub key2_start: i64,
    /// End position in key2
    pub key2_end: i64,
    /// Length of this match (only if with_match_len is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_len: Option<i64>,
}

impl From<crate::domain::repositories::LcsResult> for LcsResponse {
    fn from(result: crate::domain::repositories::LcsResult) -> Self {
        match result {
            crate::domain::repositories::LcsResult::String(s) => LcsResponse::String { lcs: s },
            crate::domain::repositories::LcsResult::Length(n) => LcsResponse::Length { length: n },
            crate::domain::repositories::LcsResult::Matches(m) => LcsResponse::Matches {
                matches: m
                    .matches
                    .into_iter()
                    .map(|lm| LcsMatchSchema {
                        key1_start: lm.key1_range.0,
                        key1_end: lm.key1_range.1,
                        key2_start: lm.key2_range.0,
                        key2_end: lm.key2_range.1,
                        match_len: lm.match_len,
                    })
                    .collect(),
                len: m.len,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::{LcsMatch, LcsMatchResult, LcsResult};
    use validator::Validate;

    #[test]
    fn test_default_increment() {
        assert_eq!(default_increment(), 1);
    }

    #[test]
    fn test_lcs_request_empty_key1() {
        let req = LcsRequest {
            key1: "".to_string(),
            key2: "b".to_string(),
            len: false,
            idx: false,
            min_match_len: None,
            with_match_len: false,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_lcs_request_empty_key2() {
        let req = LcsRequest {
            key1: "a".to_string(),
            key2: "".to_string(),
            len: false,
            idx: false,
            min_match_len: None,
            with_match_len: false,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_lcs_request_valid() {
        let req = LcsRequest {
            key1: "a".to_string(),
            key2: "b".to_string(),
            len: false,
            idx: false,
            min_match_len: None,
            with_match_len: false,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_lcs_request_with_idx_options() {
        let req = LcsRequest {
            key1: "k1".to_string(),
            key2: "k2".to_string(),
            len: false,
            idx: true,
            min_match_len: Some(3),
            with_match_len: true,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_lcs_response_from_string_result() {
        let result = LcsResult::String("hello".to_string());
        let response: LcsResponse = result.into();
        match response {
            LcsResponse::String { lcs } => assert_eq!(lcs, "hello"),
            _ => panic!("Expected String variant"),
        }
    }

    #[test]
    fn test_lcs_response_from_length_result() {
        let result = LcsResult::Length(42);
        let response: LcsResponse = result.into();
        match response {
            LcsResponse::Length { length } => assert_eq!(length, 42),
            _ => panic!("Expected Length variant"),
        }
    }

    #[test]
    fn test_lcs_response_from_matches_result() {
        let result = LcsResult::Matches(LcsMatchResult {
            matches: vec![LcsMatch {
                key1_range: (1, 3),
                key2_range: (2, 4),
                match_len: Some(3),
            }],
            len: 3,
        });
        let response: LcsResponse = result.into();
        match response {
            LcsResponse::Matches { matches, len } => {
                assert_eq!(len, 3);
                assert_eq!(matches.len(), 1);
                assert_eq!(matches[0].key1_start, 1);
                assert_eq!(matches[0].key1_end, 3);
                assert_eq!(matches[0].key2_start, 2);
                assert_eq!(matches[0].key2_end, 4);
                assert_eq!(matches[0].match_len, Some(3));
            }
            _ => panic!("Expected Matches variant"),
        }
    }
}

/// Response for GETRANGE operation
#[derive(Debug, Serialize, ToSchema)]
pub struct GetRangeResponse {
    /// The key
    pub key: String,

    /// The substring value
    pub value: String,

    /// Start index used
    pub start: i64,

    /// End index used
    pub end: i64,
}

/// Request for SETRANGE operation
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SetRangeRequest {
    /// Offset to start writing at
    #[validate(range(min = 0))]
    pub offset: i64,

    /// Value to write
    pub value: String,
}

/// Response for SETRANGE operation
#[derive(Debug, Serialize, ToSchema)]
pub struct SetRangeResponse {
    /// The key
    pub key: String,

    /// New length of the string
    pub new_length: i64,
}

/// Query parameters for GETEX
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetExParams {
    /// New TTL in seconds
    pub ttl_seconds: Option<u64>,

    /// New TTL in milliseconds
    pub ttl_ms: Option<u64>,

    /// Remove the TTL (PERSIST)
    #[serde(default)]
    pub persist: bool,
}

/// Response for GETDEL operation
#[derive(Debug, Serialize, ToSchema)]
pub struct GetDelResponse {
    /// The key that was deleted
    pub key: String,

    /// The value that was stored (None if key didn't exist)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Whether the key existed
    pub existed: bool,
}

/// Response for STRLEN operation
#[derive(Debug, Serialize, ToSchema)]
pub struct StrLenResponse {
    /// The key
    pub key: String,

    /// Length of the string
    pub length: i64,
}
