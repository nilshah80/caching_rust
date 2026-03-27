//! Bloom Filter Schemas
//!
//! Request/response schemas for Bloom filter and Cuckoo filter operations.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domain::entities::{
    BloomAddResult, BloomCardResult, BloomExistsResult, BloomInfo, BloomInsertOptions,
    BloomInsertResult, BloomLoadChunkResult, BloomReserveOptions, BloomReserveResult,
    BloomScanDumpResult, CuckooAddResult, CuckooCountResult, CuckooDelResult, CuckooExistsResult,
    CuckooInfo, CuckooInsertOptions, CuckooInsertResult, CuckooLoadChunkResult,
    CuckooReserveOptions, CuckooReserveResult, CuckooScanDumpResult,
};

// ==================== Bloom Filter Schemas ====================

/// Request to reserve a new Bloom filter
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct BloomReserveRequest {
    /// Error rate (false positive rate), must be between 0 and 1
    #[validate(range(
        min = 0.0001,
        max = 0.9999,
        message = "Error rate must be between 0.0001 and 0.9999"
    ))]
    pub error_rate: f64,

    /// Expected capacity (number of items)
    #[validate(range(min = 1, message = "Capacity must be at least 1"))]
    pub capacity: u64,

    /// Enable non-scaling behavior
    #[serde(default)]
    pub nonscaling: bool,

    /// Expansion rate when filter is full
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expansion: Option<u32>,
}

impl From<BloomReserveRequest> for BloomReserveOptions {
    fn from(req: BloomReserveRequest) -> Self {
        BloomReserveOptions {
            error_rate: req.error_rate,
            capacity: req.capacity,
            nonscaling: req.nonscaling,
            expansion: req.expansion,
        }
    }
}

