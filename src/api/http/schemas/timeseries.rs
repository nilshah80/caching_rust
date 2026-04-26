//! RedisTimeSeries schemas.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use validator::Validate;

use crate::domain::repositories::TsIgnore;

/// A single time-series sample.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct Sample {
    #[validate(range(min = 0, message = "Timestamp must be non-negative"))]
    pub timestamp: i64,
    pub value: f64,
}

/// Duplicate policy for RedisTimeSeries.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DuplicatePolicy {
    Block,
    First,
    Last,
    Min,
    Max,
    Sum,
}

/// Aggregation type.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Aggregation {
    Avg,
    Sum,
    Min,
    Max,
    Range,
    Count,
    First,
    Last,
    #[serde(rename = "std.p")]
    StdP,
    #[serde(rename = "std.s")]
    StdS,
    #[serde(rename = "var.p")]
    VarP,
    #[serde(rename = "var.s")]
    VarS,
    Twa,
}

/// IGNORE thresholds for TS.CREATE / TS.ALTER / TS.ADD (RedisTimeSeries 1.12+).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, Validate)]
pub struct TsIgnoreSchema {
    /// Maximum allowed timestamp delta from the current max for a sample to
    /// be considered a duplicate. Must be non-negative.
    #[validate(range(min = 0, message = "max_time_diff must be non-negative"))]
    pub max_time_diff: i64,
    /// Maximum allowed absolute value delta from the previous max-timestamp
    /// sample. Must be non-negative.
    #[validate(range(min = 0.0, message = "max_val_diff must be non-negative"))]
    pub max_val_diff: f64,
}

impl From<TsIgnoreSchema> for TsIgnore {
    fn from(s: TsIgnoreSchema) -> Self {
        TsIgnore {
            max_time_diff: s.max_time_diff,
            max_val_diff: s.max_val_diff,
        }
    }
}

/// Create time-series key request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TimeSeriesCreateRequest {
    #[validate(length(min = 1, message = "Key is required"))]
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_policy: Option<DuplicatePolicy>,
    /// IGNORE thresholds (10.8). Both fields are required when present;
    /// the validator runs nested.
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<TsIgnoreSchema>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

/// Add single sample request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TimeSeriesAddRequest {
    #[validate(range(min = 0, message = "Timestamp must be non-negative"))]
    pub timestamp: i64,
    pub value: f64,
    /// Override the configured DUPLICATE_POLICY for this single sample
    /// (TS.ADD `ON_DUPLICATE`, 10.8). Applies on every TS.ADD call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_duplicate: Option<DuplicatePolicy>,
    /// IGNORE thresholds for TS.ADD (10.8). Per Redis docs these are applied
    /// only when TS.ADD creates the series; on an existing series the field
    /// is silently ignored. Use TS.ALTER with `ignore` set to update an
    /// existing series.
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<TsIgnoreSchema>,
}

/// Range query parameters.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TimeSeriesRangeQuery {
    #[validate(range(min = 0, message = "from must be non-negative"))]
    pub from: i64,
    #[validate(range(min = 0, message = "to must be non-negative"))]
    pub to: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregation: Option<Aggregation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_duration_ms: Option<u64>,
}

/// MGET request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TimeSeriesMGetRequest {
    #[validate(length(min = 1, message = "At least one filter is required"))]
    pub filters: Vec<String>,
}

/// MRANGE request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TimeSeriesMRangeRequest {
    #[validate(range(min = 0, message = "from must be non-negative"))]
    pub from: i64,
    #[validate(range(min = 0, message = "to must be non-negative"))]
    pub to: i64,
    #[validate(length(min = 1, message = "At least one filter is required"))]
    pub filters: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregation: Option<Aggregation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_duration_ms: Option<u64>,
}

/// Response for write operations returning a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeSeriesWriteResponse {
    pub timestamp: i64,
}

/// Response for latest sample fetch.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeSeriesGetResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample: Option<Sample>,
}

/// Range response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeSeriesRangeResponse {
    pub samples: Vec<Sample>,
}

/// Latest multi-series result item.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeSeriesMGetItem {
    pub key: String,
    pub labels: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample: Option<Sample>,
}

/// Response for MGET.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeSeriesMGetResponse {
    pub series: Vec<TimeSeriesMGetItem>,
}

/// Multi-series range result item.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeSeriesRangeItem {
    pub key: String,
    pub labels: HashMap<String, String>,
    pub samples: Vec<Sample>,
}

/// Response for MRANGE.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeSeriesMRangeResponse {
    pub series: Vec<TimeSeriesRangeItem>,
}

/// Alter time-series key request (TS.ALTER).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TsAlterRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_policy: Option<DuplicatePolicy>,
    /// IGNORE thresholds (10.8). Validated nested.
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<TsIgnoreSchema>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

/// Multi-add item for TS.MADD.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TsMaddItem {
    #[validate(length(min = 1, message = "Key is required"))]
    pub key: String,
    #[validate(range(min = 0, message = "Timestamp must be non-negative"))]
    pub timestamp: i64,
    pub value: f64,
}

