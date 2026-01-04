//! Bloom Filter Domain Entities
//!
//! Domain types for RedisBloom operations including Bloom filters and Cuckoo filters.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ==================== Bloom Filter Types ====================

/// Options for BF.RESERVE command
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct BloomReserveOptions {
    /// Error rate (false positive rate), default 0.01
    pub error_rate: f64,
    /// Expected capacity (number of items)
    pub capacity: u64,
    /// Enable non-scaling behavior
    #[serde(default)]
    pub nonscaling: bool,
    /// Expansion rate when filter is full (default 2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expansion: Option<u32>,
}

/// Options for BF.INSERT command
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct BloomInsertOptions {
    /// Expected capacity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u64>,
    /// Error rate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_rate: Option<f64>,
    /// Expansion rate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expansion: Option<u32>,
    /// Don't create filter if it doesn't exist
    #[serde(default)]
    pub nocreate: bool,
    /// Enable non-scaling behavior
    #[serde(default)]
    pub nonscaling: bool,
}

/// Result of BF.ADD/BF.MADD operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BloomAddResult {
    /// Key name
    pub key: String,
    /// Items that were newly added (true if new, false if already existed)
    pub results: Vec<bool>,
}

/// Result of BF.EXISTS/BF.MEXISTS operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BloomExistsResult {
    /// Key name
    pub key: String,
    /// Existence results (true if probably exists, false if definitely not)
    pub results: Vec<bool>,
}

/// Result of BF.INSERT operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BloomInsertResult {
    /// Key name
    pub key: String,
    /// Insert results (true if newly added, false if already existed)
    pub results: Vec<bool>,
}

/// Result of BF.INFO operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BloomInfo {
    /// Number of sub-filters
    #[serde(rename = "numFilters")]
    pub num_filters: u64,
    /// Number of items added
    #[serde(rename = "numItemsInserted")]
    pub num_items_inserted: u64,
    /// Total capacity
    pub capacity: u64,
    /// Size in bytes
    pub size: u64,
    /// Expansion rate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expansion: Option<u32>,
}

/// Result of BF.CARD operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BloomCardResult {
    /// Key name
    pub key: String,
    /// Estimated cardinality
    pub cardinality: u64,
}

/// Result of BF.SCANDUMP operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BloomScanDumpResult {
    /// Iterator position (0 means done)
    pub iterator: u64,
    /// Chunk data (base64 encoded)
    pub data: Option<String>,
}

/// Result of BF.LOADCHUNK operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BloomLoadChunkResult {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

// ==================== Cuckoo Filter Types ====================

/// Options for CF.RESERVE command
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct CuckooReserveOptions {
    /// Expected capacity
    pub capacity: u64,
    /// Bucket size (default 2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_size: Option<u32>,
    /// Max iterations before declaring filter full
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    /// Expansion rate when filter is full
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expansion: Option<u32>,
}

/// Options for CF.INSERT command
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct CuckooInsertOptions {
    /// Expected capacity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u64>,
    /// Don't create filter if it doesn't exist
    #[serde(default)]
    pub nocreate: bool,
}

/// Result of CF.ADD/CF.ADDNX operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CuckooAddResult {
    /// Key name
    pub key: String,
    /// Whether item was added (false if filter is full or item exists for ADDNX)
    pub added: bool,
}

/// Result of CF.INSERT/CF.INSERTNX operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CuckooInsertResult {
    /// Key name
    pub key: String,
    /// Insert results for each item
    pub results: Vec<bool>,
}

/// Result of CF.EXISTS/CF.MEXISTS operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CuckooExistsResult {
    /// Key name
    pub key: String,
    /// Existence results
    pub results: Vec<bool>,
}

/// Result of CF.DEL operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CuckooDelResult {
    /// Key name
    pub key: String,
    /// Whether item was deleted
    pub deleted: bool,
}

/// Result of CF.COUNT operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CuckooCountResult {
    /// Key name
    pub key: String,
    /// Count of item occurrences
    pub count: u64,
}

/// Result of CF.INFO operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CuckooInfo {
    /// Size in bytes
    pub size: u64,
    /// Number of buckets
    #[serde(rename = "numBuckets")]
    pub num_buckets: u64,
    /// Number of sub-filters
    #[serde(rename = "numFilters")]
    pub num_filters: u64,
    /// Number of items inserted
    #[serde(rename = "numItemsInserted")]
    pub num_items_inserted: u64,
    /// Number of items deleted
    #[serde(rename = "numItemsDeleted")]
    pub num_items_deleted: u64,
    /// Bucket size
    #[serde(rename = "bucketSize")]
    pub bucket_size: u32,
    /// Expansion rate
    #[serde(rename = "expansionRate")]
    pub expansion_rate: u32,
    /// Max iterations
    #[serde(rename = "maxIterations")]
    pub max_iterations: u32,
}

/// Result of CF.SCANDUMP operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CuckooScanDumpResult {
    /// Iterator position (0 means done)
    pub iterator: u64,
    /// Chunk data (base64 encoded)
    pub data: Option<String>,
}

/// Result of CF.LOADCHUNK operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CuckooLoadChunkResult {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

// ==================== Reserve Result Types ====================

/// Result of BF.RESERVE operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BloomReserveResult {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

/// Result of CF.RESERVE operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CuckooReserveResult {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_reserve_options_default() {
        let opts = BloomReserveOptions::default();
        assert_eq!(opts.error_rate, 0.0);
        assert_eq!(opts.capacity, 0);
        assert!(!opts.nonscaling);
        assert!(opts.expansion.is_none());
    }

    #[test]
    fn test_bloom_insert_options_serialization() {
        let opts = BloomInsertOptions {
            capacity: Some(1000),
            error_rate: Some(0.01),
            expansion: Some(2),
            nocreate: true,
            nonscaling: false,
        };
        let json = serde_json::to_string(&opts).unwrap();
        assert!(json.contains("\"capacity\":1000"));
        assert!(json.contains("\"nocreate\":true"));
    }

    #[test]
    fn test_bloom_info_serialization() {
        let info = BloomInfo {
            num_filters: 1,
            num_items_inserted: 100,
            capacity: 1000,
            size: 2048,
            expansion: Some(2),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"numFilters\":1"));
        assert!(json.contains("\"numItemsInserted\":100"));
    }

    #[test]
    fn test_cuckoo_reserve_options_default() {
        let opts = CuckooReserveOptions::default();
        assert_eq!(opts.capacity, 0);
        assert!(opts.bucket_size.is_none());
        assert!(opts.max_iterations.is_none());
        assert!(opts.expansion.is_none());
    }

    #[test]
    fn test_cuckoo_info_serialization() {
        let info = CuckooInfo {
            size: 4096,
            num_buckets: 512,
            num_filters: 1,
            num_items_inserted: 100,
            num_items_deleted: 5,
            bucket_size: 2,
            expansion_rate: 1,
            max_iterations: 20,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"numBuckets\":512"));
        assert!(json.contains("\"bucketSize\":2"));
    }

    #[test]
    fn test_bloom_add_result() {
        let result = BloomAddResult {
            key: "bf:test".to_string(),
            results: vec![true, false, true],
        };
        assert_eq!(result.results.len(), 3);
        assert!(result.results[0]);
        assert!(!result.results[1]);
    }

    #[test]
    fn test_cuckoo_del_result() {
        let result = CuckooDelResult {
            key: "cf:test".to_string(),
            deleted: true,
        };
        assert!(result.deleted);
    }
}
