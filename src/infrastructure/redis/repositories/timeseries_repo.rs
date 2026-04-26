use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    TimeSeriesAddOptions, TimeSeriesCreateOptions, TimeSeriesMGetResult, TimeSeriesRangeOptions,
    TimeSeriesRangeResult, TimeSeriesRepository, TimeSeriesSample, TsAggregation,
    TsDuplicatePolicy,
};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::shared::redis_value::redis_value_to_json;

#[derive(Clone)]
pub struct RedisTimeSeriesRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisTimeSeriesRepository {
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }

    fn apply_create_options(cmd: &mut redis::Cmd, options: &TimeSeriesCreateOptions) {
        if let Some(retention) = options.retention_ms {
            cmd.arg("RETENTION").arg(retention);
        }
        if let Some(chunk_size) = options.chunk_size {
            cmd.arg("CHUNK_SIZE").arg(chunk_size);
        }
        if let Some(policy) = options.duplicate_policy {
            cmd.arg("DUPLICATE_POLICY")
                .arg(duplicate_policy_token(policy));
        }
        if let Some(ig) = &options.ignore {
            cmd.arg("IGNORE").arg(ig.max_time_diff).arg(ig.max_val_diff);
        }
        if !options.labels.is_empty() {
            cmd.arg("LABELS");
            for (key, value) in &options.labels {
                cmd.arg(key).arg(value);
            }
        }
    }

    /// Append the TS.ADD options surfaced by the API (10.8 additions only):
    /// `ON_DUPLICATE` is per-call; `IGNORE` is only honored when TS.ADD
    /// creates the series — Redis silently drops it on an existing series.
    fn apply_add_options(cmd: &mut redis::Cmd, options: &TimeSeriesAddOptions) {
        if let Some(policy) = options.on_duplicate {
            cmd.arg("ON_DUPLICATE").arg(duplicate_policy_token(policy));
        }
        if let Some(ig) = &options.ignore {
            cmd.arg("IGNORE").arg(ig.max_time_diff).arg(ig.max_val_diff);
        }
    }

    fn apply_range_options(cmd: &mut redis::Cmd, options: &TimeSeriesRangeOptions) {
        if let Some(count) = options.count {
            cmd.arg("COUNT").arg(count);
        }
        if let (Some(aggregation), Some(bucket)) = (options.aggregation, options.bucket_duration_ms)
        {
            cmd.arg("AGGREGATION")
                .arg(aggregation_token(aggregation))
                .arg(bucket);
        }
    }
}

