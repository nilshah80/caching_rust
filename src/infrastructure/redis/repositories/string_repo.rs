//! Redis String Repository Implementation
//!
//! Concrete implementation of StringRepository using Redis.

use async_trait::async_trait;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::time::Duration;

use crate::domain::entities::{
    AppendResult, GetExOptions, MGetResult, RangeResult, SetOptions, SetRangeResult, SetResult,
    StringValue,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    DelExCondition, LcsMatch, LcsMatchResult, LcsOptions, LcsResult, MSetExExistence,
    MSetExOptions, StringRepository,
};
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
                    .cmd("TYPE")
                    .arg(key)
                    .cmd("TTL")
                    .arg(key)
                    .cmd("OBJECT")
                    .arg("ENCODING")
                    .arg(key)
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

    async fn set(
        &self,
        key: &str,
        value: &str,
        options: SetOptions,
    ) -> Result<SetResult, CacheError> {
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

    async fn set_nx(
        &self,
        key: &str,
        value: &str,
        ttl: Option<Duration>,
    ) -> Result<bool, CacheError> {
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

    async fn set_range(
        &self,
        key: &str,
        offset: i64,
        value: &str,
    ) -> Result<SetRangeResult, CacheError> {
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
        let result: Option<String> = redis::cmd("GETDEL").arg(key).query_async(&mut conn).await?;
        Ok(result)
    }

    async fn msetex(
        &self,
        pairs: &[(String, String)],
        options: MSetExOptions,
    ) -> Result<bool, CacheError> {
        if pairs.is_empty() {
            return Err(CacheError::InvalidInput(
                "MSETEX requires at least one key-value pair".to_string(),
            ));
        }

        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("MSETEX");
        cmd.arg(pairs.len());
        for (key, value) in pairs {
            cmd.arg(key).arg(value);
        }
        match options.existence {
            Some(MSetExExistence::Nx) => {
                cmd.arg("NX");
            }
            Some(MSetExExistence::Xx) => {
                cmd.arg("XX");
            }
            None => {}
        }
        if options.keep_ttl {
            cmd.arg("KEEPTTL");
        } else if let (Some(mode), Some(val)) = (options.expiry_mode, options.expiry_value) {
            cmd.arg(mode.as_str()).arg(val);
        }

        let applied: i64 = cmd.query_async(&mut conn).await?;
        Ok(applied == 1)
    }

    async fn delex(
        &self,
        key: &str,
        condition: Option<DelExCondition>,
    ) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("DELEX");
        cmd.arg(key);
        match condition {
            Some(DelExCondition::IfEq(v)) => {
                cmd.arg("IFEQ").arg(v);
            }
            Some(DelExCondition::IfNe(v)) => {
                cmd.arg("IFNE").arg(v);
            }
            Some(DelExCondition::IfDeq(d)) => {
                cmd.arg("IFDEQ").arg(d);
            }
            Some(DelExCondition::IfDne(d)) => {
                cmd.arg("IFDNE").arg(d);
            }
            None => {}
        }

        let deleted: i64 = cmd.query_async(&mut conn).await?;
        Ok(deleted == 1)
    }

    async fn digest(&self, key: &str) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Option<String> = redis::cmd("DIGEST").arg(key).query_async(&mut conn).await?;
        Ok(result)
    }

    async fn lcs(
        &self,
        key1: &str,
        key2: &str,
        options: LcsOptions,
    ) -> Result<LcsResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("LCS");
        cmd.arg(key1).arg(key2);

        if options.len {
            cmd.arg("LEN");
        }
        if options.idx {
            cmd.arg("IDX");
        }
        if let Some(min) = options.min_match_len {
            cmd.arg("MINMATCHLEN").arg(min);
        }
        if options.with_match_len {
            cmd.arg("WITHMATCHLEN");
        }

        if options.len {
            let length: i64 = cmd.query_async(&mut conn).await?;
            Ok(LcsResult::Length(length))
        } else if options.idx {
            let value: redis::Value = cmd.query_async(&mut conn).await?;
            parse_lcs_idx_result(value)
        } else {
            let s: String = cmd.query_async(&mut conn).await?;
            Ok(LcsResult::String(s))
        }
    }
}

/// Parse the complex nested Redis value returned by LCS ... IDX
fn parse_lcs_idx_result(value: redis::Value) -> Result<LcsResult, CacheError> {
    // Redis returns: ["matches", [match1, match2, ...], "len", total_len]
    // Each match is: [[key1_start, key1_end], [key2_start, key2_end]] or
    //                [[key1_start, key1_end], [key2_start, key2_end], match_len] (with WITHMATCHLEN)
    match value {
        redis::Value::Array(items) => {
            let mut matches = Vec::new();
            let mut len: i64 = 0;

            let mut i = 0;
            while i < items.len() {
                match &items[i] {
                    redis::Value::BulkString(key) if key == b"matches" => {
                        i += 1;
                        if i < items.len()
                            && let redis::Value::Array(match_list) = &items[i]
                        {
                            for m in match_list {
                                if let redis::Value::Array(parts) = m
                                    && parts.len() >= 2
                                {
                                    let key1_range = parse_range(&parts[0])?;
                                    let key2_range = parse_range(&parts[1])?;
                                    let match_len = if parts.len() >= 3 {
                                        Some(parse_integer(&parts[2])?)
                                    } else {
                                        None
                                    };
                                    matches.push(LcsMatch {
                                        key1_range,
                                        key2_range,
                                        match_len,
                                    });
                                }
                            }
                        }
                    }
                    redis::Value::BulkString(key) if key == b"len" => {
                        i += 1;
                        if i < items.len() {
                            len = parse_integer(&items[i])?;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }

            Ok(LcsResult::Matches(LcsMatchResult { matches, len }))
        }
        _ => Err(CacheError::Internal(
            "Unexpected response format from LCS IDX command".to_string(),
        )),
    }
}

/// Parse a [start, end] range from a Redis Value::Array
fn parse_range(value: &redis::Value) -> Result<(i64, i64), CacheError> {
    match value {
        redis::Value::Array(pair) if pair.len() == 2 => {
            let start = parse_integer(&pair[0])?;
            let end = parse_integer(&pair[1])?;
            Ok((start, end))
        }
        _ => Err(CacheError::Internal(
            "Unexpected range format in LCS IDX response".to_string(),
        )),
    }
}

/// Parse an integer from a Redis Value
fn parse_integer(value: &redis::Value) -> Result<i64, CacheError> {
    match value {
        redis::Value::Int(n) => Ok(*n),
        _ => Err(CacheError::Internal(
            "Expected integer in LCS response".to_string(),
        )),
    }
}
