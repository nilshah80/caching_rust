//! Redis Bloom Repository Implementation
//!
//! Implementation of BloomRepository for Redis using RedisBloom module commands.

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use redis::Value;
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

/// Redis implementation of BloomRepository
pub struct RedisBloomRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisBloomRepository {
    /// Create a new RedisBloomRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }

    /// Extract a boolean from Redis value
    #[allow(dead_code)]
    fn extract_bool(value: &Value) -> bool {
        match value {
            Value::Int(i) => *i != 0,
            Value::Okay => true,
            _ => false,
        }
    }

    /// Extract a u64 from Redis value, handling both Int and BulkString representations
    fn extract_u64(value: &Value) -> u64 {
        match value {
            Value::Int(i) => *i as u64,
            // Handle BulkString that may contain numeric data (some Redis versions return strings)
            Value::BulkString(bytes) => String::from_utf8_lossy(bytes)
                .trim()
                .parse::<u64>()
                .unwrap_or(0),
            // For other types, log and return 0 (graceful degradation)
            _ => 0,
        }
    }

    /// Extract a u32 from Redis value, handling both Int and BulkString representations
    fn extract_u32(value: &Value) -> u32 {
        match value {
            Value::Int(i) => *i as u32,
            // Handle BulkString that may contain numeric data (some Redis versions return strings)
            Value::BulkString(bytes) => String::from_utf8_lossy(bytes)
                .trim()
                .parse::<u32>()
                .unwrap_or(0),
            // For other types, return 0 (graceful degradation)
            _ => 0,
        }
    }

    /// Parse BF.INFO response
    fn parse_bloom_info(value: Value) -> Result<BloomInfo, CacheError> {
        match value {
            Value::Array(arr) => {
                let mut info = BloomInfo {
                    num_filters: 0,
                    num_items_inserted: 0,
                    capacity: 0,
                    size: 0,
                    expansion: None,
                };

                let mut iter = arr.iter();
                while let Some(key) = iter.next() {
                    if let Value::BulkString(k) = key {
                        let key_str = String::from_utf8_lossy(k);
                        if let Some(val) = iter.next() {
                            match key_str.as_ref() {
                                "Capacity" => info.capacity = Self::extract_u64(val),
                                "Size" => info.size = Self::extract_u64(val),
                                "Number of filters" => info.num_filters = Self::extract_u64(val),
                                "Number of items inserted" => {
                                    info.num_items_inserted = Self::extract_u64(val)
                                }
                                "Expansion rate" => info.expansion = Some(Self::extract_u32(val)),
                                _ => {}
                            }
                        }
                    }
                }

                Ok(info)
            }
            _ => Err(CacheError::Internal("Invalid BF.INFO response".to_string())),
        }
    }

    /// Parse CF.INFO response
    fn parse_cuckoo_info(value: Value) -> Result<CuckooInfo, CacheError> {
        match value {
            Value::Array(arr) => {
                let mut info = CuckooInfo {
                    size: 0,
                    num_buckets: 0,
                    num_filters: 0,
                    num_items_inserted: 0,
                    num_items_deleted: 0,
                    bucket_size: 2,
                    expansion_rate: 1,
                    max_iterations: 20,
                };

                let mut iter = arr.iter();
                while let Some(key) = iter.next() {
                    if let Value::BulkString(k) = key {
                        let key_str = String::from_utf8_lossy(k);
                        if let Some(val) = iter.next() {
                            match key_str.as_ref() {
                                "Size" => info.size = Self::extract_u64(val),
                                "Number of buckets" => info.num_buckets = Self::extract_u64(val),
                                "Number of filters" => info.num_filters = Self::extract_u64(val),
                                "Number of items inserted" => {
                                    info.num_items_inserted = Self::extract_u64(val)
                                }
                                "Number of items deleted" => {
                                    info.num_items_deleted = Self::extract_u64(val)
                                }
                                "Bucket size" => info.bucket_size = Self::extract_u32(val),
                                "Expansion rate" => info.expansion_rate = Self::extract_u32(val),
                                "Max iterations" => info.max_iterations = Self::extract_u32(val),
                                _ => {}
                            }
                        }
                    }
                }

                Ok(info)
            }
            _ => Err(CacheError::Internal("Invalid CF.INFO response".to_string())),
        }
    }
}

#[async_trait]
impl BloomRepository for RedisBloomRepository {
    // ==================== Bloom Filter Operations ====================

