//! Functions repository trait.

use async_trait::async_trait;

use crate::domain::errors::CacheError;

/// Flush mode for `FUNCTION FLUSH`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionFlushMode {
    Async,
    Sync,
}

/// Restore policy for `FUNCTION RESTORE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionRestorePolicy {
    Append,
    Flush,
    Replace,
}

/// Repository trait for Redis Functions operations.
#[async_trait]
pub trait FunctionRepository: Send + Sync {
    async fn function_load(&self, code: &str, replace: bool) -> Result<String, CacheError>;
    async fn function_delete(&self, name: &str) -> Result<(), CacheError>;
    async fn function_flush(&self, mode: Option<FunctionFlushMode>) -> Result<(), CacheError>;
    async fn function_dump(&self) -> Result<Vec<u8>, CacheError>;
    async fn function_restore(
        &self,
        payload: &[u8],
        policy: Option<FunctionRestorePolicy>,
    ) -> Result<(), CacheError>;
    async fn function_list(&self, with_code: bool) -> Result<serde_json::Value, CacheError>;
    async fn function_stats(&self) -> Result<serde_json::Value, CacheError>;
    async fn function_kill(&self) -> Result<(), CacheError>;
    async fn fcall(
        &self,
        name: &str,
        keys: &[String],
        args: &[serde_json::Value],
        readonly: bool,
    ) -> Result<serde_json::Value, CacheError>;
}