/// Multi-add request (TS.MADD).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TsMaddRequest {
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<TsMaddItem>,
}

/// Multi-add response (TS.MADD).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TsMaddResponse {
    pub timestamps: Vec<i64>,
}

/// Increment/decrement request (TS.INCRBY / TS.DECRBY).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TsIncrDecrRequest {
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

/// Delete samples query parameters (TS.DEL).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TsDelQuery {
    #[validate(range(min = 0, message = "from must be non-negative"))]
    pub from: i64,
    #[validate(range(min = 0, message = "to must be non-negative"))]
    pub to: i64,
}

/// Delete samples response (TS.DEL).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TsDelResponse {
    pub deleted: i64,
}

/// MREVRANGE request (TS.MREVRANGE).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TsMrevRangeRequest {
    #[validate(range(min = 0, message = "from must be non-negative"))]
    pub from: i64,
    #[validate(range(min = 0, message = "to must be non-negative"))]
    pub to: i64,
    #[validate(length(min = 1, message = "At least one filter is required"))]
    pub filters: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregation: Option<Aggregation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_duration_ms: Option<u64>,
}

/// Query index request (TS.QUERYINDEX).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TsQueryIndexRequest {
    #[validate(length(min = 1, message = "At least one filter is required"))]
    pub filters: Vec<String>,
}

/// Query index response (TS.QUERYINDEX).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TsQueryIndexResponse {
    pub keys: Vec<String>,
}

/// Info response (TS.INFO).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TsInfoResponse {
    pub info: serde_json::Value,
}

/// Create compaction rule request (TS.CREATERULE).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TsCreateRuleRequest {
    #[validate(length(min = 1, message = "dest_key is required"))]
    pub dest_key: String,
    pub aggregation: Aggregation,
    #[validate(range(min = 1, message = "bucket_duration_ms must be positive"))]
    pub bucket_duration_ms: u64,
    /// Optional `alignTimestamp` (RedisTimeSeries 1.8+, expressed in ms).
    /// When set, every bucket starts at exactly `alignTimestamp + n * bucket_duration_ms`.
    #[validate(range(min = 0, message = "align_timestamp_ms must be non-negative"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_timestamp_ms: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_request_validation() {
        let request = TimeSeriesCreateRequest {
            key: "".to_string(),
            retention_ms: None,
            chunk_size: None,
            duplicate_policy: None,
            labels: HashMap::new(),
            ignore: None,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_sample_validation() {
        let sample = Sample {
            timestamp: -1,
            value: 1.0,
        };
        assert!(sample.validate().is_err());
    }

    // ─── 10.8 IGNORE / ON_DUPLICATE / alignTimestamp validation ──────────────

    #[test]
    fn test_ignore_schema_rejects_negative_time_diff() {
        let ig = TsIgnoreSchema {
            max_time_diff: -1,
            max_val_diff: 0.0,
        };
        assert!(ig.validate().is_err());
    }

    #[test]
    fn test_ignore_schema_rejects_negative_val_diff() {
        let ig = TsIgnoreSchema {
            max_time_diff: 0,
            max_val_diff: -0.5,
        };
        assert!(ig.validate().is_err());
    }

    #[test]
    fn test_create_request_propagates_nested_ignore_validation() {
        // Nested validation must trigger when the parent is validated.
        let request = TimeSeriesCreateRequest {
            key: "k".to_string(),
            retention_ms: None,
            chunk_size: None,
            duplicate_policy: None,
            ignore: Some(TsIgnoreSchema {
                max_time_diff: -10,
                max_val_diff: 0.0,
            }),
            labels: HashMap::new(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_alter_request_propagates_nested_ignore_validation() {
        let request = TsAlterRequest {
            retention_ms: None,
            chunk_size: None,
            duplicate_policy: None,
            ignore: Some(TsIgnoreSchema {
                max_time_diff: 0,
                max_val_diff: -2.0,
            }),
            labels: HashMap::new(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_add_request_propagates_nested_ignore_validation() {
        let request = TimeSeriesAddRequest {
            timestamp: 0,
            value: 1.0,
            on_duplicate: Some(DuplicatePolicy::Last),
            ignore: Some(TsIgnoreSchema {
                max_time_diff: -1,
                max_val_diff: 0.0,
            }),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_create_rule_request_rejects_negative_align() {
        let request = TsCreateRuleRequest {
            dest_key: "dst".to_string(),
            aggregation: Aggregation::Avg,
            bucket_duration_ms: 60_000,
            align_timestamp_ms: Some(-1),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_create_rule_request_accepts_zero_align() {
        // 0 means "aligned with the epoch" per Redis docs — explicitly allowed.
        let request = TsCreateRuleRequest {
            dest_key: "dst".to_string(),
            aggregation: Aggregation::Avg,
            bucket_duration_ms: 60_000,
            align_timestamp_ms: Some(0),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_mrange_validation() {
        let request = TimeSeriesMRangeRequest {
            from: 0,
            to: 10,
            filters: vec![],
            count: None,
            aggregation: None,
            bucket_duration_ms: None,
        };
        assert!(request.validate().is_err());
    }
}
