//! Redis Probabilistic Repository Implementation
//!
//! Implementation of ProbabilisticRepository for Redis using RedisBloom module commands
//! (CMS, TopK) and core Redis commands (HyperLogLog).

use async_trait::async_trait;
use redis::{Value, cmd};
use std::sync::Arc;

use crate::domain::entities::{
    CmsIncrByResult, CmsInfo, CmsInitResult, CmsMergeResult, CmsQueryResult, PfAddResult,
    PfCountResult, PfMergeResult, TopKAddResult, TopKCountResult, TopKIncrByResult, TopKInfo,
    TopKItem, TopKListResult, TopKQueryResult, TopKReserveResult,
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
            .await
            .map_err(CacheError::from)?;

        Self::parse_topk_info(result)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
            .query_async(&mut *conn)
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
