//! String Repository Trait
//!
//! Abstract interface for string operations.

use async_trait::async_trait;
use std::time::Duration;

use crate::domain::entities::{
    AppendResult, GetExOptions, MGetResult, RangeResult, SetOptions, SetRangeResult, SetResult,
    StringValue,
};
use crate::domain::errors::CacheError;

/// Repository trait for Redis string operations
#[async_trait]
pub trait StringRepository: Send + Sync {
    /// GET - Get the value of a key
    async fn get(&self, key: &str) -> Result<Option<StringValue>, CacheError>;

    /// SET - Set the value of a key with options
    async fn set(
        &self,
        key: &str,
        value: &str,
        options: SetOptions,
    ) -> Result<SetResult, CacheError>;

    /// SETNX - Set key only if it does not exist
    async fn set_nx(
        &self,
        key: &str,
        value: &str,
        ttl: Option<Duration>,
    ) -> Result<bool, CacheError>;

    /// SETEX - Set key with expiration in seconds
    async fn set_ex(&self, key: &str, value: &str, ttl: Duration) -> Result<(), CacheError>;

    /// MGET - Get values of multiple keys
    async fn mget(&self, keys: &[String]) -> Result<MGetResult, CacheError>;

    /// MSET - Set multiple key-value pairs
    async fn mset(&self, pairs: &[(String, String)]) -> Result<(), CacheError>;

    /// MSETNX - Set multiple keys only if none exist
    async fn mset_nx(&self, pairs: &[(String, String)]) -> Result<bool, CacheError>;

    /// INCR - Increment integer value by 1
    async fn incr(&self, key: &str) -> Result<i64, CacheError>;

    /// INCRBY - Increment integer value by amount
    async fn incr_by(&self, key: &str, delta: i64) -> Result<i64, CacheError>;

    /// INCRBYFLOAT - Increment float value by amount
    async fn incr_by_float(&self, key: &str, delta: f64) -> Result<f64, CacheError>;

    /// DECR - Decrement integer value by 1
    async fn decr(&self, key: &str) -> Result<i64, CacheError>;

    /// DECRBY - Decrement integer value by amount
    async fn decr_by(&self, key: &str, delta: i64) -> Result<i64, CacheError>;

    /// APPEND - Append value to existing string
    async fn append(&self, key: &str, value: &str) -> Result<AppendResult, CacheError>;

    /// STRLEN - Get length of string value
    async fn str_len(&self, key: &str) -> Result<i64, CacheError>;

    /// GETRANGE - Get substring of string value
    async fn get_range(&self, key: &str, start: i64, end: i64) -> Result<RangeResult, CacheError>;

    /// SETRANGE - Overwrite part of string at offset
    async fn set_range(
        &self,
        key: &str,
        offset: i64,
        value: &str,
    ) -> Result<SetRangeResult, CacheError>;

    /// GETEX - Get value and optionally set expiration
    async fn get_ex(&self, key: &str, options: GetExOptions) -> Result<Option<String>, CacheError>;

    /// GETDEL - Get value and delete key
    async fn get_del(&self, key: &str) -> Result<Option<String>, CacheError>;
}
