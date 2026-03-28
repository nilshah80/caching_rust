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

/// Create/alter options for a time series key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimeSeriesCreateOptions {
    pub retention_ms: Option<u64>,
    pub chunk_size: Option<u64>,
    pub duplicate_policy: Option<TsDuplicatePolicy>,
    pub labels: HashMap<String, String>,
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
    async fn ts_add(&self, key: &str, sample: TimeSeriesSample) -> Result<i64, CacheError>;
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
    ) -> Result<(), CacheError>;
    async fn ts_delete_rule(&self, source: &str, dest: &str) -> Result<(), CacheError>;
}
