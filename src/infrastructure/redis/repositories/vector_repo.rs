//! Vector Repository Implementation
//!
//! Implements Vector Sets operations using deadpool-redis.

use crate::infrastructure::redis::connection::InstrumentedPool;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::instrument;

use crate::domain::entities::{
    VectorAddResult, VectorInfo, VectorItem, VectorRangeResult, VectorSimResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::VectorRepository;

/// Implementation of the VectorRepository trait using Redis
#[derive(Clone)]
pub struct RedisVectorRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisVectorRepository {
    /// Create a new RedisVectorRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }

    /// Extract a u64 from a redis::Value, handling both Int and string-encoded forms.
    fn extract_u64_value(val: &redis::Value) -> Option<u64> {
        match val {
            redis::Value::Int(n) => Some(*n as u64),
            redis::Value::BulkString(bytes) => {
                String::from_utf8_lossy(bytes).trim().parse::<u64>().ok()
            }
            redis::Value::SimpleString(s) => s.trim().parse::<u64>().ok(),
            redis::Value::Double(f) => Some(*f as u64),
            _ => None,
        }
    }

    /// Extract a String from a redis::Value.
    fn extract_string_value(val: &redis::Value) -> Option<String> {
        match val {
            redis::Value::BulkString(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
            redis::Value::SimpleString(s) => Some(s.clone()),
            redis::Value::Int(n) => Some(n.to_string()),
            _ => None,
        }
    }

    /// Helper to get a connection from the pool
    async fn get_connection(
        &self,
    ) -> Result<crate::infrastructure::redis::pool_connection::PoolConnection, CacheError> {
        self.pool.get().await.map_err(|e| {
            CacheError::ConnectionFailed(format!("Failed to get Redis connection: {}", e))
        })
    }

    /// Classify a Redis error: known validation failures become InvalidInput (400),
    /// everything else stays as RedisError (500).
    fn classify_redis_error(e: redis::RedisError) -> CacheError {
        let msg = e.to_string();
        // Not-found errors from Redis (e.g. VDIM on non-existent key)
        if msg.contains("does not exist") || msg.contains("no such key") {
            return CacheError::KeyNotFound(msg);
        }
        let is_client_error = msg.contains("dimension mismatch")
            || msg.contains("Dimension mismatch")
            || msg.contains("DIMENSION_MISMATCH")
            || msg.contains("invalid vector")
            || msg.contains("invalid JSON")
            || msg.contains("not a valid")
            || msg.contains("wrong number of arguments")
            || msg.contains("not found in the vector set")
            || msg.contains("invalid specification");
        if is_client_error {
            CacheError::InvalidInput(msg)
        } else {
            CacheError::RedisError(e)
        }
    }
}

#[async_trait]
impl VectorRepository for RedisVectorRepository {
    #[instrument(skip(self, items), fields(key = %key))]
    async fn vadd(
        &self,
        key: &str,
        items: Vec<(String, Vec<f32>)>,
    ) -> Result<VectorAddResult, CacheError> {
        let mut conn = self.get_connection().await?;

        // Pre-validate: all vectors in a batch must have the same dimensionality.
        if let Some(first_dim) = items.first().map(|(_, v)| v.len()) {
            for (elem, vec) in &items {
                if vec.len() != first_dim {
                    return Err(CacheError::InvalidInput(format!(
                        "Dimension mismatch: element '{}' has {} dimensions, expected {}",
                        elem,
                        vec.len(),
                        first_dim
                    )));
                }
            }
        }

        // Atomic batch VADD using WATCH+MULTI+EXEC. No Lua/EVAL required.
        //  1. WATCH key — if key changes between now and EXEC, transaction aborts
        //  2. Preflight: check VDIM if key exists — reject on dimension mismatch
        //  3. MULTI — queue all VADD commands
        //  4. EXEC — either all commit or none (if key was modified by another client)
        let dim = items.first().map(|(_, v)| v.len()).unwrap_or(0);

        // WATCH the key so EXEC fails if another client modifies it mid-batch
        redis::cmd("WATCH")
            .arg(key)
            .query_async::<()>(&mut conn)
            .await
            .map_err(Self::classify_redis_error)?;

        // Preflight dimension check (under WATCH protection).
        // On any error, clear WATCH state before returning.
        let preflight_result = async {
            let exists: bool = redis::cmd("EXISTS")
                .arg(key)
                .query_async(&mut conn)
                .await
                .map_err(Self::classify_redis_error)?;
            if exists {
                let existing_dim: u64 = redis::cmd("VDIM")
                    .arg(key)
                    .query_async(&mut conn)
                    .await
                    .map_err(Self::classify_redis_error)?;
                if existing_dim != dim as u64 {
                    return Err(CacheError::InvalidInput(format!(
                        "Dimension mismatch: batch has {} dimensions but existing set has {}",
                        dim, existing_dim
                    )));
                }
            }
            Ok(())
        }
        .await;

        if let Err(e) = preflight_result {
            let _ = redis::cmd("UNWATCH").query_async::<()>(&mut conn).await;
            return Err(e);
        }

        // Build atomic pipeline: MULTI + all VADDs + EXEC
        let mut pipe = redis::pipe();
        pipe.atomic();
        for (element, vector) in &items {
            let mut cmd = redis::cmd("VADD");
            cmd.arg(key).arg("VALUES").arg(vector.len());
            for v in vector {
                cmd.arg(v);
            }
            cmd.arg(element);
            pipe.add_command(cmd);
        }

        let results: Vec<u64> = pipe.query_async(&mut conn).await.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("nil") || msg.contains("EXECABORT") {
                CacheError::TransactionAborted
            } else {
                Self::classify_redis_error(e)
            }
        })?;

        // Detect WATCH abort: EXEC returns nil when the watched key was modified
        // by another client, which redis-rs maps to an empty result vec.
        if results.len() != items.len() {
            return Err(CacheError::TransactionAborted);
        }

        let added: u64 = results.iter().sum();

        Ok(VectorAddResult {
            key: key.to_string(),
            added_count: added,
        })
    }

    #[instrument(skip(self), fields(key = %key))]
    async fn vrem(&self, key: &str, items: Vec<String>) -> Result<u64, CacheError> {
        let mut conn = self.get_connection().await?;

        // Native VREM commands — no Lua required. VREM on a non-existent
        // element returns 0 (never errors), so there is no partial-failure risk.
        // VREM is also idempotent, making partial completion safe to retry.
        let mut removed: u64 = 0;
        for item in &items {
            let result: u64 = redis::cmd("VREM")
                .arg(key)
                .arg(item)
                .query_async(&mut conn)
                .await
                .map_err(Self::classify_redis_error)?;
            removed += result;
        }

        Ok(removed)
    }

    #[instrument(skip(self, vector), fields(key = %key, k = k))]
    async fn vsim(
        &self,
        key: &str,
        vector: Vec<f32>,
        k: u64,
    ) -> Result<VectorSimResult, CacheError> {
        let mut conn = self.get_connection().await?;

        // VSIM key VALUES <dim> val [val ...] COUNT k WITHSCORES
        let mut cmd = redis::cmd("VSIM");
        cmd.arg(key).arg("VALUES").arg(vector.len());
        for v in &vector {
            cmd.arg(v);
        }
        cmd.arg("COUNT").arg(k).arg("WITHSCORES");

        let result: Vec<redis::Value> = cmd
            .query_async(&mut conn)
            .await
            .map_err(Self::classify_redis_error)?;

        // Parse alternating element/score pairs
        let mut items = Vec::new();
        let mut iter = result.into_iter();
        while let Some(id_val) = iter.next() {
            let id = match id_val {
                redis::Value::BulkString(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                redis::Value::SimpleString(s) => s,
                _ => continue,
            };
            let score = iter.next().and_then(|v| match v {
                redis::Value::BulkString(bytes) => {
                    String::from_utf8_lossy(&bytes).parse::<f64>().ok()
                }
                redis::Value::SimpleString(s) => s.parse::<f64>().ok(),
                redis::Value::Double(f) => Some(f),
                _ => None,
            });
            items.push(VectorItem {
                id,
                score,
                vector: None,
                attributes: None,
            });
        }

        Ok(VectorSimResult { items })
    }

    #[instrument(skip(self), fields(key = %key))]
    async fn vcard(&self, key: &str) -> Result<u64, CacheError> {
        let mut conn = self.get_connection().await?;
        let count: u64 = redis::cmd("VCARD")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(Self::classify_redis_error)?;
        Ok(count)
    }

    #[instrument(skip(self), fields(key = %key))]
    async fn vdim(&self, key: &str) -> Result<u64, CacheError> {
        let mut conn = self.get_connection().await?;
        let dim: u64 = redis::cmd("VDIM")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(Self::classify_redis_error)?;
        Ok(dim)
    }

    #[instrument(skip(self), fields(key = %key))]
    async fn vemb(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<Vec<Option<Vec<f32>>>, CacheError> {
        let mut conn = self.get_connection().await?;
        let mut results = Vec::with_capacity(items.len());

        // VEMB retrieves one element at a time: VEMB key element
        for item in &items {
            let result: redis::Value = redis::cmd("VEMB")
                .arg(key)
                .arg(item)
                .query_async(&mut conn)
                .await
                .map_err(Self::classify_redis_error)?;

            match result {
                redis::Value::Nil => results.push(None),
                redis::Value::Array(arr) => {
                    let floats: Vec<f32> = arr
                        .into_iter()
                        .filter_map(|v| match v {
                            redis::Value::Double(f) => Some(f as f32),
                            redis::Value::BulkString(bytes) => {
                                String::from_utf8_lossy(&bytes).parse::<f32>().ok()
                            }
                            _ => None,
                        })
                        .collect();
                    results.push(Some(floats));
                }
                _ => results.push(None),
            }
        }
        Ok(results)
    }

    #[instrument(skip(self), fields(key = %key))]
    async fn vismember(&self, key: &str, items: Vec<String>) -> Result<Vec<bool>, CacheError> {
        let mut conn = self.get_connection().await?;
        let mut results = Vec::with_capacity(items.len());

        // VISMEMBER checks one element at a time: VISMEMBER key element
        for item in &items {
            let result: u64 = redis::cmd("VISMEMBER")
                .arg(key)
                .arg(item)
                .query_async(&mut conn)
                .await
                .map_err(Self::classify_redis_error)?;
            results.push(result == 1);
        }
        Ok(results)
    }

    #[instrument(skip(self), fields(key = %key, item = %item))]
    async fn vlinks(&self, key: &str, item: &str) -> Result<Vec<Vec<String>>, CacheError> {
        let mut conn = self.get_connection().await?;
        let result: redis::Value = redis::cmd("VLINKS")
            .arg(key)
            .arg(item)
            .query_async(&mut conn)
            .await
            .map_err(Self::classify_redis_error)?;

        // Nil reply means the key or element does not exist
        if matches!(result, redis::Value::Nil) {
            return Err(CacheError::KeyNotFound(format!(
                "Vector set '{}' or element '{}' not found",
                key, item
            )));
        }

        // VLINKS returns nested arrays: one array per HNSW layer
        let mut layers = Vec::new();
        if let redis::Value::Array(level_arrays) = result {
            for level in level_arrays {
                let mut layer_neighbors = Vec::new();
                if let redis::Value::Array(items) = level {
                    for item in items {
                        match item {
                            redis::Value::BulkString(bytes) => {
                                layer_neighbors.push(String::from_utf8_lossy(&bytes).to_string());
                            }
                            redis::Value::SimpleString(s) => layer_neighbors.push(s),
                            _ => {}
                        }
                    }
                }
                layers.push(layer_neighbors);
            }
        }
        Ok(layers)
    }

    #[instrument(skip(self), fields(key = %key, count = count))]
    async fn vrandmember(&self, key: &str, count: i64) -> Result<Vec<String>, CacheError> {
        let mut conn = self.get_connection().await?;
        let members: Vec<String> = redis::cmd("VRANDMEMBER")
            .arg(key)
            .arg(count)
            .query_async(&mut conn)
            .await
            .map_err(Self::classify_redis_error)?;
        Ok(members)
    }

    #[instrument(skip(self), fields(key = %key))]
    async fn vrange(
        &self,
        key: &str,
        start: &str,
        end: &str,
        count: Option<i64>,
    ) -> Result<VectorRangeResult, CacheError> {
        let mut conn = self.get_connection().await?;
        // VRANGE key start end [count] — count is a positional 4th argument
        let mut cmd = redis::cmd("VRANGE");
        cmd.arg(key).arg(start).arg(end);
        if let Some(c) = count {
            cmd.arg(c);
        }
        let result: Vec<String> = cmd
            .query_async(&mut conn)
            .await
            .map_err(Self::classify_redis_error)?;

        let items = result
            .into_iter()
            .map(|id| VectorItem {
                id,
                score: None,
                vector: None,
                attributes: None,
            })
            .collect();

        Ok(VectorRangeResult { items })
    }

    #[instrument(skip(self), fields(key = %key))]
    async fn vinfo(&self, key: &str) -> Result<VectorInfo, CacheError> {
        let mut conn = self.get_connection().await?;
        let raw: redis::Value = redis::cmd("VINFO")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(Self::classify_redis_error)?;

        // Nil reply means the key does not exist
        let result = match raw {
            redis::Value::Nil => {
                return Err(CacheError::KeyNotFound(format!(
                    "Vector set '{}' not found",
                    key
                )));
            }
            redis::Value::Array(arr) => arr,
            _ => {
                return Err(CacheError::Internal(
                    "VINFO: unexpected response type".to_string(),
                ));
            }
        };

        // VINFO returns key-value pairs as a flat array.
        // We require dimension and count to be present; missing fields cause an error
        // rather than silently returning fabricated defaults.
        let mut dimension: Option<u64> = None;
        let mut distance_metric: Option<String> = None;
        let mut data_type: Option<String> = None;
        let mut count: Option<u64> = None;

        let mut iter = result.iter();
        while let Some(key_val) = iter.next() {
            let key_str = match key_val {
                redis::Value::BulkString(bytes) => String::from_utf8_lossy(bytes).to_string(),
                redis::Value::SimpleString(s) => s.clone(),
                _ => continue,
            };
            if let Some(val) = iter.next() {
                match key_str.as_str() {
                    "quant-type" | "data-type" => {
                        data_type = Self::extract_string_value(val);
                    }
                    "vector-dim" | "dim" => {
                        dimension = Self::extract_u64_value(val);
                    }
                    "size" | "elements" => {
                        count = Self::extract_u64_value(val);
                    }
                    "distance-metric" | "similarity-function" => {
                        distance_metric = Self::extract_string_value(val);
                    }
                    _ => {}
                }
            }
        }

        let dimension = dimension.ok_or_else(|| {
            CacheError::Internal("VINFO: missing required field 'vector-dim'".to_string())
        })?;
        let count = count.ok_or_else(|| {
            CacheError::Internal("VINFO: missing required field 'size'".to_string())
        })?;

        Ok(VectorInfo {
            dimension,
            distance_metric: distance_metric.unwrap_or_default(),
            data_type: data_type.unwrap_or_default(),
            count,
        })
    }

    #[instrument(skip(self), fields(key = %key, item = %item))]
    async fn vgetattr(&self, key: &str, item: &str) -> Result<Option<String>, CacheError> {
        let mut conn = self.get_connection().await?;
        // VGETATTR returns nil both for "no attributes" and "element unknown".
        // This matches Redis semantics directly — callers see attributes: null
        // in both cases, consistent with the response schema.
        let attrs: Option<String> = redis::cmd("VGETATTR")
            .arg(key)
            .arg(item)
            .query_async(&mut conn)
            .await
            .map_err(Self::classify_redis_error)?;
        Ok(attrs)
    }

    #[instrument(skip(self, attributes), fields(key = %key, item = %item))]
    async fn vsetattr(&self, key: &str, item: &str, attributes: &str) -> Result<bool, CacheError> {
        let mut conn = self.get_connection().await?;
        let result: redis::Value = redis::cmd("VSETATTR")
            .arg(key)
            .arg(item)
            .arg(attributes)
            .query_async(&mut conn)
            .await
            .map_err(Self::classify_redis_error)?;
        match result {
            redis::Value::Int(1) => Ok(true),
            redis::Value::Okay => Ok(true),
            // Redis returns 0 when the element does not exist in the set
            redis::Value::Int(0) => Err(CacheError::KeyNotFound(format!(
                "Element '{}' not found in vector set '{}'",
                item, key
            ))),
            _ => Ok(false),
        }
    }
}
