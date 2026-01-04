//! Bloom Service
//!
//! Application layer for Bloom filter and Cuckoo filter operations.

use std::sync::Arc;

use crate::domain::entities::{
    BloomAddResult, BloomCardResult, BloomExistsResult, BloomInfo, BloomInsertOptions,
    BloomInsertResult, BloomLoadChunkResult, BloomReserveOptions, BloomReserveResult,
    BloomScanDumpResult, CuckooAddResult, CuckooCountResult, CuckooDelResult, CuckooExistsResult,
    CuckooInfo, CuckooInsertOptions, CuckooInsertResult, CuckooLoadChunkResult,
    CuckooReserveOptions, CuckooReserveResult, CuckooScanDumpResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::BloomRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisBloomRepository;

/// Service for Bloom filter and Cuckoo filter operations
pub struct BloomService {
    repository: Arc<dyn BloomRepository>,
}

impl BloomService {
    /// Create a new BloomService with a Redis connection pool
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self {
            repository: Arc::new(RedisBloomRepository::new(pool)),
        }
    }

    /// Create a new BloomService with a custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn BloomRepository>) -> Self {
        Self { repository }
    }

    // ==================== Bloom Filter Operations ====================

    /// Create a new Bloom filter (BF.RESERVE)
    pub async fn bf_reserve(&self, key: &str, options: BloomReserveOptions) -> Result<BloomReserveResult, CacheError> {
        self.validate_key(key)?;
        self.validate_bloom_reserve_options(&options)?;
        self.repository.bf_reserve(key, options).await
    }

    /// Add an item to a Bloom filter (BF.ADD)
    pub async fn bf_add(&self, key: &str, item: &str) -> Result<BloomAddResult, CacheError> {
        self.validate_key(key)?;
        self.validate_item(item)?;
        self.repository.bf_add(key, item).await
    }

    /// Add multiple items to a Bloom filter (BF.MADD)
    pub async fn bf_madd(&self, key: &str, items: Vec<String>) -> Result<BloomAddResult, CacheError> {
        self.validate_key(key)?;
        self.validate_items(&items)?;
        self.repository.bf_madd(key, items).await
    }

    /// Check if an item exists in a Bloom filter (BF.EXISTS)
    pub async fn bf_exists(&self, key: &str, item: &str) -> Result<BloomExistsResult, CacheError> {
        self.validate_key(key)?;
        self.validate_item(item)?;
        self.repository.bf_exists(key, item).await
    }

    /// Check if multiple items exist in a Bloom filter (BF.MEXISTS)
    pub async fn bf_mexists(&self, key: &str, items: Vec<String>) -> Result<BloomExistsResult, CacheError> {
        self.validate_key(key)?;
        self.validate_items(&items)?;
        self.repository.bf_mexists(key, items).await
    }

    /// Insert items with options (BF.INSERT)
    pub async fn bf_insert(&self, key: &str, options: BloomInsertOptions, items: Vec<String>) -> Result<BloomInsertResult, CacheError> {
        self.validate_key(key)?;
        self.validate_items(&items)?;
        self.validate_bloom_insert_options(&options)?;
        self.repository.bf_insert(key, options, items).await
    }

    /// Get information about a Bloom filter (BF.INFO)
    pub async fn bf_info(&self, key: &str) -> Result<BloomInfo, CacheError> {
        self.validate_key(key)?;
        self.repository.bf_info(key).await
    }

    /// Get estimated cardinality of a Bloom filter (BF.CARD)
    pub async fn bf_card(&self, key: &str) -> Result<BloomCardResult, CacheError> {
        self.validate_key(key)?;
        self.repository.bf_card(key).await
    }

    /// Begin incremental save of a Bloom filter (BF.SCANDUMP)
    pub async fn bf_scandump(&self, key: &str, iterator: u64) -> Result<BloomScanDumpResult, CacheError> {
        self.validate_key(key)?;
        self.repository.bf_scandump(key, iterator).await
    }

    /// Restore a Bloom filter from a dump (BF.LOADCHUNK)
    pub async fn bf_loadchunk(&self, key: &str, iterator: u64, data: &[u8]) -> Result<BloomLoadChunkResult, CacheError> {
        self.validate_key(key)?;
        self.repository.bf_loadchunk(key, iterator, data).await
    }

    // ==================== Cuckoo Filter Operations ====================

    /// Create a new Cuckoo filter (CF.RESERVE)
    pub async fn cf_reserve(&self, key: &str, options: CuckooReserveOptions) -> Result<CuckooReserveResult, CacheError> {
        self.validate_key(key)?;
        self.validate_cuckoo_reserve_options(&options)?;
        self.repository.cf_reserve(key, options).await
    }

    /// Add an item to a Cuckoo filter (CF.ADD)
    pub async fn cf_add(&self, key: &str, item: &str) -> Result<CuckooAddResult, CacheError> {
        self.validate_key(key)?;
        self.validate_item(item)?;
        self.repository.cf_add(key, item).await
    }

    /// Add an item only if it doesn't exist (CF.ADDNX)
    pub async fn cf_addnx(&self, key: &str, item: &str) -> Result<CuckooAddResult, CacheError> {
        self.validate_key(key)?;
        self.validate_item(item)?;
        self.repository.cf_addnx(key, item).await
    }

    /// Insert items with options (CF.INSERT)
    pub async fn cf_insert(&self, key: &str, options: CuckooInsertOptions, items: Vec<String>) -> Result<CuckooInsertResult, CacheError> {
        self.validate_key(key)?;
        self.validate_items(&items)?;
        self.validate_cuckoo_insert_options(&options)?;
        self.repository.cf_insert(key, options, items).await
    }

    /// Insert items only if they don't exist (CF.INSERTNX)
    pub async fn cf_insertnx(&self, key: &str, options: CuckooInsertOptions, items: Vec<String>) -> Result<CuckooInsertResult, CacheError> {
        self.validate_key(key)?;
        self.validate_items(&items)?;
        self.validate_cuckoo_insert_options(&options)?;
        self.repository.cf_insertnx(key, options, items).await
    }

    /// Check if an item exists (CF.EXISTS)
    pub async fn cf_exists(&self, key: &str, item: &str) -> Result<CuckooExistsResult, CacheError> {
        self.validate_key(key)?;
        self.validate_item(item)?;
        self.repository.cf_exists(key, item).await
    }

    /// Check if multiple items exist (CF.MEXISTS)
    pub async fn cf_mexists(&self, key: &str, items: Vec<String>) -> Result<CuckooExistsResult, CacheError> {
        self.validate_key(key)?;
        self.validate_items(&items)?;
        self.repository.cf_mexists(key, items).await
    }

    /// Delete an item from a Cuckoo filter (CF.DEL)
    pub async fn cf_del(&self, key: &str, item: &str) -> Result<CuckooDelResult, CacheError> {
        self.validate_key(key)?;
        self.validate_item(item)?;
        self.repository.cf_del(key, item).await
    }

    /// Count occurrences of an item (CF.COUNT)
    pub async fn cf_count(&self, key: &str, item: &str) -> Result<CuckooCountResult, CacheError> {
        self.validate_key(key)?;
        self.validate_item(item)?;
        self.repository.cf_count(key, item).await
    }

    /// Get information about a Cuckoo filter (CF.INFO)
    pub async fn cf_info(&self, key: &str) -> Result<CuckooInfo, CacheError> {
        self.validate_key(key)?;
        self.repository.cf_info(key).await
    }

    /// Begin incremental save of a Cuckoo filter (CF.SCANDUMP)
    pub async fn cf_scandump(&self, key: &str, iterator: u64) -> Result<CuckooScanDumpResult, CacheError> {
        self.validate_key(key)?;
        self.repository.cf_scandump(key, iterator).await
    }

    /// Restore a Cuckoo filter from a dump (CF.LOADCHUNK)
    pub async fn cf_loadchunk(&self, key: &str, iterator: u64, data: &[u8]) -> Result<CuckooLoadChunkResult, CacheError> {
        self.validate_key(key)?;
        self.repository.cf_loadchunk(key, iterator, data).await
    }

    // ==================== Validation Helpers ====================

    fn validate_key(&self, key: &str) -> Result<(), CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if key.len() > 512 * 1024 * 1024 {
            return Err(CacheError::InvalidInput("Key too long (max 512MB)".to_string()));
        }
        Ok(())
    }

    fn validate_item(&self, item: &str) -> Result<(), CacheError> {
        if item.is_empty() {
            return Err(CacheError::InvalidInput("Item cannot be empty".to_string()));
        }
        Ok(())
    }

    fn validate_items(&self, items: &[String]) -> Result<(), CacheError> {
        if items.is_empty() {
            return Err(CacheError::InvalidInput("Items list cannot be empty".to_string()));
        }
        for item in items {
            if item.is_empty() {
                return Err(CacheError::InvalidInput("Item cannot be empty".to_string()));
            }
        }
        Ok(())
    }

    fn validate_bloom_reserve_options(&self, options: &BloomReserveOptions) -> Result<(), CacheError> {
        if options.error_rate <= 0.0 || options.error_rate >= 1.0 {
            return Err(CacheError::InvalidInput("Error rate must be between 0 and 1 (exclusive)".to_string()));
        }
        if options.capacity == 0 {
            return Err(CacheError::InvalidInput("Capacity must be greater than 0".to_string()));
        }
        // NONSCALING and EXPANSION are mutually exclusive
        if options.nonscaling && options.expansion.is_some() {
            return Err(CacheError::InvalidInput("NONSCALING and EXPANSION options are mutually exclusive".to_string()));
        }
        Ok(())
    }

    fn validate_bloom_insert_options(&self, options: &BloomInsertOptions) -> Result<(), CacheError> {
        // Validate capacity if provided
        if let Some(capacity) = options.capacity {
            if capacity == 0 {
                return Err(CacheError::InvalidInput("Capacity must be greater than 0".to_string()));
            }
        }
        // Validate error rate if provided
        if let Some(error_rate) = options.error_rate {
            if error_rate <= 0.0 || error_rate >= 1.0 {
                return Err(CacheError::InvalidInput("Error rate must be between 0 and 1 (exclusive)".to_string()));
            }
        }
        // NONSCALING and EXPANSION are mutually exclusive
        if options.nonscaling && options.expansion.is_some() {
            return Err(CacheError::InvalidInput("NONSCALING and EXPANSION options are mutually exclusive".to_string()));
        }
        Ok(())
    }

    fn validate_cuckoo_reserve_options(&self, options: &CuckooReserveOptions) -> Result<(), CacheError> {
        if options.capacity == 0 {
            return Err(CacheError::InvalidInput("Capacity must be greater than 0".to_string()));
        }
        // Validate optional fields if provided
        if let Some(bucket_size) = options.bucket_size {
            if bucket_size == 0 {
                return Err(CacheError::InvalidInput("Bucket size must be greater than 0".to_string()));
            }
        }
        if let Some(max_iterations) = options.max_iterations {
            if max_iterations == 0 {
                return Err(CacheError::InvalidInput("Max iterations must be greater than 0".to_string()));
            }
        }
        if let Some(expansion) = options.expansion {
            if expansion == 0 {
                return Err(CacheError::InvalidInput("Expansion must be greater than 0".to_string()));
            }
        }
        Ok(())
    }

    fn validate_cuckoo_insert_options(&self, options: &CuckooInsertOptions) -> Result<(), CacheError> {
        // Validate capacity if provided
        if let Some(capacity) = options.capacity {
            if capacity == 0 {
                return Err(CacheError::InvalidInput("Capacity must be greater than 0".to_string()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockBloomRepository {
        bf_reserve_called: Mutex<bool>,
        bf_add_called: Mutex<bool>,
        cf_add_called: Mutex<bool>,
    }

    impl MockBloomRepository {
        fn new() -> Self {
            Self {
                bf_reserve_called: Mutex::new(false),
                bf_add_called: Mutex::new(false),
                cf_add_called: Mutex::new(false),
            }
        }
    }

    #[async_trait]
    impl BloomRepository for MockBloomRepository {
        async fn bf_reserve(&self, key: &str, _options: BloomReserveOptions) -> Result<BloomReserveResult, CacheError> {
            *self.bf_reserve_called.lock().unwrap() = true;
            Ok(BloomReserveResult { key: key.to_string(), success: true })
        }

        async fn bf_add(&self, key: &str, _item: &str) -> Result<BloomAddResult, CacheError> {
            *self.bf_add_called.lock().unwrap() = true;
            Ok(BloomAddResult { key: key.to_string(), results: vec![true] })
        }

        async fn bf_madd(&self, key: &str, items: Vec<String>) -> Result<BloomAddResult, CacheError> {
            Ok(BloomAddResult { key: key.to_string(), results: vec![true; items.len()] })
        }

        async fn bf_exists(&self, key: &str, _item: &str) -> Result<BloomExistsResult, CacheError> {
            Ok(BloomExistsResult { key: key.to_string(), results: vec![true] })
        }

        async fn bf_mexists(&self, key: &str, items: Vec<String>) -> Result<BloomExistsResult, CacheError> {
            Ok(BloomExistsResult { key: key.to_string(), results: vec![true; items.len()] })
        }

        async fn bf_insert(&self, key: &str, _options: BloomInsertOptions, items: Vec<String>) -> Result<BloomInsertResult, CacheError> {
            Ok(BloomInsertResult { key: key.to_string(), results: vec![true; items.len()] })
        }

        async fn bf_info(&self, _key: &str) -> Result<BloomInfo, CacheError> {
            Ok(BloomInfo {
                num_filters: 1,
                num_items_inserted: 100,
                capacity: 1000,
                size: 2048,
                expansion: Some(2),
            })
        }

        async fn bf_card(&self, key: &str) -> Result<BloomCardResult, CacheError> {
            Ok(BloomCardResult { key: key.to_string(), cardinality: 100 })
        }

        async fn bf_scandump(&self, _key: &str, _iterator: u64) -> Result<BloomScanDumpResult, CacheError> {
            Ok(BloomScanDumpResult { iterator: 0, data: None })
        }

        async fn bf_loadchunk(&self, key: &str, _iterator: u64, _data: &[u8]) -> Result<BloomLoadChunkResult, CacheError> {
            Ok(BloomLoadChunkResult { key: key.to_string(), success: true })
        }

        async fn cf_reserve(&self, key: &str, _options: CuckooReserveOptions) -> Result<CuckooReserveResult, CacheError> {
            Ok(CuckooReserveResult { key: key.to_string(), success: true })
        }

        async fn cf_add(&self, key: &str, _item: &str) -> Result<CuckooAddResult, CacheError> {
            *self.cf_add_called.lock().unwrap() = true;
            Ok(CuckooAddResult { key: key.to_string(), added: true })
        }

        async fn cf_addnx(&self, key: &str, _item: &str) -> Result<CuckooAddResult, CacheError> {
            Ok(CuckooAddResult { key: key.to_string(), added: true })
        }

        async fn cf_insert(&self, key: &str, _options: CuckooInsertOptions, items: Vec<String>) -> Result<CuckooInsertResult, CacheError> {
            Ok(CuckooInsertResult { key: key.to_string(), results: vec![true; items.len()] })
        }

        async fn cf_insertnx(&self, key: &str, _options: CuckooInsertOptions, items: Vec<String>) -> Result<CuckooInsertResult, CacheError> {
            Ok(CuckooInsertResult { key: key.to_string(), results: vec![true; items.len()] })
        }

        async fn cf_exists(&self, key: &str, _item: &str) -> Result<CuckooExistsResult, CacheError> {
            Ok(CuckooExistsResult { key: key.to_string(), results: vec![true] })
        }

        async fn cf_mexists(&self, key: &str, items: Vec<String>) -> Result<CuckooExistsResult, CacheError> {
            Ok(CuckooExistsResult { key: key.to_string(), results: vec![true; items.len()] })
        }

        async fn cf_del(&self, key: &str, _item: &str) -> Result<CuckooDelResult, CacheError> {
            Ok(CuckooDelResult { key: key.to_string(), deleted: true })
        }

        async fn cf_count(&self, key: &str, _item: &str) -> Result<CuckooCountResult, CacheError> {
            Ok(CuckooCountResult { key: key.to_string(), count: 1 })
        }

        async fn cf_info(&self, _key: &str) -> Result<CuckooInfo, CacheError> {
            Ok(CuckooInfo {
                size: 4096,
                num_buckets: 512,
                num_filters: 1,
                num_items_inserted: 100,
                num_items_deleted: 5,
                bucket_size: 2,
                expansion_rate: 1,
                max_iterations: 20,
            })
        }

        async fn cf_scandump(&self, _key: &str, _iterator: u64) -> Result<CuckooScanDumpResult, CacheError> {
            Ok(CuckooScanDumpResult { iterator: 0, data: None })
        }

        async fn cf_loadchunk(&self, key: &str, _iterator: u64, _data: &[u8]) -> Result<CuckooLoadChunkResult, CacheError> {
            Ok(CuckooLoadChunkResult { key: key.to_string(), success: true })
        }
    }

    #[tokio::test]
    async fn test_bf_reserve_validation() {
        let service = BloomService::new_with_repository(Arc::new(MockBloomRepository::new()));

        // Empty key should fail
        let result = service.bf_reserve("", BloomReserveOptions {
            error_rate: 0.01,
            capacity: 1000,
            ..Default::default()
        }).await;
        assert!(result.is_err());

        // Invalid error rate should fail
        let result = service.bf_reserve("key", BloomReserveOptions {
            error_rate: 0.0,
            capacity: 1000,
            ..Default::default()
        }).await;
        assert!(result.is_err());

        // Zero capacity should fail
        let result = service.bf_reserve("key", BloomReserveOptions {
            error_rate: 0.01,
            capacity: 0,
            ..Default::default()
        }).await;
        assert!(result.is_err());

        // Valid options should succeed
        let result = service.bf_reserve("key", BloomReserveOptions {
            error_rate: 0.01,
            capacity: 1000,
            ..Default::default()
        }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bf_add() {
        let repo = Arc::new(MockBloomRepository::new());
        let service = BloomService::new_with_repository(repo.clone());

        let result = service.bf_add("myfilter", "item1").await;
        assert!(result.is_ok());
        assert!(*repo.bf_add_called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_cf_reserve_validation() {
        let service = BloomService::new_with_repository(Arc::new(MockBloomRepository::new()));

        // Zero capacity should fail
        let result = service.cf_reserve("key", CuckooReserveOptions {
            capacity: 0,
            ..Default::default()
        }).await;
        assert!(result.is_err());

        // Valid options should succeed
        let result = service.cf_reserve("key", CuckooReserveOptions {
            capacity: 1000,
            ..Default::default()
        }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_items_validation() {
        let service = BloomService::new_with_repository(Arc::new(MockBloomRepository::new()));

        // Empty items list should fail
        let result = service.bf_madd("key", vec![]).await;
        assert!(result.is_err());

        // Empty item in list should fail
        let result = service.bf_madd("key", vec!["".to_string()]).await;
        assert!(result.is_err());

        // Valid items should succeed
        let result = service.bf_madd("key", vec!["item1".to_string(), "item2".to_string()]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bf_reserve_nonscaling_expansion_conflict() {
        let service = BloomService::new_with_repository(Arc::new(MockBloomRepository::new()));

        // NONSCALING + EXPANSION should fail
        let result = service.bf_reserve("key", BloomReserveOptions {
            error_rate: 0.01,
            capacity: 1000,
            nonscaling: true,
            expansion: Some(2),
        }).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_bf_insert_validation() {
        let service = BloomService::new_with_repository(Arc::new(MockBloomRepository::new()));

        // Zero capacity should fail
        let result = service.bf_insert("key", BloomInsertOptions {
            capacity: Some(0),
            ..Default::default()
        }, vec!["item".to_string()]).await;
        assert!(result.is_err());

        // Invalid error rate should fail
        let result = service.bf_insert("key", BloomInsertOptions {
            error_rate: Some(0.0),
            ..Default::default()
        }, vec!["item".to_string()]).await;
        assert!(result.is_err());

        // Error rate >= 1.0 should fail
        let result = service.bf_insert("key", BloomInsertOptions {
            error_rate: Some(1.0),
            ..Default::default()
        }, vec!["item".to_string()]).await;
        assert!(result.is_err());

        // NONSCALING + EXPANSION should fail
        let result = service.bf_insert("key", BloomInsertOptions {
            nonscaling: true,
            expansion: Some(2),
            ..Default::default()
        }, vec!["item".to_string()]).await;
        assert!(result.is_err());

        // Valid options should succeed
        let result = service.bf_insert("key", BloomInsertOptions {
            capacity: Some(1000),
            error_rate: Some(0.01),
            ..Default::default()
        }, vec!["item".to_string()]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cf_reserve_optional_validation() {
        let service = BloomService::new_with_repository(Arc::new(MockBloomRepository::new()));

        // Zero bucket_size should fail
        let result = service.cf_reserve("key", CuckooReserveOptions {
            capacity: 1000,
            bucket_size: Some(0),
            ..Default::default()
        }).await;
        assert!(result.is_err());

        // Zero max_iterations should fail
        let result = service.cf_reserve("key", CuckooReserveOptions {
            capacity: 1000,
            max_iterations: Some(0),
            ..Default::default()
        }).await;
        assert!(result.is_err());

        // Zero expansion should fail
        let result = service.cf_reserve("key", CuckooReserveOptions {
            capacity: 1000,
            expansion: Some(0),
            ..Default::default()
        }).await;
        assert!(result.is_err());

        // Valid options should succeed
        let result = service.cf_reserve("key", CuckooReserveOptions {
            capacity: 1000,
            bucket_size: Some(2),
            max_iterations: Some(20),
            expansion: Some(1),
        }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cf_insert_validation() {
        let service = BloomService::new_with_repository(Arc::new(MockBloomRepository::new()));

        // Zero capacity should fail
        let result = service.cf_insert("key", CuckooInsertOptions {
            capacity: Some(0),
            ..Default::default()
        }, vec!["item".to_string()]).await;
        assert!(result.is_err());

        // Valid options should succeed
        let result = service.cf_insert("key", CuckooInsertOptions {
            capacity: Some(1000),
            ..Default::default()
        }, vec!["item".to_string()]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cf_insertnx_validation() {
        let service = BloomService::new_with_repository(Arc::new(MockBloomRepository::new()));

        // Zero capacity should fail
        let result = service.cf_insertnx("key", CuckooInsertOptions {
            capacity: Some(0),
            ..Default::default()
        }, vec!["item".to_string()]).await;
        assert!(result.is_err());

        // Valid options should succeed
        let result = service.cf_insertnx("key", CuckooInsertOptions {
            capacity: Some(1000),
            ..Default::default()
        }, vec!["item".to_string()]).await;
        assert!(result.is_ok());
    }
}
