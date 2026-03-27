use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{ExpireCondition, HashRepository};
use crate::infrastructure::redis::connection::InstrumentedPool;

#[derive(Clone)]
pub struct RedisHashRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisHashRepository {
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HashRepository for RedisHashRepository {
    async fn hget(&self, key: &str, field: &str) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Option<String> = redis::cmd("HGET")
            .arg(key)
            .arg(field)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn hset(&self, key: &str, pairs: Vec<(String, String)>) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("HSET");
        cmd.arg(key);
        for (field, value) in pairs {
            cmd.arg(field).arg(value);
        }
        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn hset_nx(&self, key: &str, field: &str, value: &str) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: bool = redis::cmd("HSETNX")
            .arg(key)
            .arg(field)
            .arg(value)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn hgetall(&self, key: &str) -> Result<HashMap<String, String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: HashMap<String, String> = redis::cmd("HGETALL")
            .arg(key)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn hmget(&self, key: &str, fields: &[String]) -> Result<Vec<Option<String>>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("HMGET");
        cmd.arg(key);
        for field in fields {
            cmd.arg(field);
        }
        let result: Vec<Option<String>> = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn hmset(&self, key: &str, pairs: Vec<(String, String)>) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("HMSET");
        cmd.arg(key);
        for (field, value) in pairs {
            cmd.arg(field).arg(value);
        }
        let _: () = cmd.query_async(&mut *conn).await?;
        Ok(())
    }

    async fn hdel(&self, key: &str, fields: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("HDEL");
        cmd.arg(key);
        for field in fields {
            cmd.arg(field);
        }
        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn hexists(&self, key: &str, field: &str) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: bool = redis::cmd("HEXISTS")
            .arg(key)
            .arg(field)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn hkeys(&self, key: &str) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Vec<String> = redis::cmd("HKEYS").arg(key).query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn hvals(&self, key: &str) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Vec<String> = redis::cmd("HVALS").arg(key).query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn hlen(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("HLEN").arg(key).query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn hincr_by(&self, key: &str, field: &str, delta: i64) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("HINCRBY")
            .arg(key)
            .arg(field)
            .arg(delta)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn hincr_by_float(&self, key: &str, field: &str, delta: f64) -> Result<f64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: f64 = redis::cmd("HINCRBYFLOAT")
            .arg(key)
            .arg(field)
            .arg(delta)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn hstr_len(&self, key: &str, field: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("HSTRLEN")
            .arg(key)
            .arg(field)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn hrand_field(
        &self,
        key: &str,
        count: Option<i64>,
        with_values: bool,
    ) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("HRANDFIELD");
        cmd.arg(key);
        if let Some(c) = count {
            cmd.arg(c);
            if with_values {
                cmd.arg("WITHVALUES");
            }
            let result: Vec<String> = cmd.query_async(&mut *conn).await?;
            Ok(result)
        } else {
            let result: Option<String> = cmd.query_async(&mut *conn).await?;
            Ok(result.into_iter().collect())
        }
    }

    async fn hscan(
        &self,
        key: &str,
        cursor: u64,
        pattern: Option<String>,
        count: Option<u64>,
    ) -> Result<(u64, Vec<String>), CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("HSCAN");
        cmd.arg(key).arg(cursor);
        if let Some(p) = pattern {
            cmd.arg("MATCH").arg(p);
        }
        if let Some(c) = count {
            cmd.arg("COUNT").arg(c);
        }
        let result: (u64, Vec<String>) = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    // --- Hash field expiration helpers ---

    async fn hexpire(
        &self,
        key: &str,
        seconds: i64,
        fields: &[String],
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        self.run_expire_cmd("HEXPIRE", key, seconds, fields, condition)
            .await
    }

    async fn hpexpire(
        &self,
        key: &str,
        milliseconds: i64,
        fields: &[String],
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        self.run_expire_cmd("HPEXPIRE", key, milliseconds, fields, condition)
            .await
    }

    async fn hexpire_at(
        &self,
        key: &str,
        unix_time: i64,
        fields: &[String],
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        self.run_expire_cmd("HEXPIREAT", key, unix_time, fields, condition)
            .await
    }

    async fn hpexpire_at(
        &self,
        key: &str,
        unix_time_ms: i64,
        fields: &[String],
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        self.run_expire_cmd("HPEXPIREAT", key, unix_time_ms, fields, condition)
            .await
    }

    async fn hexpire_time(&self, key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError> {
        self.run_field_query_cmd("HEXPIRETIME", key, fields).await
    }

    async fn hpexpire_time(&self, key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError> {
        self.run_field_query_cmd("HPEXPIRETIME", key, fields).await
    }

    async fn httl(&self, key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError> {
        self.run_field_query_cmd("HTTL", key, fields).await
    }

    async fn hpttl(&self, key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError> {
        self.run_field_query_cmd("HPTTL", key, fields).await
    }

    async fn hpersist(&self, key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError> {
        self.run_field_query_cmd("HPERSIST", key, fields).await
    }
}

impl RedisHashRepository {
    /// Helper for HEXPIRE, HPEXPIRE, HEXPIREAT, HPEXPIREAT commands.
    async fn run_expire_cmd(
        &self,
        command: &str,
        key: &str,
        time_value: i64,
        fields: &[String],
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd(command);
        cmd.arg(key).arg(time_value);
        if let Some(cond) = &condition {
            cmd.arg(cond.as_str());
        }
        cmd.arg("FIELDS").arg(fields.len());
        for field in fields {
            cmd.arg(field);
        }
        let result: Vec<i64> = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    /// Helper for HEXPIRETIME, HPEXPIRETIME, HTTL, HPTTL, HPERSIST commands.
    async fn run_field_query_cmd(
        &self,
        command: &str,
        key: &str,
        fields: &[String],
    ) -> Result<Vec<i64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd(command);
        cmd.arg(key).arg("FIELDS").arg(fields.len());
        for field in fields {
            cmd.arg(field);
        }
        let result: Vec<i64> = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }
}
