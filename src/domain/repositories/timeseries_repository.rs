//! RedisTimeSeries repository trait.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::domain::errors::CacheError;

/// Duplicate policy for RedisTimeSeries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsDuplicatePolicy {
    Block,
    First,
    Last,
    Min,
    Max,
    Sum,
}

/// Aggregation type for range queries and rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsAggregation {
    Avg,
    Sum,
    Min,
    Max,
    Range,
    Count,
    First,
    Last,
    StdP,
    StdS,
    VarP,
    VarS,
    Twa,
}

/// A single time-series sample.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSeriesSample {
    pub timestamp: i64,
    pub value: f64,
}

/// IGNORE thresholds for TS.CREATE / TS.ALTER / TS.ADD (RedisTimeSeries 1.12+).
///
/// A new sample is treated as a duplicate (and silently dropped) when both
/// `timestamp - max_timestamp <= max_time_diff` and
/// `abs(value - value_at_max_timestamp) <= max_val_diff`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TsIgnore {
    pub max_time_diff: i64,
    pub max_val_diff: f64,
}

/// Create/alter options for a time series key.
///
/// `Eq` is intentionally not derived — `TsIgnore::max_val_diff` is `f64`,
/// which only implements `PartialEq`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimeSeriesCreateOptions {
    pub retention_ms: Option<u64>,
    pub chunk_size: Option<u64>,
    pub duplicate_policy: Option<TsDuplicatePolicy>,
    /// IGNORE thresholds (RedisTimeSeries 1.12+). When unset, Redis falls
    /// back to its global `IGNORE_MAX_TIME_DIFF` / `IGNORE_MAX_VAL_DIFF`
    /// configuration (defaults to 0/0 — i.e. no filtering).
    pub ignore: Option<TsIgnore>,
    pub labels: HashMap<String, String>,
}

/// Per-call options for TS.ADD (RedisTimeSeries 1.4+ for ON_DUPLICATE,
/// 1.12+ for IGNORE). Scoped to the 10.8 additions; this intentionally does
/// not surface every TS.ADD option (RETENTION/ENCODING/CHUNK_SIZE/LABELS
/// are not exposed because the existing add path created them via TS.CREATE).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimeSeriesAddOptions {
    /// Override the configured DUPLICATE_POLICY for this single sample
    /// (TS.ADD `ON_DUPLICATE`). Applies on every TS.ADD call.
    pub on_duplicate: Option<TsDuplicatePolicy>,
    /// IGNORE thresholds applied **only when TS.ADD creates the series**.
    /// Per Redis docs, IGNORE on TS.ADD is silently ignored if the series
    /// already exists — call TS.ALTER with `ignore` set to update an
    /// existing series.
    pub ignore: Option<TsIgnore>,
}

/// Query options for range-style commands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimeSeriesRangeOptions {
    pub count: Option<u64>,
    pub aggregation: Option<TsAggregation>,
    pub bucket_duration_ms: Option<u64>,
}

/// Latest sample result for MGET.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSeriesMGetResult {
    pub key: String,
    pub labels: HashMap<String, String>,
    pub sample: Option<TimeSeriesSample>,
}

/// Range result for MRANGE.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSeriesRangeResult {
    pub key: String,
    pub labels: HashMap<String, String>,
    pub samples: Vec<TimeSeriesSample>,
}

/// Repository trait for RedisTimeSeries operations.
#[async_trait]
pub trait TimeSeriesRepository: Send + Sync {
    async fn ts_create(
        &self,
        key: &str,
        options: TimeSeriesCreateOptions,
    ) -> Result<(), CacheError>;
    async fn ts_alter(&self, key: &str, options: TimeSeriesCreateOptions)
    -> Result<(), CacheError>;
    async fn ts_add(
        &self,
        key: &str,
        sample: TimeSeriesSample,
        options: TimeSeriesAddOptions,
    ) -> Result<i64, CacheError>;
    async fn ts_madd(&self, items: &[(String, TimeSeriesSample)]) -> Result<Vec<i64>, CacheError>;
    async fn ts_incr_by(
        &self,
        key: &str,
        value: f64,
        timestamp: Option<i64>,
    ) -> Result<i64, CacheError>;
    async fn ts_decr_by(
        &self,
        key: &str,
        value: f64,
        timestamp: Option<i64>,
    ) -> Result<i64, CacheError>;
    async fn ts_del(&self, key: &str, from: i64, to: i64) -> Result<i64, CacheError>;
    async fn ts_get(&self, key: &str) -> Result<Option<TimeSeriesSample>, CacheError>;
    async fn ts_mget(&self, filters: &[String]) -> Result<Vec<TimeSeriesMGetResult>, CacheError>;
    async fn ts_range(
        &self,
        key: &str,
        from: i64,
        to: i64,
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesSample>, CacheError>;
    async fn ts_rev_range(
        &self,
        key: &str,
        from: i64,
        to: i64,
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesSample>, CacheError>;
    async fn ts_mrange(
        &self,
        from: i64,
        to: i64,
        filters: &[String],
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesRangeResult>, CacheError>;
    async fn ts_mrev_range(
        &self,
        from: i64,
        to: i64,
        filters: &[String],
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesRangeResult>, CacheError>;
    async fn ts_query_index(&self, filters: &[String]) -> Result<Vec<String>, CacheError>;
    async fn ts_info(&self, key: &str) -> Result<serde_json::Value, CacheError>;
    async fn ts_create_rule(
        &self,
        source: &str,
        dest: &str,
        aggregation: TsAggregation,
        bucket_duration_ms: u64,
        align_timestamp_ms: Option<i64>,
    ) -> Result<(), CacheError>;
    async fn ts_delete_rule(&self, source: &str, dest: &str) -> Result<(), CacheError>;
}
