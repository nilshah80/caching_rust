//! Redis String Repository Implementation
//!
//! Concrete implementation of StringRepository using Redis.

use async_trait::async_trait;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::time::Duration;

use crate::domain::entities::{
    AppendResult, GetExOptions, MGetResult,
    RangeResult, SetOptions, SetRangeResult, SetResult, StringValue,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::StringRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Redis implementation of StringRepository
pub struct RedisStringRepository {
    pool: std::sync::Arc<InstrumentedPool>,
}

impl RedisStringRepository {
    /// Create a new RedisStringRepository
    pub fn new(pool: std::sync::Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StringRepository for RedisStringRepository {
    async fn get(&self, key: &str) -> Result<Option<StringValue>, CacheError> {
        let mut conn = self.pool.get().await?;

        // Get value
        let value: Option<String> = conn.get(key).await?;

        match value {
            Some(v) => {
                // Get additional metadata in parallel
                let (data_type, ttl, encoding): (String, i64, Option<String>) = redis::pipe()
                    .cmd("TYPE").arg(key)
                    .cmd("TTL").arg(key)
                    .cmd("OBJECT").arg("ENCODING").arg(key)
                    .query_async(&mut conn)
                    .await
                    .unwrap_or(("string".to_string(), -1, None));

                Ok(Some(StringValue {
                    key: key.to_string(),
                    length: v.len(),
                    value: v,
                    data_type,
                    ttl: if ttl == -1 { None } else { Some(ttl) },
                    encoding,
                }))
            }
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, value: &str, options: SetOptions) -> Result<SetResult, CacheError> {
        let mut conn = self.pool.get().await?;

        // Build SET command with options
        let mut cmd = redis::cmd("SET");
        cmd.arg(key).arg(value);

        if options.nx {
            cmd.arg("NX");
        }
        if options.xx {
            cmd.arg("XX");
        }
        if options.get {
            cmd.arg("GET");
        }
        if let (Some(mode), Some(val)) = (options.expiry_mode, options.expiry_value) {
            cmd.arg(mode.as_str()).arg(val);
        }
        if options.keep_ttl {
            cmd.arg("KEEPTTL");
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;

        // Parse result
        let (success, previous_value) = match result {
            redis::Value::Okay => (true, None),
            redis::Value::Nil => (false, None),
            redis::Value::BulkString(bytes) => {
                let prev = String::from_utf8_lossy(&bytes).to_string();
                (true, Some(prev))
            }
            redis::Value::SimpleString(s) => {
                if s == "OK" {
                    (true, None)
                } else {
                    (true, Some(s))
                }
            }
            _ => (true, None),
        };

        Ok(SetResult {
            key: key.to_string(),
            success,
            previous_value,
        })
    }

    async fn set_nx(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: bool = if let Some(ttl) = ttl {
            // Use SET with NX and EX
            let result: redis::Value = redis::cmd("SET")
                .arg(key)
                .arg(value)
                .arg("NX")
                .arg("EX")
                .arg(ttl.as_secs())
                .query_async(&mut conn)
                .await?;

            !matches!(result, redis::Value::Nil)
        } else {
            conn.set_nx(key, value).await?
        };

        Ok(result)
    }

    async fn set_ex(&self, key: &str, value: &str, ttl: Duration) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let _: () = conn.set_ex(key, value, ttl.as_secs()).await?;
        Ok(())
    }

    async fn mget(&self, keys: &[String]) -> Result<MGetResult, CacheError> {
        if keys.is_empty() {
            return Ok(MGetResult {
                found: HashMap::new(),
                missing: vec![],
            });
        }

        let mut conn = self.pool.get().await?;
        let values: Vec<Option<String>> = conn.mget(keys).await?;

        let mut found = HashMap::new();
        let mut missing = Vec::new();

        for (key, value) in keys.iter().zip(values.iter()) {
            match value {
                Some(v) => {
                    found.insert(key.clone(), v.clone());
                }
                None => {
                    missing.push(key.clone());
                }
            }
        }

        Ok(MGetResult { found, missing })
    }

    async fn mset(&self, pairs: &[(String, String)]) -> Result<(), CacheError> {
        if pairs.is_empty() {
            return Ok(());
        }

        let mut conn = self.pool.get().await?;

        let flat: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let _: () = conn.mset(&flat).await?;
        Ok(())
    }

    async fn mset_nx(&self, pairs: &[(String, String)]) -> Result<bool, CacheError> {
        if pairs.is_empty() {
            return Ok(true);
        }

        let mut conn = self.pool.get().await?;

        let flat: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let result: bool = conn.mset_nx(&flat).await?;
        Ok(result)
    }

    async fn incr(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = conn.incr(key, 1).await?;
        Ok(result)
    }

    async fn incr_by(&self, key: &str, delta: i64) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = conn.incr(key, delta).await?;
        Ok(result)
    }

    async fn incr_by_float(&self, key: &str, delta: f64) -> Result<f64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: f64 = conn.incr(key, delta).await?;
        Ok(result)
    }

    async fn decr(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = conn.decr(key, 1).await?;
        Ok(result)
    }

    async fn decr_by(&self, key: &str, delta: i64) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = conn.decr(key, delta).await?;
        Ok(result)
    }

    async fn append(&self, key: &str, value: &str) -> Result<AppendResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let new_length: i64 = conn.append(key, value).await?;

        Ok(AppendResult {
            key: key.to_string(),
            new_length,
        })
    }

    async fn str_len(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let length: i64 = conn.strlen(key).await?;
        Ok(length)
    }

    async fn get_range(&self, key: &str, start: i64, end: i64) -> Result<RangeResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let value: String = conn.getrange(key, start as isize, end as isize).await?;

        Ok(RangeResult {
            key: key.to_string(),
            value,
            start,
            end,
        })
    }

    async fn set_range(&self, key: &str, offset: i64, value: &str) -> Result<SetRangeResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let new_length: i64 = conn.setrange(key, offset as isize, value).await?;

        Ok(SetRangeResult {
            key: key.to_string(),
            new_length,
        })
    }

    async fn get_ex(&self, key: &str, options: GetExOptions) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("GETEX");
        cmd.arg(key);

        if options.persist {
            cmd.arg("PERSIST");
        } else if let (Some(mode), Some(val)) = (options.expiry_mode, options.expiry_value) {
            cmd.arg(mode.as_str()).arg(val);
        }

        let result: Option<String> = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn get_del(&self, key: &str) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Option<String> = redis::cmd("GETDEL")
            .arg(key)
            .query_async(&mut conn)
            .await?;
        Ok(result)
    }
}