    async fn bf_reserve(
        &self,
        key: &str,
        options: BloomReserveOptions,
    ) -> Result<BloomReserveResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("BF.RESERVE");
        cmd.arg(key).arg(options.error_rate).arg(options.capacity);

        if let Some(expansion) = options.expansion {
            cmd.arg("EXPANSION").arg(expansion);
        }

        if options.nonscaling {
            cmd.arg("NONSCALING");
        }

        let _: () = cmd.query_async(&mut *conn).await?;

        Ok(BloomReserveResult {
            key: key.to_string(),
            success: true,
        })
    }

    async fn bf_add(&self, key: &str, item: &str) -> Result<BloomAddResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: i64 = redis::cmd("BF.ADD")
            .arg(key)
            .arg(item)
            .query_async(&mut *conn)
            .await?;

        Ok(BloomAddResult {
            key: key.to_string(),
            results: vec![result == 1],
        })
    }

    async fn bf_madd(&self, key: &str, items: Vec<String>) -> Result<BloomAddResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("BF.MADD");
        cmd.arg(key);
        for item in &items {
            cmd.arg(item);
        }

        let result: Vec<i64> = cmd.query_async(&mut *conn).await?;

        Ok(BloomAddResult {
            key: key.to_string(),
            results: result.iter().map(|&r| r == 1).collect(),
        })
    }

    async fn bf_exists(&self, key: &str, item: &str) -> Result<BloomExistsResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: i64 = redis::cmd("BF.EXISTS")
            .arg(key)
            .arg(item)
            .query_async(&mut *conn)
            .await?;

        Ok(BloomExistsResult {
            key: key.to_string(),
            results: vec![result == 1],
        })
    }

    async fn bf_mexists(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<BloomExistsResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("BF.MEXISTS");
        cmd.arg(key);
        for item in &items {
            cmd.arg(item);
        }

        let result: Vec<i64> = cmd.query_async(&mut *conn).await?;

        Ok(BloomExistsResult {
            key: key.to_string(),
            results: result.iter().map(|&r| r == 1).collect(),
        })
    }

    async fn bf_insert(
        &self,
        key: &str,
        options: BloomInsertOptions,
        items: Vec<String>,
    ) -> Result<BloomInsertResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("BF.INSERT");
        cmd.arg(key);

        if let Some(capacity) = options.capacity {
            cmd.arg("CAPACITY").arg(capacity);
        }

        if let Some(error_rate) = options.error_rate {
            cmd.arg("ERROR").arg(error_rate);
        }

        if let Some(expansion) = options.expansion {
            cmd.arg("EXPANSION").arg(expansion);
        }

        if options.nocreate {
            cmd.arg("NOCREATE");
        }

        if options.nonscaling {
            cmd.arg("NONSCALING");
        }

        cmd.arg("ITEMS");
        for item in &items {
            cmd.arg(item);
        }

        let result: Vec<i64> = cmd.query_async(&mut *conn).await?;

        Ok(BloomInsertResult {
            key: key.to_string(),
            results: result.iter().map(|&r| r == 1).collect(),
        })
    }

    async fn bf_info(&self, key: &str) -> Result<BloomInfo, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = redis::cmd("BF.INFO")
            .arg(key)
            .query_async(&mut *conn)
            .await?;

        Self::parse_bloom_info(result)
    }

    async fn bf_card(&self, key: &str) -> Result<BloomCardResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: i64 = redis::cmd("BF.CARD")
            .arg(key)
            .query_async(&mut *conn)
            .await?;

        Ok(BloomCardResult {
            key: key.to_string(),
            cardinality: result as u64,
        })
    }

    async fn bf_scandump(
        &self,
        key: &str,
        iterator: u64,
    ) -> Result<BloomScanDumpResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = redis::cmd("BF.SCANDUMP")
            .arg(key)
            .arg(iterator)
            .query_async(&mut *conn)
            .await?;

        match result {
            Value::Array(arr) if arr.len() >= 2 => {
                let iter = Self::extract_u64(&arr[0]);
                let data = match &arr[1] {
                    Value::BulkString(bytes) => Some(BASE64.encode(bytes)),
                    Value::Nil => None,
                    _ => None,
                };
                Ok(BloomScanDumpResult {
                    iterator: iter,
                    data,
                })
            }
            _ => Err(CacheError::Internal(
                "Invalid BF.SCANDUMP response".to_string(),
            )),
        }
    }

    async fn bf_loadchunk(
        &self,
        key: &str,
        iterator: u64,
        data: &[u8],
    ) -> Result<BloomLoadChunkResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let _: () = redis::cmd("BF.LOADCHUNK")
            .arg(key)
            .arg(iterator)
            .arg(data)
            .query_async(&mut *conn)
            .await?;

        Ok(BloomLoadChunkResult {
            key: key.to_string(),
            success: true,
        })
    }

    // ==================== Cuckoo Filter Operations ====================

    async fn cf_reserve(
        &self,
        key: &str,
        options: CuckooReserveOptions,
    ) -> Result<CuckooReserveResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("CF.RESERVE");
        cmd.arg(key).arg(options.capacity);

        if let Some(bucket_size) = options.bucket_size {
            cmd.arg("BUCKETSIZE").arg(bucket_size);
        }

        if let Some(max_iterations) = options.max_iterations {
            cmd.arg("MAXITERATIONS").arg(max_iterations);
        }

        if let Some(expansion) = options.expansion {
            cmd.arg("EXPANSION").arg(expansion);
        }

        let _: () = cmd.query_async(&mut *conn).await?;

        Ok(CuckooReserveResult {
            key: key.to_string(),
            success: true,
        })
    }

    async fn cf_add(&self, key: &str, item: &str) -> Result<CuckooAddResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: i64 = redis::cmd("CF.ADD")
            .arg(key)
            .arg(item)
            .query_async(&mut *conn)
            .await?;

        Ok(CuckooAddResult {
            key: key.to_string(),
            added: result == 1,
        })
    }

    async fn cf_addnx(&self, key: &str, item: &str) -> Result<CuckooAddResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: i64 = redis::cmd("CF.ADDNX")
            .arg(key)
            .arg(item)
            .query_async(&mut *conn)
            .await?;

        Ok(CuckooAddResult {
            key: key.to_string(),
            added: result == 1,
        })
    }

    async fn cf_insert(
        &self,
        key: &str,
        options: CuckooInsertOptions,
        items: Vec<String>,
    ) -> Result<CuckooInsertResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("CF.INSERT");
        cmd.arg(key);

        if let Some(capacity) = options.capacity {
            cmd.arg("CAPACITY").arg(capacity);
        }

        if options.nocreate {
            cmd.arg("NOCREATE");
        }

        cmd.arg("ITEMS");
        for item in &items {
            cmd.arg(item);
        }

        let result: Vec<i64> = cmd.query_async(&mut *conn).await?;

        Ok(CuckooInsertResult {
            key: key.to_string(),
            results: result.iter().map(|&r| r == 1).collect(),
        })
    }

    async fn cf_insertnx(
        &self,
        key: &str,
        options: CuckooInsertOptions,
        items: Vec<String>,
    ) -> Result<CuckooInsertResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("CF.INSERTNX");
        cmd.arg(key);

        if let Some(capacity) = options.capacity {
            cmd.arg("CAPACITY").arg(capacity);
        }

        if options.nocreate {
            cmd.arg("NOCREATE");
        }

        cmd.arg("ITEMS");
        for item in &items {
            cmd.arg(item);
        }

        let result: Vec<i64> = cmd.query_async(&mut *conn).await?;

        Ok(CuckooInsertResult {
            key: key.to_string(),
            results: result.iter().map(|&r| r == 1).collect(),
        })
    }

    async fn cf_exists(&self, key: &str, item: &str) -> Result<CuckooExistsResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: i64 = redis::cmd("CF.EXISTS")
            .arg(key)
            .arg(item)
            .query_async(&mut *conn)
            .await?;

        Ok(CuckooExistsResult {
            key: key.to_string(),
            results: vec![result == 1],
        })
    }

    async fn cf_mexists(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<CuckooExistsResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("CF.MEXISTS");
        cmd.arg(key);
        for item in &items {
            cmd.arg(item);
        }

        let result: Vec<i64> = cmd.query_async(&mut *conn).await?;

        Ok(CuckooExistsResult {
            key: key.to_string(),
            results: result.iter().map(|&r| r == 1).collect(),
        })
    }

    async fn cf_del(&self, key: &str, item: &str) -> Result<CuckooDelResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: i64 = redis::cmd("CF.DEL")
            .arg(key)
            .arg(item)
            .query_async(&mut *conn)
            .await?;

        Ok(CuckooDelResult {
            key: key.to_string(),
            deleted: result == 1,
        })
    }

    async fn cf_count(&self, key: &str, item: &str) -> Result<CuckooCountResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: i64 = redis::cmd("CF.COUNT")
            .arg(key)
            .arg(item)
            .query_async(&mut *conn)
            .await?;

        Ok(CuckooCountResult {
            key: key.to_string(),
            count: result as u64,
        })
    }

    async fn cf_info(&self, key: &str) -> Result<CuckooInfo, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = redis::cmd("CF.INFO")
            .arg(key)
            .query_async(&mut *conn)
            .await?;

        Self::parse_cuckoo_info(result)
    }

    async fn cf_scandump(
        &self,
        key: &str,
        iterator: u64,
    ) -> Result<CuckooScanDumpResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Value = redis::cmd("CF.SCANDUMP")
            .arg(key)
            .arg(iterator)
            .query_async(&mut *conn)
            .await?;

        match result {
            Value::Array(arr) if arr.len() >= 2 => {
                let iter = Self::extract_u64(&arr[0]);
                let data = match &arr[1] {
                    Value::BulkString(bytes) => Some(BASE64.encode(bytes)),
                    Value::Nil => None,
                    _ => None,
                };
                Ok(CuckooScanDumpResult {
                    iterator: iter,
                    data,
                })
            }
            _ => Err(CacheError::Internal(
                "Invalid CF.SCANDUMP response".to_string(),
            )),
        }
    }

    async fn cf_loadchunk(
        &self,
        key: &str,
        iterator: u64,
        data: &[u8],
    ) -> Result<CuckooLoadChunkResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let _: () = redis::cmd("CF.LOADCHUNK")
            .arg(key)
            .arg(iterator)
            .arg(data)
            .query_async(&mut *conn)
            .await?;

        Ok(CuckooLoadChunkResult {
            key: key.to_string(),
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::Value;

    #[test]
    fn test_extract_bool() {
        assert!(RedisBloomRepository::extract_bool(&Value::Int(1)));
        assert!(!RedisBloomRepository::extract_bool(&Value::Int(0)));
        assert!(RedisBloomRepository::extract_bool(&Value::Okay));
        assert!(!RedisBloomRepository::extract_bool(&Value::Nil));
    }

    #[test]
    fn test_extract_u64() {
        // Int value
        assert_eq!(RedisBloomRepository::extract_u64(&Value::Int(42)), 42);
        // BulkString with numeric value
        assert_eq!(
            RedisBloomRepository::extract_u64(&Value::BulkString(b"42".to_vec())),
            42
        );
        // BulkString with whitespace
        assert_eq!(
            RedisBloomRepository::extract_u64(&Value::BulkString(b" 123 ".to_vec())),
            123
        );
        // BulkString with non-numeric value (should return 0)
        assert_eq!(
            RedisBloomRepository::extract_u64(&Value::BulkString(b"not_a_number".to_vec())),
            0
        );
        // Nil value (should return 0)
        assert_eq!(RedisBloomRepository::extract_u64(&Value::Nil), 0);
        // Other types (should return 0)
        assert_eq!(RedisBloomRepository::extract_u64(&Value::Okay), 0);
    }

    #[test]
    fn test_extract_u32() {
        // Int value
        assert_eq!(RedisBloomRepository::extract_u32(&Value::Int(42)), 42);
        // BulkString with numeric value
        assert_eq!(
            RedisBloomRepository::extract_u32(&Value::BulkString(b"42".to_vec())),
            42
        );
        // BulkString with whitespace
        assert_eq!(
            RedisBloomRepository::extract_u32(&Value::BulkString(b" 123 ".to_vec())),
            123
        );
        // BulkString with non-numeric value (should return 0)
        assert_eq!(
            RedisBloomRepository::extract_u32(&Value::BulkString(b"not_a_number".to_vec())),
            0
        );
        // Nil value (should return 0)
        assert_eq!(RedisBloomRepository::extract_u32(&Value::Nil), 0);
    }

    #[test]
    fn test_parse_bloom_info() {
        let value = Value::Array(vec![
            Value::BulkString(b"Capacity".to_vec()),
            Value::Int(1000),
            Value::BulkString(b"Size".to_vec()),
            Value::Int(2048),
            Value::BulkString(b"Number of filters".to_vec()),
            Value::Int(1),
            Value::BulkString(b"Number of items inserted".to_vec()),
            Value::Int(100),
            Value::BulkString(b"Expansion rate".to_vec()),
            Value::Int(2),
        ]);

        let info = RedisBloomRepository::parse_bloom_info(value).unwrap();
        assert_eq!(info.capacity, 1000);
        assert_eq!(info.size, 2048);
        assert_eq!(info.num_filters, 1);
        assert_eq!(info.num_items_inserted, 100);
        assert_eq!(info.expansion, Some(2));
    }

    #[test]
    fn test_parse_cuckoo_info() {
        let value = Value::Array(vec![
            Value::BulkString(b"Size".to_vec()),
            Value::Int(4096),
            Value::BulkString(b"Number of buckets".to_vec()),
            Value::Int(512),
            Value::BulkString(b"Number of filters".to_vec()),
            Value::Int(1),
            Value::BulkString(b"Number of items inserted".to_vec()),
            Value::Int(100),
            Value::BulkString(b"Number of items deleted".to_vec()),
            Value::Int(5),
            Value::BulkString(b"Bucket size".to_vec()),
            Value::Int(2),
            Value::BulkString(b"Expansion rate".to_vec()),
            Value::Int(1),
            Value::BulkString(b"Max iterations".to_vec()),
            Value::Int(20),
        ]);

        let info = RedisBloomRepository::parse_cuckoo_info(value).unwrap();
        assert_eq!(info.size, 4096);
        assert_eq!(info.num_buckets, 512);
        assert_eq!(info.num_filters, 1);
        assert_eq!(info.num_items_inserted, 100);
        assert_eq!(info.num_items_deleted, 5);
        assert_eq!(info.bucket_size, 2);
    }

    #[test]
    fn test_parse_bloom_info_invalid_response() {
        // Non-array response should error
        let result = RedisBloomRepository::parse_bloom_info(Value::Nil);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CacheError::Internal(_)));

        // String response should error
        let result = RedisBloomRepository::parse_bloom_info(Value::BulkString(b"invalid".to_vec()));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cuckoo_info_invalid_response() {
        // Non-array response should error
        let result = RedisBloomRepository::parse_cuckoo_info(Value::Nil);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CacheError::Internal(_)));

        // Int response should error
        let result = RedisBloomRepository::parse_cuckoo_info(Value::Int(42));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_bloom_info_with_bulk_string_values() {
        // Test that INFO parsing handles BulkString numeric values (some Redis versions may return these)
        let value = Value::Array(vec![
            Value::BulkString(b"Capacity".to_vec()),
            Value::BulkString(b"1000".to_vec()),
            Value::BulkString(b"Size".to_vec()),
            Value::BulkString(b"2048".to_vec()),
            Value::BulkString(b"Number of filters".to_vec()),
            Value::BulkString(b"1".to_vec()),
            Value::BulkString(b"Number of items inserted".to_vec()),
            Value::BulkString(b"100".to_vec()),
        ]);

        let info = RedisBloomRepository::parse_bloom_info(value).unwrap();
        assert_eq!(info.capacity, 1000);
        assert_eq!(info.size, 2048);
        assert_eq!(info.num_filters, 1);
        assert_eq!(info.num_items_inserted, 100);
    }

    #[test]
    fn test_parse_bloom_info_unknown_fields() {
        // INFO response with unknown fields should be handled gracefully
        let value = Value::Array(vec![
            Value::BulkString(b"Capacity".to_vec()),
            Value::Int(1000),
            Value::BulkString(b"UnknownField".to_vec()),
            Value::Int(999),
            Value::BulkString(b"Size".to_vec()),
            Value::Int(2048),
        ]);

        let info = RedisBloomRepository::parse_bloom_info(value).unwrap();
        assert_eq!(info.capacity, 1000);
        assert_eq!(info.size, 2048);
    }

    #[test]
    fn test_parse_cuckoo_info_with_bulk_string_values() {
        // Test that INFO parsing handles BulkString numeric values
        let value = Value::Array(vec![
            Value::BulkString(b"Size".to_vec()),
            Value::BulkString(b"4096".to_vec()),
            Value::BulkString(b"Number of buckets".to_vec()),
            Value::BulkString(b"512".to_vec()),
        ]);

        let info = RedisBloomRepository::parse_cuckoo_info(value).unwrap();
        assert_eq!(info.size, 4096);
        assert_eq!(info.num_buckets, 512);
    }
}
