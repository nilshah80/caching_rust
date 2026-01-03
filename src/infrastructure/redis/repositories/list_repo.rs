//! Redis List Repository Implementation
//!
//! Concrete implementation of ListRepository using Redis.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    BlockingPopResult, InsertPosition, ListDirection, ListRepository, LPosOptions,
};
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Redis implementation of ListRepository
#[derive(Clone)]
pub struct RedisListRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisListRepository {
    /// Create a new RedisListRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ListRepository for RedisListRepository {
    async fn lpush(&self, key: &str, values: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("LPUSH");
        cmd.arg(key);
        for value in values {
            cmd.arg(value);
        }
        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn rpush(&self, key: &str, values: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("RPUSH");
        cmd.arg(key);
        for value in values {
            cmd.arg(value);
        }
        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn lpush_x(&self, key: &str, values: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("LPUSHX");
        cmd.arg(key);
        for value in values {
            cmd.arg(value);
        }
        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn rpush_x(&self, key: &str, values: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("RPUSHX");
        cmd.arg(key);
        for value in values {
            cmd.arg(value);
        }
        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn lpop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("LPOP");
        cmd.arg(key);

        if let Some(c) = count {
            cmd.arg(c);
            let result: Vec<String> = cmd.query_async(&mut *conn).await.unwrap_or_default();
            Ok(result)
        } else {
            // Without count, LPOP returns a single value or nil
            let result: Option<String> = cmd.query_async(&mut *conn).await?;
            Ok(result.into_iter().collect())
        }
    }

    async fn rpop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("RPOP");
        cmd.arg(key);

        if let Some(c) = count {
            cmd.arg(c);
            let result: Vec<String> = cmd.query_async(&mut *conn).await.unwrap_or_default();
            Ok(result)
        } else {
            // Without count, RPOP returns a single value or nil
            let result: Option<String> = cmd.query_async(&mut *conn).await?;
            Ok(result.into_iter().collect())
        }
    }

    async fn lrange(&self, key: &str, start: i64, stop: i64) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Vec<String> = redis::cmd("LRANGE")
            .arg(key)
            .arg(start)
            .arg(stop)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn llen(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("LLEN")
            .arg(key)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn lindex(&self, key: &str, index: i64) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Option<String> = redis::cmd("LINDEX")
            .arg(key)
            .arg(index)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn lset(&self, key: &str, index: i64, value: &str) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let _: () = redis::cmd("LSET")
            .arg(key)
            .arg(index)
            .arg(value)
            .query_async(&mut *conn)
            .await?;
        Ok(())
    }

    async fn linsert(
        &self,
        key: &str,
        position: InsertPosition,
        pivot: &str,
        value: &str,
    ) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("LINSERT")
            .arg(key)
            .arg(position.as_str())
            .arg(pivot)
            .arg(value)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn lrem(&self, key: &str, count: i64, value: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("LREM")
            .arg(key)
            .arg(count)
            .arg(value)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn ltrim(&self, key: &str, start: i64, stop: i64) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let _: () = redis::cmd("LTRIM")
            .arg(key)
            .arg(start)
            .arg(stop)
            .query_async(&mut *conn)
            .await?;
        Ok(())
    }

    async fn lpos(
        &self,
        key: &str,
        element: &str,
        options: LPosOptions,
    ) -> Result<Vec<i64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("LPOS");
        cmd.arg(key).arg(element);

        if let Some(rank) = options.rank {
            cmd.arg("RANK").arg(rank);
        }
        if let Some(count) = options.count {
            cmd.arg("COUNT").arg(count);
        }
        if let Some(max_len) = options.max_len {
            cmd.arg("MAXLEN").arg(max_len);
        }

        // If COUNT is specified, LPOS returns an array; otherwise it returns a single value or nil
        if options.count.is_some() {
            let result: Vec<i64> = cmd.query_async(&mut *conn).await.unwrap_or_default();
            Ok(result)
        } else {
            let result: Option<i64> = cmd.query_async(&mut *conn).await?;
            Ok(result.into_iter().collect())
        }
    }

    async fn lmove(
        &self,
        source: &str,
        destination: &str,
        src_dir: ListDirection,
        dst_dir: ListDirection,
    ) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Option<String> = redis::cmd("LMOVE")
            .arg(source)
            .arg(destination)
            .arg(src_dir.as_str())
            .arg(dst_dir.as_str())
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn rpop_lpush(&self, source: &str, destination: &str) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Option<String> = redis::cmd("RPOPLPUSH")
            .arg(source)
            .arg(destination)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn blpop(
        &self,
        keys: &[String],
        timeout: Duration,
    ) -> Result<Option<BlockingPopResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("BLPOP");
        for key in keys {
            cmd.arg(key);
        }
        cmd.arg(timeout.as_secs_f64());

        let result: Option<(String, String)> = cmd.query_async(&mut *conn).await?;
        Ok(result.map(|(key, value)| BlockingPopResult { key, value }))
    }

    async fn brpop(
        &self,
        keys: &[String],
        timeout: Duration,
    ) -> Result<Option<BlockingPopResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("BRPOP");
        for key in keys {
            cmd.arg(key);
        }
        cmd.arg(timeout.as_secs_f64());

        let result: Option<(String, String)> = cmd.query_async(&mut *conn).await?;
        Ok(result.map(|(key, value)| BlockingPopResult { key, value }))
    }

    async fn blmove(
        &self,
        source: &str,
        destination: &str,
        src_dir: ListDirection,
        dst_dir: ListDirection,
        timeout: Duration,
    ) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Option<String> = redis::cmd("BLMOVE")
            .arg(source)
            .arg(destination)
            .arg(src_dir.as_str())
            .arg(dst_dir.as_str())
            .arg(timeout.as_secs_f64())
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn brpop_lpush(
        &self,
        source: &str,
        destination: &str,
        timeout: Duration,
    ) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Option<String> = redis::cmd("BRPOPLPUSH")
            .arg(source)
            .arg(destination)
            .arg(timeout.as_secs_f64())
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }
}
