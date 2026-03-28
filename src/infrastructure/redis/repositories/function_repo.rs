use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{FunctionFlushMode, FunctionRepository, FunctionRestorePolicy};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::shared::redis_value::{json_to_redis_arg, redis_value_to_json};

#[derive(Clone)]
pub struct RedisFunctionRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisFunctionRepository {
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FunctionRepository for RedisFunctionRepository {
    async fn function_load(&self, code: &str, replace: bool) -> Result<String, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("FUNCTION");
        cmd.arg("LOAD");
        if replace {
            cmd.arg("REPLACE");
        }
        cmd.arg(code);
        let library_name: String = cmd.query_async(&mut conn).await?;
        Ok(library_name)
    }

    async fn function_delete(&self, name: &str) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let _: () = redis::cmd("FUNCTION")
            .arg("DELETE")
            .arg(name)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    async fn function_flush(&self, mode: Option<FunctionFlushMode>) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("FUNCTION");
        cmd.arg("FLUSH");
        if let Some(mode) = mode {
            cmd.arg(match mode {
                FunctionFlushMode::Async => "ASYNC",
                FunctionFlushMode::Sync => "SYNC",
            });
        }
        let _: () = cmd.query_async(&mut conn).await?;
        Ok(())
    }

    async fn function_dump(&self) -> Result<Vec<u8>, CacheError> {
        let mut conn = self.pool.get().await?;
        let payload: Vec<u8> = redis::cmd("FUNCTION")
            .arg("DUMP")
            .query_async(&mut conn)
            .await?;
        Ok(payload)
    }

    async fn function_restore(
        &self,
        payload: &[u8],
        policy: Option<FunctionRestorePolicy>,
    ) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("FUNCTION");
        cmd.arg("RESTORE").arg(payload);
        if let Some(policy) = policy {
            cmd.arg(match policy {
                FunctionRestorePolicy::Append => "APPEND",
                FunctionRestorePolicy::Flush => "FLUSH",
                FunctionRestorePolicy::Replace => "REPLACE",
            });
        }
        let _: () = cmd.query_async(&mut conn).await?;
        Ok(())
    }

    async fn function_list(&self, with_code: bool) -> Result<serde_json::Value, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("FUNCTION");
        cmd.arg("LIST");
        if with_code {
            cmd.arg("WITHCODE");
        }
        let value: redis::Value = cmd.query_async(&mut conn).await?;
        Ok(redis_value_to_json(value))
    }

    async fn function_stats(&self) -> Result<serde_json::Value, CacheError> {
        let mut conn = self.pool.get().await?;
        let value: redis::Value = redis::cmd("FUNCTION")
            .arg("STATS")
            .query_async(&mut conn)
            .await?;
        Ok(redis_value_to_json(value))
    }

    async fn function_kill(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let _: () = redis::cmd("FUNCTION")
            .arg("KILL")
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    async fn fcall(
        &self,
        name: &str,
        keys: &[String],
        args: &[serde_json::Value],
        readonly: bool,
    ) -> Result<serde_json::Value, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd(if readonly { "FCALL_RO" } else { "FCALL" });
        cmd.arg(name).arg(keys.len());
        for key in keys {
            cmd.arg(key);
        }
        for arg in args {
            cmd.arg(json_to_redis_arg(arg));
        }
        let value: redis::Value = cmd.query_async(&mut conn).await?;
        Ok(redis_value_to_json(value))
    }
}
