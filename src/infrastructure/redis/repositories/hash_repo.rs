use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::errors::CacheError;
use crate::domain::repositories::HashRepository;
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
}