#[async_trait]
impl TimeSeriesRepository for RedisTimeSeriesRepository {
    async fn ts_create(
        &self,
        key: &str,
        options: TimeSeriesCreateOptions,
    ) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.CREATE");
        cmd.arg(key);
        Self::apply_create_options(&mut cmd, &options);
        let _: () = cmd.query_async(&mut conn).await?;
        Ok(())
    }

    async fn ts_alter(
        &self,
        key: &str,
        options: TimeSeriesCreateOptions,
    ) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.ALTER");
        cmd.arg(key);
        Self::apply_create_options(&mut cmd, &options);
        let _: () = cmd.query_async(&mut conn).await?;
        Ok(())
    }

    async fn ts_add(
        &self,
        key: &str,
        sample: TimeSeriesSample,
        options: TimeSeriesAddOptions,
    ) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.ADD");
        cmd.arg(key).arg(sample.timestamp).arg(sample.value);
        Self::apply_add_options(&mut cmd, &options);
        let timestamp: i64 = cmd.query_async(&mut conn).await?;
        Ok(timestamp)
    }

    async fn ts_madd(&self, items: &[(String, TimeSeriesSample)]) -> Result<Vec<i64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.MADD");
        for (key, sample) in items {
            cmd.arg(key).arg(sample.timestamp).arg(sample.value);
        }
        let timestamps: Vec<i64> = cmd.query_async(&mut conn).await?;
        Ok(timestamps)
    }

    async fn ts_incr_by(
        &self,
        key: &str,
        value: f64,
        timestamp: Option<i64>,
    ) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.INCRBY");
        cmd.arg(key).arg(value);
        if let Some(timestamp) = timestamp {
            cmd.arg("TIMESTAMP").arg(timestamp);
        }
        let result: i64 = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn ts_decr_by(
        &self,
        key: &str,
        value: f64,
        timestamp: Option<i64>,
    ) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.DECRBY");
        cmd.arg(key).arg(value);
        if let Some(timestamp) = timestamp {
            cmd.arg("TIMESTAMP").arg(timestamp);
        }
        let result: i64 = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn ts_del(&self, key: &str, from: i64, to: i64) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let deleted: i64 = redis::cmd("TS.DEL")
            .arg(key)
            .arg(from)
            .arg(to)
            .query_async(&mut conn)
            .await?;
        Ok(deleted)
    }

    async fn ts_get(&self, key: &str) -> Result<Option<TimeSeriesSample>, CacheError> {
        let mut conn = self.pool.get().await?;
        let value: redis::Value = redis::cmd("TS.GET").arg(key).query_async(&mut conn).await?;
        match value {
            redis::Value::Nil => Ok(None),
            redis::Value::Array(items) if items.len() == 2 => Ok(Some(TimeSeriesSample {
                timestamp: parse_i64(&items[0])?,
                value: parse_f64(&items[1])?,
            })),
            other => Err(CacheError::Internal(format!(
                "Unexpected TS.GET response: {other:?}"
            ))),
        }
    }

    async fn ts_mget(&self, filters: &[String]) -> Result<Vec<TimeSeriesMGetResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.MGET");
        cmd.arg("WITHLABELS").arg("FILTER");
        for filter in filters {
            cmd.arg(filter);
        }
        let value: redis::Value = cmd.query_async(&mut conn).await?;
        parse_mget(value)
    }

    async fn ts_range(
        &self,
        key: &str,
        from: i64,
        to: i64,
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesSample>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.RANGE");
        cmd.arg(key).arg(from).arg(to);
        Self::apply_range_options(&mut cmd, &options);
        let value: redis::Value = cmd.query_async(&mut conn).await?;
        parse_sample_list(value)
    }

    async fn ts_rev_range(
        &self,
        key: &str,
        from: i64,
        to: i64,
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesSample>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.REVRANGE");
        cmd.arg(key).arg(from).arg(to);
        Self::apply_range_options(&mut cmd, &options);
        let value: redis::Value = cmd.query_async(&mut conn).await?;
        parse_sample_list(value)
    }

    async fn ts_mrange(
        &self,
        from: i64,
        to: i64,
        filters: &[String],
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesRangeResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.MRANGE");
        cmd.arg(from).arg(to);
        Self::apply_range_options(&mut cmd, &options);
        cmd.arg("WITHLABELS").arg("FILTER");
        for filter in filters {
            cmd.arg(filter);
        }
        let value: redis::Value = cmd.query_async(&mut conn).await?;
        parse_mrange(value)
    }

    async fn ts_mrev_range(
        &self,
        from: i64,
        to: i64,
        filters: &[String],
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesRangeResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.MREVRANGE");
        cmd.arg(from).arg(to);
        Self::apply_range_options(&mut cmd, &options);
        cmd.arg("WITHLABELS").arg("FILTER");
        for filter in filters {
            cmd.arg(filter);
        }
        let value: redis::Value = cmd.query_async(&mut conn).await?;
        parse_mrange(value)
    }

    async fn ts_query_index(&self, filters: &[String]) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.QUERYINDEX");
        for filter in filters {
            cmd.arg(filter);
        }
        let result: Vec<String> = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn ts_info(&self, key: &str) -> Result<serde_json::Value, CacheError> {
        let mut conn = self.pool.get().await?;
        let value: redis::Value = redis::cmd("TS.INFO")
            .arg(key)
            .query_async(&mut conn)
            .await?;
        Ok(redis_value_to_json(value))
    }

    async fn ts_create_rule(
        &self,
        source: &str,
        dest: &str,
        aggregation: TsAggregation,
        bucket_duration_ms: u64,
        align_timestamp_ms: Option<i64>,
    ) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("TS.CREATERULE");
        cmd.arg(source)
            .arg(dest)
            .arg("AGGREGATION")
            .arg(aggregation_token(aggregation))
            .arg(bucket_duration_ms);
        // alignTimestamp is a trailing positional arg in TS.CREATERULE,
        // present since RedisTimeSeries 1.8. Older RTS will reject it
        // with "wrong number of arguments" — surfaced as a Redis error.
        if let Some(align) = align_timestamp_ms {
            cmd.arg(align);
        }
        let _: () = cmd.query_async(&mut conn).await?;
        Ok(())
    }

    async fn ts_delete_rule(&self, source: &str, dest: &str) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let _: () = redis::cmd("TS.DELETERULE")
            .arg(source)
            .arg(dest)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }
}

/// Wire token for a `TsDuplicatePolicy`, shared by DUPLICATE_POLICY (CREATE/ALTER)
/// and ON_DUPLICATE (ADD).
fn duplicate_policy_token(policy: TsDuplicatePolicy) -> &'static str {
    match policy {
        TsDuplicatePolicy::Block => "BLOCK",
        TsDuplicatePolicy::First => "FIRST",
        TsDuplicatePolicy::Last => "LAST",
        TsDuplicatePolicy::Min => "MIN",
        TsDuplicatePolicy::Max => "MAX",
        TsDuplicatePolicy::Sum => "SUM",
    }
}

/// Wire token for a `TsAggregation`, shared by AGGREGATION clauses in
/// range queries and CREATERULE.
fn aggregation_token(aggregation: TsAggregation) -> &'static str {
    match aggregation {
        TsAggregation::Avg => "avg",
        TsAggregation::Sum => "sum",
        TsAggregation::Min => "min",
        TsAggregation::Max => "max",
        TsAggregation::Range => "range",
        TsAggregation::Count => "count",
        TsAggregation::First => "first",
        TsAggregation::Last => "last",
        TsAggregation::StdP => "std.p",
        TsAggregation::StdS => "std.s",
        TsAggregation::VarP => "var.p",
        TsAggregation::VarS => "var.s",
        TsAggregation::Twa => "twa",
    }
}

