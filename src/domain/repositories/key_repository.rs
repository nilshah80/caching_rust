//! Key Repository Trait
//!
//! Abstract interface for key management operations.

use async_trait::async_trait;

use crate::domain::entities::{
    CopyOptions, CopyResult, DeleteResult, DumpResult, ExistsResult, ExpireOptions, ExpireResult,
    KeyInfo, PersistResult, RandomKeyResult, RenameResult, ScanResult, TouchResult,
};
use crate::domain::errors::CacheError;

/// Options for SORT command
#[derive(Debug, Clone, Default)]
pub struct SortOptions {
    pub by: Option<String>,
    pub get: Vec<String>,
    pub limit: Option<(i64, i64)>,
    pub order: SortOrder,
    pub alpha: bool,
}

/// Sort order for SORT command
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// Repository trait for Redis key operations
#[async_trait]
pub trait KeyRepository: Send + Sync {
    /// DEL - Delete one or more keys
    async fn delete(&self, keys: &[String]) -> Result<DeleteResult, CacheError>;

    /// EXISTS - Check if one or more keys exist
    async fn exists(&self, keys: &[String]) -> Result<ExistsResult, CacheError>;

    /// EXPIRE - Set expiration in seconds
    async fn expire(
        &self,
        key: &str,
        seconds: i64,
        options: ExpireOptions,
    ) -> Result<ExpireResult, CacheError>;

    /// EXPIREAT - Set expiration as Unix timestamp (seconds)
    async fn expire_at(
        &self,
        key: &str,
        timestamp: i64,
        options: ExpireOptions,
    ) -> Result<ExpireResult, CacheError>;

    /// PEXPIRE - Set expiration in milliseconds
    async fn pexpire(
        &self,
        key: &str,
        milliseconds: i64,
        options: ExpireOptions,
    ) -> Result<ExpireResult, CacheError>;

    /// PEXPIREAT - Set expiration as Unix timestamp (milliseconds)
    async fn pexpire_at(
        &self,
        key: &str,
        timestamp: i64,
        options: ExpireOptions,
    ) -> Result<ExpireResult, CacheError>;

    /// TTL - Get time-to-live in seconds
    async fn ttl(&self, key: &str) -> Result<i64, CacheError>;

    /// PTTL - Get time-to-live in milliseconds
    async fn pttl(&self, key: &str) -> Result<i64, CacheError>;

    /// PERSIST - Remove expiration from a key
    async fn persist(&self, key: &str) -> Result<PersistResult, CacheError>;

    /// TYPE - Get the type of a key
    async fn key_type(&self, key: &str) -> Result<String, CacheError>;

    /// RENAME - Rename a key
    async fn rename(&self, key: &str, new_key: &str) -> Result<RenameResult, CacheError>;

    /// RENAMENX - Rename a key only if new key doesn't exist
    async fn rename_nx(&self, key: &str, new_key: &str) -> Result<RenameResult, CacheError>;

    /// COPY - Copy a key to a new key
    async fn copy(
        &self,
        source: &str,
        destination: &str,
        options: CopyOptions,
    ) -> Result<CopyResult, CacheError>;

    /// SCAN - Incrementally iterate keys
    async fn scan(
        &self,
        cursor: u64,
        pattern: Option<&str>,
        count: Option<u64>,
        key_type: Option<&str>,
    ) -> Result<ScanResult, CacheError>;

    /// KEYS - Find all keys matching a pattern (use with caution in production)
    async fn keys(&self, pattern: &str) -> Result<Vec<String>, CacheError>;

    /// RANDOMKEY - Return a random key
    async fn random_key(&self) -> Result<RandomKeyResult, CacheError>;

    /// TOUCH - Alters the last access time of keys
    async fn touch(&self, keys: &[String]) -> Result<TouchResult, CacheError>;

    /// UNLINK - Delete keys asynchronously
    async fn unlink(&self, keys: &[String]) -> Result<DeleteResult, CacheError>;

    /// DUMP - Serialize a key's value
    async fn dump(&self, key: &str) -> Result<DumpResult, CacheError>;

    /// RESTORE - Deserialize a value into a key
    async fn restore(
        &self,
        key: &str,
        ttl: i64,
        data: &[u8],
        replace: bool,
    ) -> Result<bool, CacheError>;

    /// OBJECT ENCODING - Get the encoding of a key
    async fn object_encoding(&self, key: &str) -> Result<Option<String>, CacheError>;

    /// OBJECT IDLETIME - Get idle time of a key
    async fn object_idletime(&self, key: &str) -> Result<Option<u64>, CacheError>;

    /// OBJECT REFCOUNT - Get reference count of a key
    async fn object_refcount(&self, key: &str) -> Result<Option<u64>, CacheError>;

    /// OBJECT FREQ - Get frequency counter of a key (LFU)
    async fn object_freq(&self, key: &str) -> Result<Option<u64>, CacheError>;

    /// Get comprehensive key info (combines TYPE, TTL, PTTL, OBJECT commands)
    async fn key_info(&self, key: &str) -> Result<KeyInfo, CacheError>;

    /// EXPIRETIME - Get absolute Unix timestamp when key will expire (Redis 7.0+)
    async fn expire_time(&self, key: &str) -> Result<i64, CacheError>;

    /// PEXPIRETIME - Get absolute Unix timestamp in milliseconds when key will expire (Redis 7.0+)
    async fn pexpire_time(&self, key: &str) -> Result<i64, CacheError>;

    /// SORT - Sort the elements in a list, set or sorted set
    async fn sort(
        &self,
        key: &str,
        options: SortOptions,
    ) -> Result<Vec<Option<String>>, CacheError>;

    /// SORT...STORE - Sort and store the result in a destination key
    async fn sort_store(
        &self,
        key: &str,
        destination: &str,
        options: SortOptions,
    ) -> Result<i64, CacheError>;

    /// SORT_RO - Read-only variant of SORT (Redis 7.0+)
    async fn sort_ro(
        &self,
        key: &str,
        options: SortOptions,
    ) -> Result<Vec<Option<String>>, CacheError>;
}
