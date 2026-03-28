//! Redis Set Repository Implementation
//!
//! Concrete implementation of SetRepository using Redis.

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{SetRepository, SetScanResult};
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Redis implementation of SetRepository
#[derive(Clone)]
pub struct RedisSetRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisSetRepository {
    /// Create a new RedisSetRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SetRepository for RedisSetRepository {
    async fn sadd(&self, key: &str, members: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SADD");
        cmd.arg(key);
        for member in members {
            cmd.arg(member);
        }
        let result: i64 = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn srem(&self, key: &str, members: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SREM");
        cmd.arg(key);
        for member in members {
            cmd.arg(member);
        }
        let result: i64 = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn smembers(&self, key: &str) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Vec<String> = redis::cmd("SMEMBERS")
            .arg(key)
            .query_async(&mut conn)
            .await?;
        Ok(result)
    }

    async fn sismember(&self, key: &str, member: &str) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("SISMEMBER")
            .arg(key)
            .arg(member)
            .query_async(&mut conn)
            .await?;
        Ok(result == 1)
    }

    async fn smismember(&self, key: &str, members: &[String]) -> Result<Vec<bool>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SMISMEMBER");
        cmd.arg(key);
        for member in members {
            cmd.arg(member);
        }
        let result: Vec<i64> = cmd.query_async(&mut conn).await?;
        Ok(result.into_iter().map(|v| v == 1).collect())
    }

    async fn scard(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("SCARD").arg(key).query_async(&mut conn).await?;
        Ok(result)
    }

    async fn srandmember(&self, key: &str, count: Option<i64>) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SRANDMEMBER");
        cmd.arg(key);

        if let Some(c) = count {
            cmd.arg(c);
            let result: Vec<String> = cmd.query_async(&mut conn).await.unwrap_or_default();
            Ok(result)
        } else {
            // Without count, SRANDMEMBER returns a single value or nil
            let result: Option<String> = cmd.query_async(&mut conn).await?;
            Ok(result.into_iter().collect())
        }
    }

    async fn spop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SPOP");
        cmd.arg(key);

        if let Some(c) = count {
            cmd.arg(c);
            let result: Vec<String> = cmd.query_async(&mut conn).await.unwrap_or_default();
            Ok(result)
        } else {
            // Without count, SPOP returns a single value or nil
            let result: Option<String> = cmd.query_async(&mut conn).await?;
            Ok(result.into_iter().collect())
        }
    }

    async fn smove(
        &self,
        source: &str,
        destination: &str,
        member: &str,
    ) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("SMOVE")
            .arg(source)
            .arg(destination)
            .arg(member)
            .query_async(&mut conn)
            .await?;
        Ok(result == 1)
    }

    async fn sinter(&self, keys: &[String]) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SINTER");
        for key in keys {
            cmd.arg(key);
        }
        let result: Vec<String> = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn sinterstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SINTERSTORE");
        cmd.arg(destination);
        for key in keys {
            cmd.arg(key);
        }
        let result: i64 = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn sintercard(&self, keys: &[String], limit: Option<u64>) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SINTERCARD");
        cmd.arg(keys.len());
        for key in keys {
            cmd.arg(key);
        }
        if let Some(l) = limit {
            cmd.arg("LIMIT").arg(l);
        }
        let result: i64 = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn sunion(&self, keys: &[String]) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SUNION");
        for key in keys {
            cmd.arg(key);
        }
        let result: Vec<String> = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn sunionstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SUNIONSTORE");
        cmd.arg(destination);
        for key in keys {
            cmd.arg(key);
        }
        let result: i64 = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn sdiff(&self, keys: &[String]) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SDIFF");
        for key in keys {
            cmd.arg(key);
        }
        let result: Vec<String> = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn sdiffstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SDIFFSTORE");
        cmd.arg(destination);
        for key in keys {
            cmd.arg(key);
        }
        let result: i64 = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn sscan(
        &self,
        key: &str,
        cursor: u64,
        pattern: Option<&str>,
        count: Option<u64>,
    ) -> Result<SetScanResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SSCAN");
        cmd.arg(key).arg(cursor);

        if let Some(p) = pattern {
            cmd.arg("MATCH").arg(p);
        }
        if let Some(c) = count {
            cmd.arg("COUNT").arg(c);
        }

        let (next_cursor, members): (u64, Vec<String>) = cmd.query_async(&mut conn).await?;
        Ok(SetScanResult {
            cursor: next_cursor,
            members,
        })
    }
}