/// Response for Bloom filter reserve operation
#[derive(Debug, Serialize, ToSchema)]
pub struct BloomReserveResponse {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

impl From<BloomReserveResult> for BloomReserveResponse {
    fn from(result: BloomReserveResult) -> Self {
        BloomReserveResponse {
            key: result.key,
            success: result.success,
        }
    }
}

/// Request to add item(s) to a Bloom filter
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct BloomAddRequest {
    /// Items to add
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<String>,
}

/// Response for Bloom filter add operation
#[derive(Debug, Serialize, ToSchema)]
pub struct BloomAddResponse {
    /// Key name
    pub key: String,
    /// Results for each item (true if newly added, false if already existed)
    pub results: Vec<bool>,
}

impl From<BloomAddResult> for BloomAddResponse {
    fn from(result: BloomAddResult) -> Self {
        BloomAddResponse {
            key: result.key,
            results: result.results,
        }
    }
}

/// Request to check if item(s) exist in a Bloom filter
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct BloomExistsRequest {
    /// Items to check
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<String>,
}

/// Response for Bloom filter exists operation
#[derive(Debug, Serialize, ToSchema)]
pub struct BloomExistsResponse {
    /// Key name
    pub key: String,
    /// Results for each item (true if probably exists, false if definitely not)
    pub results: Vec<bool>,
}

impl From<BloomExistsResult> for BloomExistsResponse {
    fn from(result: BloomExistsResult) -> Self {
        BloomExistsResponse {
            key: result.key,
            results: result.results,
        }
    }
}

/// Request to insert items with options
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct BloomInsertRequest {
    /// Items to insert
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<String>,

    /// Expected capacity (auto-create with this capacity if filter doesn't exist)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u64>,

    /// Error rate (auto-create with this error rate if filter doesn't exist)
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

impl From<BloomInsertRequest> for BloomInsertOptions {
    fn from(req: BloomInsertRequest) -> Self {
        BloomInsertOptions {
            capacity: req.capacity,
            error_rate: req.error_rate,
            expansion: req.expansion,
            nocreate: req.nocreate,
            nonscaling: req.nonscaling,
        }
    }
}

/// Response for Bloom filter insert operation
#[derive(Debug, Serialize, ToSchema)]
pub struct BloomInsertResponse {
    /// Key name
    pub key: String,
    /// Results for each item (true if newly added, false if already existed)
    pub results: Vec<bool>,
}

impl From<BloomInsertResult> for BloomInsertResponse {
    fn from(result: BloomInsertResult) -> Self {
        BloomInsertResponse {
            key: result.key,
            results: result.results,
        }
    }
}

/// Response for Bloom filter info operation
#[derive(Debug, Serialize, ToSchema)]
pub struct BloomInfoResponse {
    /// Number of sub-filters
    #[serde(rename = "numFilters")]
    pub num_filters: u64,
    /// Number of items inserted
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

impl From<BloomInfo> for BloomInfoResponse {
    fn from(info: BloomInfo) -> Self {
        BloomInfoResponse {
            num_filters: info.num_filters,
            num_items_inserted: info.num_items_inserted,
            capacity: info.capacity,
            size: info.size,
            expansion: info.expansion,
        }
    }
}

/// Response for Bloom filter cardinality operation
#[derive(Debug, Serialize, ToSchema)]
pub struct BloomCardResponse {
    /// Key name
    pub key: String,
    /// Estimated cardinality
    pub cardinality: u64,
}

impl From<BloomCardResult> for BloomCardResponse {
    fn from(result: BloomCardResult) -> Self {
        BloomCardResponse {
            key: result.key,
            cardinality: result.cardinality,
        }
    }
}

/// Query parameters for Bloom filter scandump
#[derive(Debug, Deserialize, ToSchema)]
pub struct BloomScanDumpParams {
    /// Iterator position (start with 0)
    #[serde(default)]
    pub iterator: u64,
}

/// Response for Bloom filter scandump operation
#[derive(Debug, Serialize, ToSchema)]
pub struct BloomScanDumpResponse {
    /// Iterator position (0 means done)
    pub iterator: u64,
    /// Chunk data (base64 encoded)
    pub data: Option<String>,
}

impl From<BloomScanDumpResult> for BloomScanDumpResponse {
    fn from(result: BloomScanDumpResult) -> Self {
        BloomScanDumpResponse {
            iterator: result.iterator,
            data: result.data,
        }
    }
}

/// Request to load a Bloom filter chunk
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct BloomLoadChunkRequest {
    /// Iterator position
    pub iterator: u64,
    /// Chunk data (base64 encoded)
    #[validate(length(min = 1, message = "Data is required"))]
    pub data: String,
}

/// Response for Bloom filter loadchunk operation
#[derive(Debug, Serialize, ToSchema)]
pub struct BloomLoadChunkResponse {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

impl From<BloomLoadChunkResult> for BloomLoadChunkResponse {
    fn from(result: BloomLoadChunkResult) -> Self {
        BloomLoadChunkResponse {
            key: result.key,
            success: result.success,
        }
    }
}

// ==================== Cuckoo Filter Schemas ====================

/// Request to reserve a new Cuckoo filter
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CuckooReserveRequest {
    /// Expected capacity
    #[validate(range(min = 1, message = "Capacity must be at least 1"))]
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

impl From<CuckooReserveRequest> for CuckooReserveOptions {
    fn from(req: CuckooReserveRequest) -> Self {
        CuckooReserveOptions {
            capacity: req.capacity,
            bucket_size: req.bucket_size,
            max_iterations: req.max_iterations,
            expansion: req.expansion,
        }
    }
}

/// Response for Cuckoo filter reserve operation
#[derive(Debug, Serialize, ToSchema)]
pub struct CuckooReserveResponse {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

impl From<CuckooReserveResult> for CuckooReserveResponse {
    fn from(result: CuckooReserveResult) -> Self {
        CuckooReserveResponse {
            key: result.key,
            success: result.success,
        }
    }
}

/// Request to add item(s) to a Cuckoo filter
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CuckooAddRequest {
    /// Item to add
    #[validate(length(min = 1, message = "Item cannot be empty"))]
    pub item: String,
}

/// Response for Cuckoo filter add operation
#[derive(Debug, Serialize, ToSchema)]
pub struct CuckooAddResponse {
    /// Key name
    pub key: String,
    /// Whether item was added
    pub added: bool,
}

impl From<CuckooAddResult> for CuckooAddResponse {
    fn from(result: CuckooAddResult) -> Self {
        CuckooAddResponse {
            key: result.key,
            added: result.added,
        }
    }
}

/// Request to insert items with options
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CuckooInsertRequest {
    /// Items to insert
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<String>,

    /// Expected capacity (auto-create with this capacity if filter doesn't exist)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u64>,

    /// Don't create filter if it doesn't exist
    #[serde(default)]
    pub nocreate: bool,
}

impl From<CuckooInsertRequest> for CuckooInsertOptions {
    fn from(req: CuckooInsertRequest) -> Self {
        CuckooInsertOptions {
            capacity: req.capacity,
            nocreate: req.nocreate,
        }
    }
}

/// Response for Cuckoo filter insert operation
#[derive(Debug, Serialize, ToSchema)]
pub struct CuckooInsertResponse {
    /// Key name
    pub key: String,
    /// Results for each item
    pub results: Vec<bool>,
}

impl From<CuckooInsertResult> for CuckooInsertResponse {
    fn from(result: CuckooInsertResult) -> Self {
        CuckooInsertResponse {
            key: result.key,
            results: result.results,
        }
    }
}

/// Request to check if item(s) exist in a Cuckoo filter
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CuckooExistsRequest {
    /// Items to check
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<String>,
}

/// Response for Cuckoo filter exists operation
#[derive(Debug, Serialize, ToSchema)]
pub struct CuckooExistsResponse {
    /// Key name
    pub key: String,
    /// Results for each item
    pub results: Vec<bool>,
}

impl From<CuckooExistsResult> for CuckooExistsResponse {
    fn from(result: CuckooExistsResult) -> Self {
        CuckooExistsResponse {
            key: result.key,
            results: result.results,
        }
    }
}

/// Request to delete an item from a Cuckoo filter
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CuckooDelRequest {
    /// Item to delete
    #[validate(length(min = 1, message = "Item cannot be empty"))]
    pub item: String,
}

/// Response for Cuckoo filter delete operation
#[derive(Debug, Serialize, ToSchema)]
pub struct CuckooDelResponse {
    /// Key name
    pub key: String,
    /// Whether item was deleted
    pub deleted: bool,
}

impl From<CuckooDelResult> for CuckooDelResponse {
    fn from(result: CuckooDelResult) -> Self {
        CuckooDelResponse {
            key: result.key,
            deleted: result.deleted,
        }
    }
}

/// Request to count item occurrences
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CuckooCountRequest {
    /// Item to count
    #[validate(length(min = 1, message = "Item cannot be empty"))]
    pub item: String,
}

/// Response for Cuckoo filter count operation
#[derive(Debug, Serialize, ToSchema)]
pub struct CuckooCountResponse {
    /// Key name
    pub key: String,
    /// Count of item occurrences
    pub count: u64,
}

impl From<CuckooCountResult> for CuckooCountResponse {
    fn from(result: CuckooCountResult) -> Self {
        CuckooCountResponse {
            key: result.key,
            count: result.count,
        }
    }
}

/// Response for Cuckoo filter info operation
#[derive(Debug, Serialize, ToSchema)]
pub struct CuckooInfoResponse {
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

impl From<CuckooInfo> for CuckooInfoResponse {
    fn from(info: CuckooInfo) -> Self {
        CuckooInfoResponse {
            size: info.size,
            num_buckets: info.num_buckets,
            num_filters: info.num_filters,
            num_items_inserted: info.num_items_inserted,
            num_items_deleted: info.num_items_deleted,
            bucket_size: info.bucket_size,
            expansion_rate: info.expansion_rate,
            max_iterations: info.max_iterations,
        }
    }
}

/// Query parameters for Cuckoo filter scandump
#[derive(Debug, Deserialize, ToSchema)]
pub struct CuckooScanDumpParams {
    /// Iterator position (start with 0)
    #[serde(default)]
    pub iterator: u64,
}

/// Response for Cuckoo filter scandump operation
#[derive(Debug, Serialize, ToSchema)]
pub struct CuckooScanDumpResponse {
    /// Iterator position (0 means done)
    pub iterator: u64,
    /// Chunk data (base64 encoded)
    pub data: Option<String>,
}

impl From<CuckooScanDumpResult> for CuckooScanDumpResponse {
    fn from(result: CuckooScanDumpResult) -> Self {
        CuckooScanDumpResponse {
            iterator: result.iterator,
            data: result.data,
        }
    }
}

/// Request to load a Cuckoo filter chunk
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CuckooLoadChunkRequest {
    /// Iterator position
    pub iterator: u64,
    /// Chunk data (base64 encoded)
    #[validate(length(min = 1, message = "Data is required"))]
    pub data: String,
}

/// Response for Cuckoo filter loadchunk operation
#[derive(Debug, Serialize, ToSchema)]
pub struct CuckooLoadChunkResponse {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

impl From<CuckooLoadChunkResult> for CuckooLoadChunkResponse {
    fn from(result: CuckooLoadChunkResult) -> Self {
        CuckooLoadChunkResponse {
            key: result.key,
            success: result.success,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_reserve_request_conversion() {
        let req = BloomReserveRequest {
            error_rate: 0.01,
            capacity: 1000,
            nonscaling: true,
            expansion: Some(2),
        };
        let opts: BloomReserveOptions = req.into();
        assert_eq!(opts.error_rate, 0.01);
        assert_eq!(opts.capacity, 1000);
        assert!(opts.nonscaling);
        assert_eq!(opts.expansion, Some(2));
    }

    #[test]
    fn test_bloom_insert_request_conversion() {
        let req = BloomInsertRequest {
            items: vec!["a".to_string()],
            capacity: Some(100),
            error_rate: Some(0.01),
            expansion: None,
            nocreate: true,
            nonscaling: false,
        };
        let opts: BloomInsertOptions = req.into();
        assert_eq!(opts.capacity, Some(100));
        assert!(opts.nocreate);
    }

    #[test]
    fn test_cuckoo_reserve_request_conversion() {
        let req = CuckooReserveRequest {
            capacity: 1000,
            bucket_size: Some(4),
            max_iterations: Some(500),
            expansion: Some(2),
        };
        let opts: CuckooReserveOptions = req.into();
        assert_eq!(opts.capacity, 1000);
        assert_eq!(opts.bucket_size, Some(4));
    }

    #[test]
    fn test_bloom_info_response_conversion() {
        let info = BloomInfo {
            num_filters: 1,
            num_items_inserted: 100,
            capacity: 1000,
            size: 2048,
            expansion: Some(2),
        };
        let resp: BloomInfoResponse = info.into();
        assert_eq!(resp.num_filters, 1);
        assert_eq!(resp.capacity, 1000);
    }

    #[test]
    fn test_cuckoo_info_response_conversion() {
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
        let resp: CuckooInfoResponse = info.into();
        assert_eq!(resp.size, 4096);
        assert_eq!(resp.num_items_deleted, 5);
    }
}
