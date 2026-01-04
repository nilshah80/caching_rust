//! Bloom Repository Trait
//!
//! Defines the interface for Bloom filter and Cuckoo filter operations.

use async_trait::async_trait;

use crate::domain::entities::{
    BloomAddResult, BloomCardResult, BloomExistsResult, BloomInfo, BloomInsertOptions,
    BloomInsertResult, BloomLoadChunkResult, BloomReserveOptions, BloomReserveResult,
    BloomScanDumpResult, CuckooAddResult, CuckooCountResult, CuckooDelResult, CuckooExistsResult,
    CuckooInfo, CuckooInsertOptions, CuckooInsertResult, CuckooLoadChunkResult,
    CuckooReserveOptions, CuckooReserveResult, CuckooScanDumpResult,
};
use crate::domain::errors::CacheError;

/// Repository trait for Bloom and Cuckoo filter operations
#[async_trait]
pub trait BloomRepository: Send + Sync {
    // ==================== Bloom Filter Operations ====================

    /// Create a new Bloom filter with specified options (BF.RESERVE)
    async fn bf_reserve(&self, key: &str, options: BloomReserveOptions) -> Result<BloomReserveResult, CacheError>;

    /// Add an item to a Bloom filter (BF.ADD)
    async fn bf_add(&self, key: &str, item: &str) -> Result<BloomAddResult, CacheError>;

    /// Add multiple items to a Bloom filter (BF.MADD)
    async fn bf_madd(&self, key: &str, items: Vec<String>) -> Result<BloomAddResult, CacheError>;

    /// Check if an item exists in a Bloom filter (BF.EXISTS)
    async fn bf_exists(&self, key: &str, item: &str) -> Result<BloomExistsResult, CacheError>;

    /// Check if multiple items exist in a Bloom filter (BF.MEXISTS)
    async fn bf_mexists(&self, key: &str, items: Vec<String>) -> Result<BloomExistsResult, CacheError>;

    /// Insert items with options, auto-creating filter if needed (BF.INSERT)
    async fn bf_insert(&self, key: &str, options: BloomInsertOptions, items: Vec<String>) -> Result<BloomInsertResult, CacheError>;

    /// Get information about a Bloom filter (BF.INFO)
    async fn bf_info(&self, key: &str) -> Result<BloomInfo, CacheError>;

    /// Get estimated cardinality of a Bloom filter (BF.CARD)
    async fn bf_card(&self, key: &str) -> Result<BloomCardResult, CacheError>;

    /// Begin incremental save of a Bloom filter (BF.SCANDUMP)
    async fn bf_scandump(&self, key: &str, iterator: u64) -> Result<BloomScanDumpResult, CacheError>;

    /// Restore a Bloom filter from a dump (BF.LOADCHUNK)
    async fn bf_loadchunk(&self, key: &str, iterator: u64, data: &[u8]) -> Result<BloomLoadChunkResult, CacheError>;

    // ==================== Cuckoo Filter Operations ====================

    /// Create a new Cuckoo filter with specified options (CF.RESERVE)
    async fn cf_reserve(&self, key: &str, options: CuckooReserveOptions) -> Result<CuckooReserveResult, CacheError>;

    /// Add an item to a Cuckoo filter (CF.ADD)
    async fn cf_add(&self, key: &str, item: &str) -> Result<CuckooAddResult, CacheError>;

    /// Add an item only if it doesn't exist (CF.ADDNX)
    async fn cf_addnx(&self, key: &str, item: &str) -> Result<CuckooAddResult, CacheError>;

    /// Insert items with options (CF.INSERT)
    async fn cf_insert(&self, key: &str, options: CuckooInsertOptions, items: Vec<String>) -> Result<CuckooInsertResult, CacheError>;

    /// Insert items only if they don't exist (CF.INSERTNX)
    async fn cf_insertnx(&self, key: &str, options: CuckooInsertOptions, items: Vec<String>) -> Result<CuckooInsertResult, CacheError>;

    /// Check if an item exists in a Cuckoo filter (CF.EXISTS)
    async fn cf_exists(&self, key: &str, item: &str) -> Result<CuckooExistsResult, CacheError>;

    /// Check if multiple items exist in a Cuckoo filter (CF.MEXISTS)
    async fn cf_mexists(&self, key: &str, items: Vec<String>) -> Result<CuckooExistsResult, CacheError>;

    /// Delete an item from a Cuckoo filter (CF.DEL)
    async fn cf_del(&self, key: &str, item: &str) -> Result<CuckooDelResult, CacheError>;

    /// Count occurrences of an item in a Cuckoo filter (CF.COUNT)
    async fn cf_count(&self, key: &str, item: &str) -> Result<CuckooCountResult, CacheError>;

    /// Get information about a Cuckoo filter (CF.INFO)
    async fn cf_info(&self, key: &str) -> Result<CuckooInfo, CacheError>;

    /// Begin incremental save of a Cuckoo filter (CF.SCANDUMP)
    async fn cf_scandump(&self, key: &str, iterator: u64) -> Result<CuckooScanDumpResult, CacheError>;

    /// Restore a Cuckoo filter from a dump (CF.LOADCHUNK)
    async fn cf_loadchunk(&self, key: &str, iterator: u64, data: &[u8]) -> Result<CuckooLoadChunkResult, CacheError>;
}
