//! String Repository Trait
//!
//! Abstract interface for string operations.

use async_trait::async_trait;
use serde::Serialize;
use std::time::Duration;

use crate::domain::entities::{
    AppendResult, ExpiryMode, GetExOptions, MGetResult, RangeResult, SetOptions, SetRangeResult,
    SetResult, StringValue,
};
use crate::domain::errors::CacheError;

/// Options for the LCS (Longest Common Subsequence) command (Redis 7.0+)
#[derive(Debug, Clone, Default)]
pub struct LcsOptions {
    /// Return just the length instead of the string
    pub len: bool,
    /// Return match positions
    pub idx: bool,
    /// Minimum match length (used with IDX)
    pub min_match_len: Option<u64>,
    /// Include match lengths in IDX output
    pub with_match_len: bool,
}

/// Result of the LCS command
#[derive(Debug, Clone, Serialize)]
pub enum LcsResult {
    /// The LCS string itself
    String(String),
    /// Just the length of the LCS
    Length(i64),
    /// Match positions with metadata
    Matches(LcsMatchResult),
}

/// Match result containing positions and total length
#[derive(Debug, Clone, Serialize)]
pub struct LcsMatchResult {
    /// List of match positions
    pub matches: Vec<LcsMatch>,
    /// Total length of the LCS
    pub len: i64,
}

/// A single match in the LCS result
#[derive(Debug, Clone, Serialize)]
pub struct LcsMatch {
    /// Range in key1 (start, end)
    pub key1_range: (i64, i64),
    /// Range in key2 (start, end)
    pub key2_range: (i64, i64),
    /// Length of this match (only if WITHMATCHLEN)
    pub match_len: Option<i64>,
}

/// Existence condition for the MSETEX command (Redis 8.4+)
#[derive(Debug, Clone, Copy)]
pub enum MSetExExistence {
    /// Set only if none of the keys exist (NX)
    Nx,
    /// Set only if all of the keys exist (XX)
    Xx,
}

/// Options for the MSETEX command (Redis 8.4+)
#[derive(Debug, Clone, Default)]
pub struct MSetExOptions {
    /// NX/XX condition (mutually exclusive)
    pub existence: Option<MSetExExistence>,
    /// Expiry mode (EX, PX, EXAT, PXAT). Mutually exclusive with `keep_ttl`.
    pub expiry_mode: Option<ExpiryMode>,
    /// Expiry value paired with `expiry_mode`.
    pub expiry_value: Option<u64>,
    /// Retain existing TTL on each key (KEEPTTL)
    pub keep_ttl: bool,
}

/// Conditional predicate for the DELEX command (Redis 8.4+).
///
/// At most one variant may be set per request — Redis itself rejects multiple
/// conditions. Service-layer validation enforces this before dispatch.
#[derive(Debug, Clone)]
pub enum DelExCondition {
    /// IFEQ — delete only when the value equals the supplied string
    IfEq(String),
    /// IFNE — delete only when the value is not equal to the supplied string
    IfNe(String),
    /// IFDEQ — delete only when the value's XXH3 digest matches
    IfDeq(String),
    /// IFDNE — delete only when the value's XXH3 digest does not match
    IfDne(String),
}

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

    /// LCS - Longest Common Subsequence (Redis 7.0+)
    async fn lcs(
        &self,
        key1: &str,
        key2: &str,
        options: LcsOptions,
    ) -> Result<LcsResult, CacheError>;

    /// MSETEX - Atomic multi-key SET with shared TTL (Redis 8.4+).
    ///
    /// Returns `true` if all keys were set, `false` when an NX/XX precondition
    /// caused Redis to skip the entire batch.
    async fn msetex(
        &self,
        pairs: &[(String, String)],
        options: MSetExOptions,
    ) -> Result<bool, CacheError>;

    /// DELEX - Conditional delete by value or digest (Redis 8.4+).
    ///
    /// Returns `true` when the key was deleted, `false` when the key did not
    /// exist or the supplied condition (if any) was not satisfied.
    async fn delex(&self, key: &str, condition: Option<DelExCondition>)
    -> Result<bool, CacheError>;

    /// DIGEST - Compute the XXH3 hash digest of a string value (Redis 8.4+).
    ///
    /// Returns `None` when the key does not exist. Errors out on non-string keys.
    async fn digest(&self, key: &str) -> Result<Option<String>, CacheError>;
}