fn parse_i64(value: &redis::Value) -> Result<i64, CacheError> {
    match value {
        redis::Value::Int(i) => Ok(*i),
        redis::Value::BulkString(bytes) => String::from_utf8(bytes.clone())
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| CacheError::Internal("Invalid integer response".to_string())),
        redis::Value::SimpleString(s) => s
            .parse::<i64>()
            .map_err(|_| CacheError::Internal("Invalid integer response".to_string())),
        other => Err(CacheError::Internal(format!(
            "Unexpected integer response: {other:?}"
        ))),
    }
}

fn parse_f64(value: &redis::Value) -> Result<f64, CacheError> {
    match value {
        redis::Value::Double(f) => Ok(*f),
        redis::Value::Int(i) => Ok(*i as f64),
        redis::Value::BulkString(bytes) => String::from_utf8(bytes.clone())
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| CacheError::Internal("Invalid float response".to_string())),
        redis::Value::SimpleString(s) => s
            .parse::<f64>()
            .map_err(|_| CacheError::Internal("Invalid float response".to_string())),
        other => Err(CacheError::Internal(format!(
            "Unexpected float response: {other:?}"
        ))),
    }
}

fn parse_labels(value: &redis::Value) -> Result<HashMap<String, String>, CacheError> {
    match value {
        redis::Value::Array(items) => {
            let mut labels = HashMap::new();
            for item in items {
                if let redis::Value::Array(pair) = item
                    && pair.len() == 2
                {
                    labels.insert(parse_string(&pair[0])?, parse_string(&pair[1])?);
                }
            }
            Ok(labels)
        }
        redis::Value::Map(items) => {
            let mut labels = HashMap::new();
            for (k, v) in items {
                labels.insert(parse_string(k)?, parse_string(v)?);
            }
            Ok(labels)
        }
        other => Err(CacheError::Internal(format!(
            "Unexpected labels response: {other:?}"
        ))),
    }
}

fn parse_string(value: &redis::Value) -> Result<String, CacheError> {
    match value {
        redis::Value::BulkString(bytes) => String::from_utf8(bytes.clone())
            .map_err(|_| CacheError::Internal("Invalid UTF-8 string".to_string())),
        redis::Value::SimpleString(s) => Ok(s.clone()),
        redis::Value::Int(i) => Ok(i.to_string()),
        other => Err(CacheError::Internal(format!(
            "Unexpected string response: {other:?}"
        ))),
    }
}

fn parse_sample_list(value: redis::Value) -> Result<Vec<TimeSeriesSample>, CacheError> {
    match value {
        redis::Value::Array(items) => items
            .into_iter()
            .map(|item| match item {
                redis::Value::Array(sample) if sample.len() == 2 => Ok(TimeSeriesSample {
                    timestamp: parse_i64(&sample[0])?,
                    value: parse_f64(&sample[1])?,
                }),
                other => Err(CacheError::Internal(format!(
                    "Unexpected sample response: {other:?}"
                ))),
            })
            .collect(),
        other => Err(CacheError::Internal(format!(
            "Unexpected TS.RANGE response: {other:?}"
        ))),
    }
}

fn parse_mget(value: redis::Value) -> Result<Vec<TimeSeriesMGetResult>, CacheError> {
    match value {
        redis::Value::Array(series) => series
            .into_iter()
            .map(|entry| match entry {
                redis::Value::Array(parts) if parts.len() == 3 => {
                    let sample = match &parts[2] {
                        redis::Value::Nil => None,
                        redis::Value::Array(sample) if sample.len() == 2 => {
                            Some(TimeSeriesSample {
                                timestamp: parse_i64(&sample[0])?,
                                value: parse_f64(&sample[1])?,
                            })
                        }
                        other => {
                            return Err(CacheError::Internal(format!(
                                "Unexpected TS.MGET sample response: {other:?}"
                            )));
                        }
                    };
                    Ok(TimeSeriesMGetResult {
                        key: parse_string(&parts[0])?,
                        labels: parse_labels(&parts[1])?,
                        sample,
                    })
                }
                other => Err(CacheError::Internal(format!(
                    "Unexpected TS.MGET response: {other:?}"
                ))),
            })
            .collect(),
        other => Err(CacheError::Internal(format!(
            "Unexpected TS.MGET response: {other:?}"
        ))),
    }
}

fn parse_mrange(value: redis::Value) -> Result<Vec<TimeSeriesRangeResult>, CacheError> {
    match value {
        redis::Value::Array(series) => series
            .into_iter()
            .map(|entry| match entry {
                redis::Value::Array(parts) if parts.len() == 3 => Ok(TimeSeriesRangeResult {
                    key: parse_string(&parts[0])?,
                    labels: parse_labels(&parts[1])?,
                    samples: parse_sample_list(parts[2].clone())?,
                }),
                other => Err(CacheError::Internal(format!(
                    "Unexpected TS.MRANGE response: {other:?}"
                ))),
            })
            .collect(),
        other => Err(CacheError::Internal(format!(
            "Unexpected TS.MRANGE response: {other:?}"
        ))),
    }
}
