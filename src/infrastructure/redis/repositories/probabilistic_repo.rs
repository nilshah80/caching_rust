//! Redis Probabilistic Repository Implementation
//!
//! Implementation of ProbabilisticRepository for Redis using RedisBloom module commands
//! (CMS, TopK) and core Redis commands (HyperLogLog).

use async_trait::async_trait;
use redis::{Value, cmd};
use std::sync::Arc;

use crate::domain::entities::{
    CmsIncrByResult, CmsInfo, CmsInitResult, CmsMergeResult, CmsQueryResult, PfAddResult,
    PfCountResult, PfMergeResult, TDigestAckResult, TDigestInfo, TDigestRanksResult,
    TDigestScalarResult, TDigestValuesResult, TopKAddResult, TopKCountResult, TopKIncrByResult,
    TopKInfo, TopKItem, TopKListResult, TopKQueryResult, TopKReserveResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::ProbabilisticRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Redis implementation of ProbabilisticRepository
pub struct RedisProbabilisticRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisProbabilisticRepository {
    /// Create a new RedisProbabilisticRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }

    /// Extract a u64 from Redis value
    fn extract_u64(value: &Value) -> u64 {
        match value {
            Value::Int(i) => *i as u64,
            Value::BulkString(bytes) => String::from_utf8_lossy(bytes)
                .trim()
                .parse::<u64>()
                .unwrap_or(0),
            _ => 0,
        }
    }

    /// Extract a f64 from Redis value
    fn extract_f64(value: &Value) -> f64 {
        match value {
            Value::Double(f) => *f,
            Value::BulkString(bytes) => String::from_utf8_lossy(bytes)
                .trim()
                .parse::<f64>()
                .unwrap_or(0.0),
            Value::Int(i) => *i as f64,
            _ => 0.0,
        }
    }

    /// Extract an Option<f64> preserving `nan` and `Value::Nil` as None.
    ///
    /// T-Digest returns `nan` for operations on empty sketches (e.g. MIN/MAX/QUANTILE)
    /// and sometimes sends it as the string "nan"; upstream JSON can't carry NaN,
    /// so we surface that as a JSON `null` at the API boundary.
    fn extract_optional_f64(value: &Value) -> Option<f64> {
        match value {
            Value::Double(f) if f.is_nan() => None,
            Value::Double(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            Value::BulkString(bytes) => {
                let s = String::from_utf8_lossy(bytes);
                let trimmed = s.trim();
                if trimmed.eq_ignore_ascii_case("nan") {
                    None
                } else {
                    trimmed.parse::<f64>().ok().and_then(
                        |v| {
                            if v.is_nan() { None } else { Some(v) }
                        },
                    )
                }
            }
            Value::SimpleString(s) if s.eq_ignore_ascii_case("nan") => None,
            Value::SimpleString(s) => s.parse::<f64>().ok(),
            Value::Nil => None,
            _ => None,
        }
    }

    /// Extract a signed 64-bit integer. T-Digest uses -1 (out of range)
    /// and -2 (empty sketch) as sentinels, so we preserve the sign.
    fn extract_i64(value: &Value) -> i64 {
        match value {
            Value::Int(i) => *i,
            Value::BulkString(bytes) => String::from_utf8_lossy(bytes)
                .trim()
                .parse::<i64>()
                .unwrap_or(0),
            _ => 0,
        }
    }

    /// Extract a string from Redis value
    fn extract_string(value: &Value) -> Option<String> {
        match value {
            Value::BulkString(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
            Value::SimpleString(s) => Some(s.clone()),
            Value::Nil => None,
            _ => None,
        }
    }

    /// Parse CMS.INFO response
    fn parse_cms_info(value: Value) -> Result<CmsInfo, CacheError> {
        match value {
            Value::Array(arr) => {
                let mut info = CmsInfo {
                    width: 0,
                    depth: 0,
                    count: 0,
                };

                let mut iter = arr.iter();
                while let Some(key) = iter.next() {
                    if let Value::BulkString(k) = key {
                        let key_str = String::from_utf8_lossy(k);
                        if let Some(val) = iter.next() {
                            match key_str.as_ref() {
                                "width" => info.width = Self::extract_u64(val),
                                "depth" => info.depth = Self::extract_u64(val),
                                "count" => info.count = Self::extract_u64(val),
                                _ => {}
                            }
                        }
                    }
                }

                Ok(info)
            }
            _ => Err(CacheError::Internal(
                "Invalid CMS.INFO response".to_string(),
            )),
        }
    }

    /// Parse TDIGEST.INFO response (flat [name, value, name, value, ...] array).
    ///
    /// Fields returned by the module vary slightly across versions; we read by name
    /// and fall back to 0 for fields we don't recognise so newer Redis builds
    /// don't break older clients.
    fn parse_tdigest_info(value: Value) -> Result<TDigestInfo, CacheError> {
        let arr = match value {
            Value::Array(a) => a,
            _ => {
                return Err(CacheError::Internal(
                    "Invalid TDIGEST.INFO response".to_string(),
                ));
            }
        };

        let mut info = TDigestInfo {
            compression: 0,
            capacity: 0,
            merged_nodes: 0,
            unmerged_nodes: 0,
            merged_weight: 0.0,
            unmerged_weight: 0.0,
            observations: 0,
            total_compressions: 0,
            memory_usage: 0,
        };

        let mut iter = arr.iter();
        while let Some(key) = iter.next() {
            let key_str = match key {
                Value::BulkString(b) => String::from_utf8_lossy(b).to_string(),
                Value::SimpleString(s) => s.clone(),
                _ => continue,
            };
            let Some(val) = iter.next() else { break };
            let key_lower = key_str.to_ascii_lowercase();
            match key_lower.as_str() {
                "compression" => info.compression = Self::extract_u64(val),
                "capacity" => info.capacity = Self::extract_u64(val),
                "merged nodes" | "merged_nodes" => info.merged_nodes = Self::extract_u64(val),
                "unmerged nodes" | "unmerged_nodes" => info.unmerged_nodes = Self::extract_u64(val),
                "merged weight" | "merged_weight" => info.merged_weight = Self::extract_f64(val),
                "unmerged weight" | "unmerged_weight" => {
                    info.unmerged_weight = Self::extract_f64(val);
                }
                "observations" => info.observations = Self::extract_u64(val),
                "total compressions" | "total_compressions" => {
                    info.total_compressions = Self::extract_u64(val);
                }
                "memory usage" | "memory_usage" => info.memory_usage = Self::extract_u64(val),
                _ => {}
            }
        }

        Ok(info)
    }

    /// Parse TOPK.INFO response
    fn parse_topk_info(value: Value) -> Result<TopKInfo, CacheError> {
        match value {
            Value::Array(arr) => {
                let mut info = TopKInfo {
                    k: 0,
                    width: 0,
                    depth: 0,
                    decay: 0.9,
                };

                let mut iter = arr.iter();
                while let Some(key) = iter.next() {
                    if let Value::BulkString(k) = key {
                        let key_str = String::from_utf8_lossy(k);
                        if let Some(val) = iter.next() {
                            match key_str.as_ref() {
                                "k" => info.k = Self::extract_u64(val),
                                "width" => info.width = Self::extract_u64(val),
                                "depth" => info.depth = Self::extract_u64(val),
                                "decay" => info.decay = Self::extract_f64(val),
                                _ => {}
                            }
                        }
                    }
                }

                Ok(info)
            }
            _ => Err(CacheError::Internal(
                "Invalid TOPK.INFO response".to_string(),
            )),
        }
    }
}

#[async_trait]
impl ProbabilisticRepository for RedisProbabilisticRepository {
    // ==================== Count-Min Sketch Operations ====================

    async fn cms_init_by_dim(
        &self,
        key: &str,
        width: u64,
        depth: u64,
    ) -> Result<CmsInitResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = cmd("CMS.INITBYDIM")
            .arg(key)
            .arg(width)
            .arg(depth)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let success = matches!(result, Value::Okay);

        Ok(CmsInitResult {
            key: key.to_string(),
            success,
        })
    }

    async fn cms_init_by_prob(
        &self,
        key: &str,
        error: f64,
        probability: f64,
    ) -> Result<CmsInitResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = cmd("CMS.INITBYPROB")
            .arg(key)
            .arg(error)
            .arg(probability)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let success = matches!(result, Value::Okay);

        Ok(CmsInitResult {
            key: key.to_string(),
            success,
        })
    }

    async fn cms_incr_by(
        &self,
        key: &str,
        items: Vec<(String, u64)>,
    ) -> Result<CmsIncrByResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("CMS.INCRBY");
        command.arg(key);
        for (item, increment) in &items {
            command.arg(item).arg(*increment);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let counts = match result {
            Value::Array(arr) => arr.iter().map(Self::extract_u64).collect(),
            _ => vec![],
        };

        Ok(CmsIncrByResult {
            key: key.to_string(),
            counts,
        })
    }

    async fn cms_query(&self, key: &str, items: Vec<String>) -> Result<CmsQueryResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("CMS.QUERY");
        command.arg(key);
        for item in &items {
            command.arg(item);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let counts = match result {
            Value::Array(arr) => arr.iter().map(Self::extract_u64).collect(),
            _ => vec![],
        };

        Ok(CmsQueryResult {
            key: key.to_string(),
            counts,
        })
    }

    async fn cms_merge(
        &self,
        dest: &str,
        sources: Vec<String>,
        weights: Option<Vec<u64>>,
    ) -> Result<CmsMergeResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("CMS.MERGE");
        command.arg(dest).arg(sources.len());
        for source in &sources {
            command.arg(source);
        }

        if let Some(w) = weights {
            command.arg("WEIGHTS");
            for weight in w {
                command.arg(weight);
            }
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let success = matches!(result, Value::Okay);

        Ok(CmsMergeResult {
            key: dest.to_string(),
            success,
        })
    }

    async fn cms_info(&self, key: &str) -> Result<CmsInfo, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = cmd("CMS.INFO")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Self::parse_cms_info(result)
    }

    // ==================== Top-K Operations ====================

    async fn topk_reserve(
        &self,
        key: &str,
        k: u64,
        width: Option<u64>,
        depth: Option<u64>,
        decay: Option<f64>,
    ) -> Result<TopKReserveResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TOPK.RESERVE");
        command.arg(key).arg(k);

        if let Some(w) = width {
            command.arg(w);
            if let Some(d) = depth {
                command.arg(d);
                if let Some(decay_val) = decay {
                    command.arg(decay_val);
                }
            }
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let success = matches!(result, Value::Okay);

        Ok(TopKReserveResult {
            key: key.to_string(),
            success,
        })
    }

    async fn topk_add(&self, key: &str, items: Vec<String>) -> Result<TopKAddResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TOPK.ADD");
        command.arg(key);
        for item in &items {
            command.arg(item);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let dropped = match result {
            Value::Array(arr) => arr.iter().map(Self::extract_string).collect(),
            _ => vec![],
        };

        Ok(TopKAddResult {
            key: key.to_string(),
            dropped,
        })
    }

    async fn topk_incr_by(
        &self,
        key: &str,
        items: Vec<(String, u64)>,
    ) -> Result<TopKIncrByResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TOPK.INCRBY");
        command.arg(key);
        for (item, increment) in &items {
            command.arg(item).arg(*increment);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let dropped = match result {
            Value::Array(arr) => arr.iter().map(Self::extract_string).collect(),
            _ => vec![],
        };

        Ok(TopKIncrByResult {
            key: key.to_string(),
            dropped,
        })
    }

    async fn topk_query(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<TopKQueryResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TOPK.QUERY");
        command.arg(key);
        for item in &items {
            command.arg(item);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let results = match result {
            Value::Array(arr) => arr.iter().map(|v| matches!(v, Value::Int(1))).collect(),
            _ => vec![],
        };

        Ok(TopKQueryResult {
            key: key.to_string(),
            results,
        })
    }

    async fn topk_count(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<TopKCountResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TOPK.COUNT");
        command.arg(key);
        for item in &items {
            command.arg(item);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let counts = match result {
            Value::Array(arr) => arr.iter().map(Self::extract_u64).collect(),
            _ => vec![],
        };

        Ok(TopKCountResult {
            key: key.to_string(),
            counts,
        })
    }

    async fn topk_list(&self, key: &str, with_count: bool) -> Result<TopKListResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TOPK.LIST");
        command.arg(key);
        if with_count {
            command.arg("WITHCOUNT");
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let items = match result {
            Value::Array(arr) => {
                if with_count {
                    // With WITHCOUNT, response is: item1, count1, item2, count2, ...
                    arr.chunks(2)
                        .filter_map(|chunk| {
                            if chunk.len() == 2 {
                                let item = Self::extract_string(&chunk[0])?;
                                let count = Self::extract_u64(&chunk[1]);
                                Some(TopKItem { item, count })
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    // Without WITHCOUNT, response is: item1, item2, ...
                    arr.iter()
                        .filter_map(|v| {
                            let item = Self::extract_string(v)?;
                            Some(TopKItem { item, count: 0 })
                        })
                        .collect()
                }
            }
            _ => vec![],
        };

        Ok(TopKListResult {
            key: key.to_string(),
            items,
        })
    }

    async fn topk_info(&self, key: &str) -> Result<TopKInfo, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = cmd("TOPK.INFO")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Self::parse_topk_info(result)
    }

    // ==================== T-Digest Operations ====================

    async fn tdigest_create(
        &self,
        key: &str,
        compression: Option<u64>,
    ) -> Result<TDigestAckResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TDIGEST.CREATE");
        command.arg(key);
        if let Some(c) = compression {
            command.arg("COMPRESSION").arg(c);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Ok(TDigestAckResult {
            key: key.to_string(),
            success: matches!(result, Value::Okay),
        })
    }

    async fn tdigest_add(
        &self,
        key: &str,
        values: Vec<f64>,
    ) -> Result<TDigestAckResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TDIGEST.ADD");
        command.arg(key);
        for v in &values {
            command.arg(*v);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Ok(TDigestAckResult {
            key: key.to_string(),
            success: matches!(result, Value::Okay),
        })
    }

    async fn tdigest_quantile(
        &self,
        key: &str,
        quantiles: Vec<f64>,
    ) -> Result<TDigestValuesResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TDIGEST.QUANTILE");
        command.arg(key);
        for q in &quantiles {
            command.arg(*q);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let values = match result {
            Value::Array(arr) => arr.iter().map(Self::extract_optional_f64).collect(),
            _ => vec![],
        };

        Ok(TDigestValuesResult {
            key: key.to_string(),
            values,
        })
    }

    async fn tdigest_cdf(
        &self,
        key: &str,
        values: Vec<f64>,
    ) -> Result<TDigestValuesResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TDIGEST.CDF");
        command.arg(key);
        for v in &values {
            command.arg(*v);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let parsed = match result {
            Value::Array(arr) => arr.iter().map(Self::extract_optional_f64).collect(),
            _ => vec![],
        };

        Ok(TDigestValuesResult {
            key: key.to_string(),
            values: parsed,
        })
    }

    async fn tdigest_rank(
        &self,
        key: &str,
        values: Vec<f64>,
    ) -> Result<TDigestRanksResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TDIGEST.RANK");
        command.arg(key);
        for v in &values {
            command.arg(*v);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let ranks = match result {
            Value::Array(arr) => arr.iter().map(Self::extract_i64).collect(),
            _ => vec![],
        };

        Ok(TDigestRanksResult {
            key: key.to_string(),
            ranks,
        })
    }

    async fn tdigest_revrank(
        &self,
        key: &str,
        values: Vec<f64>,
    ) -> Result<TDigestRanksResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TDIGEST.REVRANK");
        command.arg(key);
        for v in &values {
            command.arg(*v);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let ranks = match result {
            Value::Array(arr) => arr.iter().map(Self::extract_i64).collect(),
            _ => vec![],
        };

        Ok(TDigestRanksResult {
            key: key.to_string(),
            ranks,
        })
    }

    async fn tdigest_byrank(
        &self,
        key: &str,
        ranks: Vec<u64>,
    ) -> Result<TDigestValuesResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TDIGEST.BYRANK");
        command.arg(key);
        for r in &ranks {
            command.arg(*r);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let values = match result {
            Value::Array(arr) => arr.iter().map(Self::extract_optional_f64).collect(),
            _ => vec![],
        };

        Ok(TDigestValuesResult {
            key: key.to_string(),
            values,
        })
    }

    async fn tdigest_byrevrank(
        &self,
        key: &str,
        ranks: Vec<u64>,
    ) -> Result<TDigestValuesResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TDIGEST.BYREVRANK");
        command.arg(key);
        for r in &ranks {
            command.arg(*r);
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        let values = match result {
            Value::Array(arr) => arr.iter().map(Self::extract_optional_f64).collect(),
            _ => vec![],
        };

        Ok(TDigestValuesResult {
            key: key.to_string(),
            values,
        })
    }

    async fn tdigest_min(&self, key: &str) -> Result<TDigestScalarResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = cmd("TDIGEST.MIN")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Ok(TDigestScalarResult {
            key: key.to_string(),
            value: Self::extract_optional_f64(&result),
        })
    }

    async fn tdigest_max(&self, key: &str) -> Result<TDigestScalarResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = cmd("TDIGEST.MAX")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Ok(TDigestScalarResult {
            key: key.to_string(),
            value: Self::extract_optional_f64(&result),
        })
    }

    async fn tdigest_info(&self, key: &str) -> Result<TDigestInfo, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = cmd("TDIGEST.INFO")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Self::parse_tdigest_info(result)
    }

    async fn tdigest_merge(
        &self,
        dest: &str,
        sources: Vec<String>,
        compression: Option<u64>,
        override_existing: bool,
    ) -> Result<TDigestAckResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("TDIGEST.MERGE");
        command.arg(dest).arg(sources.len());
        for source in &sources {
            command.arg(source);
        }
        if let Some(c) = compression {
            command.arg("COMPRESSION").arg(c);
        }
        if override_existing {
            command.arg("OVERRIDE");
        }

        let result: Value = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Ok(TDigestAckResult {
            key: dest.to_string(),
            success: matches!(result, Value::Okay),
        })
    }

    async fn tdigest_reset(&self, key: &str) -> Result<TDigestAckResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = cmd("TDIGEST.RESET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Ok(TDigestAckResult {
            key: key.to_string(),
            success: matches!(result, Value::Okay),
        })
    }

    async fn tdigest_trimmed_mean(
        &self,
        key: &str,
        low_cut_quantile: f64,
        high_cut_quantile: f64,
    ) -> Result<TDigestScalarResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = cmd("TDIGEST.TRIMMED_MEAN")
            .arg(key)
            .arg(low_cut_quantile)
            .arg(high_cut_quantile)
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Ok(TDigestScalarResult {
            key: key.to_string(),
            value: Self::extract_optional_f64(&result),
        })
    }

    // ==================== HyperLogLog Operations ====================

    async fn pf_add(&self, key: &str, elements: Vec<String>) -> Result<PfAddResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("PFADD");
        command.arg(key);
        for element in &elements {
            command.arg(element);
        }

        let result: i64 = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Ok(PfAddResult {
            key: key.to_string(),
            changed: result == 1,
        })
    }

    async fn pf_count(&self, keys: Vec<String>) -> Result<PfCountResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("PFCOUNT");
        for key in &keys {
            command.arg(key);
        }

        let count: u64 = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Ok(PfCountResult { keys, count })
    }

    async fn pf_merge(
        &self,
        dest: &str,
        sources: Vec<String>,
    ) -> Result<PfMergeResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut command = cmd("PFMERGE");
        command.arg(dest);
        for source in &sources {
            command.arg(source);
        }

        let _: () = command
            .query_async(&mut conn)
            .await
            .map_err(CacheError::from)?;

        Ok(PfMergeResult {
            dest_key: dest.to_string(),
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_u64() {
        assert_eq!(
            RedisProbabilisticRepository::extract_u64(&Value::Int(42)),
            42
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_u64(&Value::BulkString(b"100".to_vec())),
            100
        );
        assert_eq!(RedisProbabilisticRepository::extract_u64(&Value::Nil), 0);
    }

    #[test]
    fn test_extract_f64() {
        assert_eq!(
            RedisProbabilisticRepository::extract_f64(&Value::Double(0.9)),
            0.9
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_f64(&Value::Int(1)),
            1.0
        );
    }

    #[test]
    fn test_extract_string() {
        assert_eq!(
            RedisProbabilisticRepository::extract_string(&Value::BulkString(b"test".to_vec())),
            Some("test".to_string())
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_string(&Value::Nil),
            None
        );
    }

    #[test]
    fn test_parse_cms_info() {
        let value = Value::Array(vec![
            Value::BulkString(b"width".to_vec()),
            Value::Int(2000),
            Value::BulkString(b"depth".to_vec()),
            Value::Int(5),
            Value::BulkString(b"count".to_vec()),
            Value::Int(1000),
        ]);

        let info = RedisProbabilisticRepository::parse_cms_info(value).unwrap();
        assert_eq!(info.width, 2000);
        assert_eq!(info.depth, 5);
        assert_eq!(info.count, 1000);
    }

    #[test]
    fn test_extract_optional_f64_nan_becomes_none() {
        assert_eq!(
            RedisProbabilisticRepository::extract_optional_f64(&Value::Double(f64::NAN)),
            None
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_optional_f64(&Value::Double(1.5)),
            Some(1.5)
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_optional_f64(&Value::BulkString(b"nan".to_vec())),
            None
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_optional_f64(&Value::BulkString(b"NaN".to_vec())),
            None
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_optional_f64(&Value::BulkString(b"2.5".to_vec())),
            Some(2.5)
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_optional_f64(&Value::Int(7)),
            Some(7.0)
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_optional_f64(&Value::Nil),
            None
        );
    }

    #[test]
    fn test_extract_i64_handles_sentinels() {
        assert_eq!(
            RedisProbabilisticRepository::extract_i64(&Value::Int(-2)),
            -2
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_i64(&Value::Int(-1)),
            -1
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_i64(&Value::Int(42)),
            42
        );
        assert_eq!(
            RedisProbabilisticRepository::extract_i64(&Value::BulkString(b"-1".to_vec())),
            -1
        );
    }

    #[test]
    fn test_parse_tdigest_info() {
        let value = Value::Array(vec![
            Value::BulkString(b"Compression".to_vec()),
            Value::Int(100),
            Value::BulkString(b"Capacity".to_vec()),
            Value::Int(610),
            Value::BulkString(b"Merged nodes".to_vec()),
            Value::Int(10),
            Value::BulkString(b"Unmerged nodes".to_vec()),
            Value::Int(2),
            Value::BulkString(b"Merged weight".to_vec()),
            Value::Double(100.0),
            Value::BulkString(b"Unmerged weight".to_vec()),
            Value::Double(5.0),
            Value::BulkString(b"Observations".to_vec()),
            Value::Int(105),
            Value::BulkString(b"Total compressions".to_vec()),
            Value::Int(1),
            Value::BulkString(b"Memory usage".to_vec()),
            Value::Int(2048),
        ]);
        let info = RedisProbabilisticRepository::parse_tdigest_info(value).unwrap();
        assert_eq!(info.compression, 100);
        assert_eq!(info.capacity, 610);
        assert_eq!(info.merged_nodes, 10);
        assert_eq!(info.unmerged_nodes, 2);
        assert_eq!(info.merged_weight, 100.0);
        assert_eq!(info.unmerged_weight, 5.0);
        assert_eq!(info.observations, 105);
        assert_eq!(info.total_compressions, 1);
        assert_eq!(info.memory_usage, 2048);
    }

    #[test]
    fn test_parse_tdigest_info_tolerates_unknown_and_missing_fields() {
        // Only compression provided — other fields should default to 0.
        let value = Value::Array(vec![
            Value::BulkString(b"Compression".to_vec()),
            Value::Int(50),
            Value::BulkString(b"Some Future Field".to_vec()),
            Value::Int(999),
        ]);
        let info = RedisProbabilisticRepository::parse_tdigest_info(value).unwrap();
        assert_eq!(info.compression, 50);
        assert_eq!(info.capacity, 0);
        assert_eq!(info.memory_usage, 0);
    }

    #[test]
    fn test_parse_tdigest_info_rejects_non_array() {
        let result = RedisProbabilisticRepository::parse_tdigest_info(Value::Int(0));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_topk_info() {
        let value = Value::Array(vec![
            Value::BulkString(b"k".to_vec()),
            Value::Int(10),
            Value::BulkString(b"width".to_vec()),
            Value::Int(2000),
            Value::BulkString(b"depth".to_vec()),
            Value::Int(7),
            Value::BulkString(b"decay".to_vec()),
            Value::Double(0.9),
        ]);

        let info = RedisProbabilisticRepository::parse_topk_info(value).unwrap();
        assert_eq!(info.k, 10);
        assert_eq!(info.width, 2000);
        assert_eq!(info.depth, 7);
        assert_eq!(info.decay, 0.9);
    }
}
