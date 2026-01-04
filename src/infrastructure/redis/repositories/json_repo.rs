//! Redis JSON Repository Implementation
//!
//! Concrete implementation of JsonRepository using RedisJSON module.

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::domain::entities::{
    JsonArrAppendResult, JsonArrIndexResult, JsonArrInsertResult, JsonArrLenResult,
    JsonArrPopResult, JsonArrTrimResult, JsonClearResult, JsonDebugMemoryResult, JsonDelResult,
    JsonGetResult, JsonMGetItem, JsonMGetResult, JsonMSetItem, JsonNumResult, JsonObjKeysResult,
    JsonObjLenResult, JsonRespResult, JsonSetOptions, JsonSetResult, JsonStrAppendResult,
    JsonStrLenResult, JsonToggleResult, JsonTypeResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::JsonRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Redis implementation of JsonRepository using RedisJSON module
pub struct RedisJsonRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisJsonRepository {
    /// Create a new RedisJsonRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }

    /// Parse redis Value into serde_json Value
    fn parse_json_value(value: redis::Value) -> Option<Value> {
        match value {
            redis::Value::Nil => None,
            redis::Value::BulkString(bytes) => {
                let s = String::from_utf8_lossy(&bytes);
                serde_json::from_str(&s).ok()
            }
            redis::Value::SimpleString(s) => serde_json::from_str(&s).ok(),
            redis::Value::Array(arr) => {
                let parsed: Vec<Value> = arr
                    .into_iter()
                    .filter_map(Self::parse_json_value)
                    .collect();
                if parsed.is_empty() {
                    None
                } else {
                    Some(Value::Array(parsed))
                }
            }
            redis::Value::Int(i) => Some(Value::Number(i.into())),
            _ => None,
        }
    }

    /// Parse optional integer results from Redis array
    fn parse_optional_i64_array(value: redis::Value) -> Vec<Option<i64>> {
        match value {
            redis::Value::Array(arr) => arr
                .into_iter()
                .map(|v| match v {
                    redis::Value::Int(i) => Some(i),
                    redis::Value::Nil => None,
                    _ => None,
                })
                .collect(),
            redis::Value::Int(i) => vec![Some(i)],
            redis::Value::Nil => vec![None],
            _ => vec![],
        }
    }

    /// Parse optional string results from Redis array
    fn parse_optional_string_array(value: redis::Value) -> Vec<Option<String>> {
        match value {
            redis::Value::Array(arr) => arr
                .into_iter()
                .map(|v| match v {
                    redis::Value::BulkString(bytes) => {
                        Some(String::from_utf8_lossy(&bytes).to_string())
                    }
                    redis::Value::SimpleString(s) => Some(s),
                    redis::Value::Nil => None,
                    _ => None,
                })
                .collect(),
            redis::Value::BulkString(bytes) => {
                vec![Some(String::from_utf8_lossy(&bytes).to_string())]
            }
            redis::Value::SimpleString(s) => vec![Some(s)],
            redis::Value::Nil => vec![None],
            _ => vec![],
        }
    }

    /// Parse optional boolean results from Redis array (0/1 to bool)
    fn parse_optional_bool_array(value: redis::Value) -> Vec<Option<bool>> {
        match value {
            redis::Value::Array(arr) => arr
                .into_iter()
                .map(|v| match v {
                    redis::Value::Int(i) => Some(i != 0),
                    redis::Value::Nil => None,
                    _ => None,
                })
                .collect(),
            redis::Value::Int(i) => vec![Some(i != 0)],
            redis::Value::Nil => vec![None],
            _ => vec![],
        }
    }

    /// Parse optional JSON value results from Redis array
    fn parse_optional_json_array(value: redis::Value) -> Vec<Option<Value>> {
        match value {
            redis::Value::Array(arr) => arr
                .into_iter()
                .map(|v| Self::parse_json_value(v))
                .collect(),
            _ => vec![Self::parse_json_value(value)],
        }
    }

    /// Parse optional string array results from Redis array (for OBJKEYS)
    fn parse_optional_string_array_array(value: redis::Value) -> Vec<Option<Vec<String>>> {
        match value {
            redis::Value::Array(arr) => arr
                .into_iter()
                .map(|v| match v {
                    redis::Value::Array(inner) => {
                        let strings: Vec<String> = inner
                            .into_iter()
                            .filter_map(|s| match s {
                                redis::Value::BulkString(bytes) => {
                                    Some(String::from_utf8_lossy(&bytes).to_string())
                                }
                                redis::Value::SimpleString(s) => Some(s),
                                _ => None,
                            })
                            .collect();
                        if strings.is_empty() {
                            None
                        } else {
                            Some(strings)
                        }
                    }
                    redis::Value::Nil => None,
                    _ => None,
                })
                .collect(),
            redis::Value::Nil => vec![None],
            _ => vec![],
        }
    }
}

#[async_trait]
impl JsonRepository for RedisJsonRepository {
    // ==================== Core Operations ====================

    async fn json_set(
        &self,
        key: &str,
        path: &str,
        value: Value,
        options: JsonSetOptions,
    ) -> Result<JsonSetResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let json_str = serde_json::to_string(&value)
            .map_err(|e| CacheError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        let mut cmd = redis::cmd("JSON.SET");
        cmd.arg(key).arg(path).arg(&json_str);

        if options.nx {
            cmd.arg("NX");
        }
        if options.xx {
            cmd.arg("XX");
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;

        let success = !matches!(result, redis::Value::Nil);

        Ok(JsonSetResult {
            key: key.to_string(),
            path: path.to_string(),
            success,
        })
    }

    async fn json_get(
        &self,
        key: &str,
        paths: &[String],
    ) -> Result<Option<JsonGetResult>, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("JSON.GET");
        cmd.arg(key);

        // Add paths or default to root
        if paths.is_empty() {
            cmd.arg("$");
        } else {
            for path in paths {
                cmd.arg(path);
            }
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;

        match result {
            redis::Value::Nil => Ok(None),
            _ => {
                let value = Self::parse_json_value(result).unwrap_or(Value::Null);
                Ok(Some(JsonGetResult {
                    key: key.to_string(),
                    paths: if paths.is_empty() {
                        vec!["$".to_string()]
                    } else {
                        paths.to_vec()
                    },
                    value,
                }))
            }
        }
    }

    async fn json_mget(&self, keys: &[String], path: &str) -> Result<JsonMGetResult, CacheError> {
        if keys.is_empty() {
            return Ok(JsonMGetResult {
                results: vec![],
                path: path.to_string(),
            });
        }

        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("JSON.MGET");
        for key in keys {
            cmd.arg(key);
        }
        cmd.arg(path);

        let result: redis::Value = cmd.query_async(&mut conn).await?;

        let values = match result {
            redis::Value::Array(arr) => arr,
            _ => vec![],
        };

        let results: Vec<JsonMGetItem> = keys
            .iter()
            .zip(values.into_iter())
            .map(|(key, v)| JsonMGetItem {
                key: key.clone(),
                value: Self::parse_json_value(v),
            })
            .collect();

        Ok(JsonMGetResult {
            results,
            path: path.to_string(),
        })
    }

    async fn json_mset(&self, items: &[JsonMSetItem]) -> Result<(), CacheError> {
        if items.is_empty() {
            return Ok(());
        }

        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("JSON.MSET");
        for item in items {
            let json_str = serde_json::to_string(&item.value)
                .map_err(|e| CacheError::InvalidInput(format!("Invalid JSON: {}", e)))?;
            cmd.arg(&item.key).arg(&item.path).arg(&json_str);
        }

        let _: () = cmd.query_async(&mut conn).await?;
        Ok(())
    }

    async fn json_del(&self, key: &str, path: &str) -> Result<JsonDelResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let deleted_count: i64 = redis::cmd("JSON.DEL")
            .arg(key)
            .arg(path)
            .query_async(&mut conn)
            .await?;

        Ok(JsonDelResult {
            key: key.to_string(),
            path: path.to_string(),
            deleted_count,
        })
    }

    async fn json_type(&self, key: &str, path: &str) -> Result<JsonTypeResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("JSON.TYPE")
            .arg(key)
            .arg(path)
            .query_async(&mut conn)
            .await?;

        let types = Self::parse_optional_string_array(result);

        Ok(JsonTypeResult {
            key: key.to_string(),
            path: path.to_string(),
            types,
        })
    }

    // ==================== String Operations ====================

    async fn json_str_len(&self, key: &str, path: &str) -> Result<JsonStrLenResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("JSON.STRLEN")
            .arg(key)
            .arg(path)
            .query_async(&mut conn)
            .await?;

        let lengths = Self::parse_optional_i64_array(result);

        Ok(JsonStrLenResult {
            key: key.to_string(),
            path: path.to_string(),
            lengths,
        })
    }

    async fn json_str_append(
        &self,
        key: &str,
        path: &str,
        value: &str,
    ) -> Result<JsonStrAppendResult, CacheError> {
        let mut conn = self.pool.get().await?;

        // JSON.STRAPPEND expects the value as a JSON string (quoted)
        let json_value = serde_json::to_string(value)
            .map_err(|e| CacheError::InvalidInput(format!("Invalid string: {}", e)))?;

        let result: redis::Value = redis::cmd("JSON.STRAPPEND")
            .arg(key)
            .arg(path)
            .arg(&json_value)
            .query_async(&mut conn)
            .await?;

        let new_lengths = Self::parse_optional_i64_array(result);

        Ok(JsonStrAppendResult {
            key: key.to_string(),
            path: path.to_string(),
            new_lengths,
        })
    }

    // ==================== Numeric Operations ====================

    async fn json_num_incr_by(
        &self,
        key: &str,
        path: &str,
        value: f64,
    ) -> Result<JsonNumResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("JSON.NUMINCRBY")
            .arg(key)
            .arg(path)
            .arg(value)
            .query_async(&mut conn)
            .await?;

        let values = Self::parse_json_value(result).unwrap_or(Value::Null);

        Ok(JsonNumResult {
            key: key.to_string(),
            path: path.to_string(),
            values,
        })
    }

    async fn json_num_mult_by(
        &self,
        key: &str,
        path: &str,
        value: f64,
    ) -> Result<JsonNumResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("JSON.NUMMULTBY")
            .arg(key)
            .arg(path)
            .arg(value)
            .query_async(&mut conn)
            .await?;

        let values = Self::parse_json_value(result).unwrap_or(Value::Null);

        Ok(JsonNumResult {
            key: key.to_string(),
            path: path.to_string(),
            values,
        })
    }

    async fn json_toggle(&self, key: &str, path: &str) -> Result<JsonToggleResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("JSON.TOGGLE")
            .arg(key)
            .arg(path)
            .query_async(&mut conn)
            .await?;

        let values = Self::parse_optional_bool_array(result);

        Ok(JsonToggleResult {
            key: key.to_string(),
            path: path.to_string(),
            values,
        })
    }

    async fn json_clear(&self, key: &str, path: &str) -> Result<JsonClearResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let cleared_count: i64 = redis::cmd("JSON.CLEAR")
            .arg(key)
            .arg(path)
            .query_async(&mut conn)
            .await?;

        Ok(JsonClearResult {
            key: key.to_string(),
            path: path.to_string(),
            cleared_count,
        })
    }

    // ==================== Array Operations ====================

    async fn json_arr_len(&self, key: &str, path: &str) -> Result<JsonArrLenResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("JSON.ARRLEN")
            .arg(key)
            .arg(path)
            .query_async(&mut conn)
            .await?;

        let lengths = Self::parse_optional_i64_array(result);

        Ok(JsonArrLenResult {
            key: key.to_string(),
            path: path.to_string(),
            lengths,
        })
    }

    async fn json_arr_append(
        &self,
        key: &str,
        path: &str,
        values: &[Value],
    ) -> Result<JsonArrAppendResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("JSON.ARRAPPEND");
        cmd.arg(key).arg(path);

        for value in values {
            let json_str = serde_json::to_string(value)
                .map_err(|e| CacheError::InvalidInput(format!("Invalid JSON: {}", e)))?;
            cmd.arg(&json_str);
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;
        let new_lengths = Self::parse_optional_i64_array(result);

        Ok(JsonArrAppendResult {
            key: key.to_string(),
            path: path.to_string(),
            new_lengths,
        })
    }

    async fn json_arr_index(
        &self,
        key: &str,
        path: &str,
        value: &Value,
        start: Option<i64>,
        stop: Option<i64>,
    ) -> Result<JsonArrIndexResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let json_str = serde_json::to_string(value)
            .map_err(|e| CacheError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        let mut cmd = redis::cmd("JSON.ARRINDEX");
        cmd.arg(key).arg(path).arg(&json_str);

        if let Some(s) = start {
            cmd.arg(s);
            if let Some(e) = stop {
                cmd.arg(e);
            }
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;
        let indices = Self::parse_optional_i64_array(result);

        Ok(JsonArrIndexResult {
            key: key.to_string(),
            path: path.to_string(),
            indices,
        })
    }

    async fn json_arr_insert(
        &self,
        key: &str,
        path: &str,
        index: i64,
        values: &[Value],
    ) -> Result<JsonArrInsertResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("JSON.ARRINSERT");
        cmd.arg(key).arg(path).arg(index);

        for value in values {
            let json_str = serde_json::to_string(value)
                .map_err(|e| CacheError::InvalidInput(format!("Invalid JSON: {}", e)))?;
            cmd.arg(&json_str);
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;
        let new_lengths = Self::parse_optional_i64_array(result);

        Ok(JsonArrInsertResult {
            key: key.to_string(),
            path: path.to_string(),
            new_lengths,
        })
    }

    async fn json_arr_pop(
        &self,
        key: &str,
        path: &str,
        index: Option<i64>,
    ) -> Result<JsonArrPopResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("JSON.ARRPOP");
        cmd.arg(key).arg(path);

        if let Some(idx) = index {
            cmd.arg(idx);
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;
        let values = Self::parse_optional_json_array(result);

        Ok(JsonArrPopResult {
            key: key.to_string(),
            path: path.to_string(),
            values,
        })
    }

    async fn json_arr_trim(
        &self,
        key: &str,
        path: &str,
        start: i64,
        stop: i64,
    ) -> Result<JsonArrTrimResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("JSON.ARRTRIM")
            .arg(key)
            .arg(path)
            .arg(start)
            .arg(stop)
            .query_async(&mut conn)
            .await?;

        let new_lengths = Self::parse_optional_i64_array(result);

        Ok(JsonArrTrimResult {
            key: key.to_string(),
            path: path.to_string(),
            new_lengths,
        })
    }

    // ==================== Object Operations ====================

    async fn json_obj_len(&self, key: &str, path: &str) -> Result<JsonObjLenResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("JSON.OBJLEN")
            .arg(key)
            .arg(path)
            .query_async(&mut conn)
            .await?;

        let lengths = Self::parse_optional_i64_array(result);

        Ok(JsonObjLenResult {
            key: key.to_string(),
            path: path.to_string(),
            lengths,
        })
    }

    async fn json_obj_keys(&self, key: &str, path: &str) -> Result<JsonObjKeysResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("JSON.OBJKEYS")
            .arg(key)
            .arg(path)
            .query_async(&mut conn)
            .await?;

        let keys = Self::parse_optional_string_array_array(result);

        Ok(JsonObjKeysResult {
            key: key.to_string(),
            path: path.to_string(),
            keys,
        })
    }

    // ==================== Debug Operations ====================

    async fn json_debug_memory(
        &self,
        key: &str,
        path: &str,
    ) -> Result<JsonDebugMemoryResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("JSON.DEBUG")
            .arg("MEMORY")
            .arg(key)
            .arg(path)
            .query_async(&mut conn)
            .await?;

        let memory_bytes = Self::parse_optional_i64_array(result);

        Ok(JsonDebugMemoryResult {
            key: key.to_string(),
            path: path.to_string(),
            memory_bytes,
        })
    }

    async fn json_resp(&self, key: &str, path: &str) -> Result<JsonRespResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("JSON.RESP")
            .arg(key)
            .arg(path)
            .query_async(&mut conn)
            .await?;

        let resp = Self::parse_json_value(result).unwrap_or(Value::Null);

        Ok(JsonRespResult {
            key: key.to_string(),
            path: path.to_string(),
            resp,
        })
    }
}
