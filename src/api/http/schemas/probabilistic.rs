//! Probabilistic Data Structure Schemas
//!
//! Request/response schemas for Count-Min Sketch, Top-K, and HyperLogLog operations.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domain::entities::{
    CmsIncrByResult, CmsInfo, CmsInitResult, CmsMergeResult, CmsQueryResult, PfAddResult,
    PfCountResult, PfMergeResult, TDigestAckResult, TDigestInfo, TDigestRanksResult,
    TDigestScalarResult, TDigestValuesResult, TopKAddResult, TopKCountResult, TopKIncrByResult,
    TopKInfo, TopKItem, TopKListResult, TopKQueryResult, TopKReserveResult,
};

// ==================== Count-Min Sketch Schemas ====================

/// Request to initialize a Count-Min Sketch by dimensions
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CmsInitByDimRequest {
    /// Width (number of counters per row)
    #[validate(range(min = 1, message = "Width must be at least 1"))]
    pub width: u64,

    /// Depth (number of hash functions/rows)
    #[validate(range(min = 1, message = "Depth must be at least 1"))]
    pub depth: u64,
}

/// Request to initialize a Count-Min Sketch by probability
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CmsInitByProbRequest {
    /// Error rate (overestimation, e.g., 0.01 for 1%)
    #[validate(range(
        min = 0.0001,
        max = 0.9999,
        message = "Error must be between 0.0001 and 0.9999"
    ))]
    pub error: f64,

    /// Probability of error (e.g., 0.001 for 0.1%)
    #[validate(range(
        min = 0.0001,
        max = 0.9999,
        message = "Probability must be between 0.0001 and 0.9999"
    ))]
    pub probability: f64,
}

/// Response for CMS initialization
#[derive(Debug, Serialize, ToSchema)]
pub struct CmsInitResponse {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

impl From<CmsInitResult> for CmsInitResponse {
    fn from(result: CmsInitResult) -> Self {
        Self {
            key: result.key,
            success: result.success,
        }
    }
}

/// Item with increment value for CMS.INCRBY
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CmsIncrByItem {
    /// Item name
    pub item: String,
    /// Increment amount
    #[serde(default = "default_increment")]
    pub increment: u64,
}

fn default_increment() -> u64 {
    1
}

/// Request to increment item counts in a CMS
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CmsIncrByRequest {
    /// Items with their increment values
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<CmsIncrByItem>,
}

/// Response for CMS.INCRBY
#[derive(Debug, Serialize, ToSchema)]
pub struct CmsIncrByResponse {
    /// Key name
    pub key: String,
    /// New counts for each item after increment
    pub counts: Vec<u64>,
}

impl From<CmsIncrByResult> for CmsIncrByResponse {
    fn from(result: CmsIncrByResult) -> Self {
        Self {
            key: result.key,
            counts: result.counts,
        }
    }
}

/// Request to query item counts in a CMS
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CmsQueryRequest {
    /// Items to query
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<String>,
}

/// Response for CMS.QUERY
#[derive(Debug, Serialize, ToSchema)]
pub struct CmsQueryResponse {
    /// Key name
    pub key: String,
    /// Estimated counts for each item
    pub counts: Vec<u64>,
}

impl From<CmsQueryResult> for CmsQueryResponse {
    fn from(result: CmsQueryResult) -> Self {
        Self {
            key: result.key,
            counts: result.counts,
        }
    }
}

/// Request to merge multiple Count-Min Sketches
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CmsMergeRequest {
    /// Source keys to merge
    #[validate(length(min = 1, message = "At least one source is required"))]
    pub sources: Vec<String>,

    /// Optional weights for each source (must match number of sources if provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weights: Option<Vec<u64>>,
}

/// Response for CMS.MERGE
#[derive(Debug, Serialize, ToSchema)]
pub struct CmsMergeResponse {
    /// Destination key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

impl From<CmsMergeResult> for CmsMergeResponse {
    fn from(result: CmsMergeResult) -> Self {
        Self {
            key: result.key,
            success: result.success,
        }
    }
}

/// Response for CMS.INFO
#[derive(Debug, Serialize, ToSchema)]
pub struct CmsInfoResponse {
    /// Width (number of counters per row)
    pub width: u64,
    /// Depth (number of hash functions)
    pub depth: u64,
    /// Total count of all increments
    pub count: u64,
}

impl From<CmsInfo> for CmsInfoResponse {
    fn from(info: CmsInfo) -> Self {
        Self {
            width: info.width,
            depth: info.depth,
            count: info.count,
        }
    }
}

// ==================== Top-K Schemas ====================

/// Request to reserve a Top-K filter
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TopKReserveRequest {
    /// Number of top items to track
    #[validate(range(min = 1, message = "K must be at least 1"))]
    pub k: u64,

    /// Width of the underlying Count-Min Sketch (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u64>,

    /// Depth of the underlying Count-Min Sketch (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u64>,

    /// Decay constant (optional, between 0 and 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decay: Option<f64>,
}

/// Response for TOPK.RESERVE
#[derive(Debug, Serialize, ToSchema)]
pub struct TopKReserveResponse {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

impl From<TopKReserveResult> for TopKReserveResponse {
    fn from(result: TopKReserveResult) -> Self {
        Self {
            key: result.key,
            success: result.success,
        }
    }
}

/// Request to add items to a Top-K filter
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TopKAddRequest {
    /// Items to add
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<String>,
}

/// Response for TOPK.ADD
#[derive(Debug, Serialize, ToSchema)]
pub struct TopKAddResponse {
    /// Key name
    pub key: String,
    /// Items that were dropped from top-k (None if item didn't cause a drop)
    pub dropped: Vec<Option<String>>,
}

impl From<TopKAddResult> for TopKAddResponse {
    fn from(result: TopKAddResult) -> Self {
        Self {
            key: result.key,
            dropped: result.dropped,
        }
    }
}

/// Item with increment value for TOPK.INCRBY
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct TopKIncrByItem {
    /// Item name
    pub item: String,
    /// Increment amount
    #[serde(default = "default_increment")]
    pub increment: u64,
}

/// Request to increment item counts in a Top-K filter
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TopKIncrByRequest {
    /// Items with their increment values
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<TopKIncrByItem>,
}

/// Response for TOPK.INCRBY
#[derive(Debug, Serialize, ToSchema)]
pub struct TopKIncrByResponse {
    /// Key name
    pub key: String,
    /// Items that were dropped from top-k
    pub dropped: Vec<Option<String>>,
}

impl From<TopKIncrByResult> for TopKIncrByResponse {
    fn from(result: TopKIncrByResult) -> Self {
        Self {
            key: result.key,
            dropped: result.dropped,
        }
    }
}

/// Request to query items in a Top-K filter
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TopKQueryRequest {
    /// Items to query
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<String>,
}

/// Response for TOPK.QUERY
#[derive(Debug, Serialize, ToSchema)]
pub struct TopKQueryResponse {
    /// Key name
    pub key: String,
    /// Whether each item is in the top-k
    pub results: Vec<bool>,
}

impl From<TopKQueryResult> for TopKQueryResponse {
    fn from(result: TopKQueryResult) -> Self {
        Self {
            key: result.key,
            results: result.results,
        }
    }
}

/// Response for TOPK.COUNT
#[derive(Debug, Serialize, ToSchema)]
pub struct TopKCountResponse {
    /// Key name
    pub key: String,
    /// Estimated counts for each item
    pub counts: Vec<u64>,
}

impl From<TopKCountResult> for TopKCountResponse {
    fn from(result: TopKCountResult) -> Self {
        Self {
            key: result.key,
            counts: result.counts,
        }
    }
}

/// Query parameters for TOPK.LIST
#[derive(Debug, Deserialize, ToSchema)]
pub struct TopKListQuery {
    /// Include counts in response
    #[serde(default)]
    pub with_count: bool,
}

/// Response item for TOPK.LIST
#[derive(Debug, Serialize, ToSchema)]
pub struct TopKListItem {
    /// Item value
    pub item: String,
    /// Estimated count (0 if with_count was false)
    pub count: u64,
}

impl From<TopKItem> for TopKListItem {
    fn from(item: TopKItem) -> Self {
        Self {
            item: item.item,
            count: item.count,
        }
    }
}

/// Response for TOPK.LIST
#[derive(Debug, Serialize, ToSchema)]
pub struct TopKListResponse {
    /// Key name
    pub key: String,
    /// Items in the top-k
    pub items: Vec<TopKListItem>,
}

impl From<TopKListResult> for TopKListResponse {
    fn from(result: TopKListResult) -> Self {
        Self {
            key: result.key,
            items: result.items.into_iter().map(TopKListItem::from).collect(),
        }
    }
}

/// Response for TOPK.INFO
#[derive(Debug, Serialize, ToSchema)]
pub struct TopKInfoResponse {
    /// Number of top items to track
    pub k: u64,
    /// Width of the underlying Count-Min Sketch
    pub width: u64,
    /// Depth of the underlying Count-Min Sketch
    pub depth: u64,
    /// Decay constant
    pub decay: f64,
}

impl From<TopKInfo> for TopKInfoResponse {
    fn from(info: TopKInfo) -> Self {
        Self {
            k: info.k,
            width: info.width,
            depth: info.depth,
            decay: info.decay,
        }
    }
}

// ==================== T-Digest Schemas ====================

/// Request body for TDIGEST.CREATE.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TDigestCreateRequest {
    /// Optional compression (accuracy/memory trade-off, default 100)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, message = "compression must be >= 1"))]
    pub compression: Option<u64>,
}

/// Generic ack response used by TDIGEST.CREATE/ADD/RESET/MERGE.
#[derive(Debug, Serialize, ToSchema)]
pub struct TDigestAckResponse {
    /// Key name
    pub key: String,
    /// Whether Redis returned OK
    pub success: bool,
}

impl From<TDigestAckResult> for TDigestAckResponse {
    fn from(result: TDigestAckResult) -> Self {
        Self {
            key: result.key,
            success: result.success,
        }
    }
}

/// Request body for TDIGEST.ADD.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TDigestAddRequest {
    /// Values to add to the sketch
    #[validate(length(min = 1, message = "At least one value is required"))]
    pub values: Vec<f64>,
}

/// Request body for TDIGEST.QUANTILE.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TDigestQuantileRequest {
    /// Quantiles to estimate (each must be between 0 and 1)
    #[validate(length(min = 1, message = "At least one quantile is required"))]
    pub quantiles: Vec<f64>,
}

/// Request body for TDIGEST.CDF / TDIGEST.RANK / TDIGEST.REVRANK.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TDigestValuesRequest {
    /// Values to look up
    #[validate(length(min = 1, message = "At least one value is required"))]
    pub values: Vec<f64>,
}

/// Request body for TDIGEST.BYRANK / TDIGEST.BYREVRANK.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TDigestRanksRequest {
    /// Ranks to look up
    #[validate(length(min = 1, message = "At least one rank is required"))]
    pub ranks: Vec<u64>,
}

/// Request body for TDIGEST.MERGE.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TDigestMergeRequest {
    /// Source sketches to merge into the destination
    #[validate(length(min = 1, message = "At least one source is required"))]
    pub sources: Vec<String>,

    /// Optional compression override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, message = "compression must be >= 1"))]
    pub compression: Option<u64>,

    /// If true, allow overwriting the destination if it already exists
    #[serde(default)]
    pub override_existing: bool,
}

/// Request body for TDIGEST.TRIMMED_MEAN.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TDigestTrimmedMeanRequest {
    /// Lower quantile cut-off (inclusive)
    #[validate(range(min = 0.0, max = 1.0, message = "low_cut_quantile must be in [0, 1]"))]
    pub low_cut_quantile: f64,

    /// Upper quantile cut-off (inclusive)
    #[validate(range(min = 0.0, max = 1.0, message = "high_cut_quantile must be in [0, 1]"))]
    pub high_cut_quantile: f64,
}

/// Response for commands that return an array of floats in the same order as the
/// input (TDIGEST.QUANTILE / TDIGEST.CDF / TDIGEST.BYRANK / TDIGEST.BYREVRANK).
/// `null` is used where Redis returned `nan` (e.g. empty sketch).
#[derive(Debug, Serialize, ToSchema)]
pub struct TDigestValuesResponse {
    /// Key name
    pub key: String,
    /// One result per input, preserving order
    pub values: Vec<Option<f64>>,
}

impl From<TDigestValuesResult> for TDigestValuesResponse {
    fn from(result: TDigestValuesResult) -> Self {
        Self {
            key: result.key,
            values: result.values,
        }
    }
}

/// Response for TDIGEST.RANK / TDIGEST.REVRANK.
/// Raw signed ints preserve sentinel values (-1/-2) from the module.
#[derive(Debug, Serialize, ToSchema)]
pub struct TDigestRanksResponse {
    /// Key name
    pub key: String,
    /// Rank per input, preserving order
    pub ranks: Vec<i64>,
}

impl From<TDigestRanksResult> for TDigestRanksResponse {
    fn from(result: TDigestRanksResult) -> Self {
        Self {
            key: result.key,
            ranks: result.ranks,
        }
    }
}

/// Response for TDIGEST.MIN / TDIGEST.MAX / TDIGEST.TRIMMED_MEAN.
#[derive(Debug, Serialize, ToSchema)]
pub struct TDigestScalarResponse {
    /// Key name
    pub key: String,
    /// Scalar value (null if sketch is empty and Redis returned `nan`)
    pub value: Option<f64>,
}

impl From<TDigestScalarResult> for TDigestScalarResponse {
    fn from(result: TDigestScalarResult) -> Self {
        Self {
            key: result.key,
            value: result.value,
        }
    }
}

/// Response for TDIGEST.INFO.
#[derive(Debug, Serialize, ToSchema)]
pub struct TDigestInfoResponse {
    /// Compression parameter
    pub compression: u64,
    /// Maximum capacity (derived from compression)
    pub capacity: u64,
    /// Number of merged centroids
    pub merged_nodes: u64,
    /// Number of unmerged centroids
    pub unmerged_nodes: u64,
    /// Total weight of merged centroids
    pub merged_weight: f64,
    /// Total weight of unmerged centroids
    pub unmerged_weight: f64,
    /// Count of observations added
    pub observations: u64,
    /// Number of compressions performed
    pub total_compressions: u64,
    /// Memory footprint in bytes
    pub memory_usage: u64,
}

impl From<TDigestInfo> for TDigestInfoResponse {
    fn from(info: TDigestInfo) -> Self {
        Self {
            compression: info.compression,
            capacity: info.capacity,
            merged_nodes: info.merged_nodes,
            unmerged_nodes: info.unmerged_nodes,
            merged_weight: info.merged_weight,
            unmerged_weight: info.unmerged_weight,
            observations: info.observations,
            total_compressions: info.total_compressions,
            memory_usage: info.memory_usage,
        }
    }
}

// ==================== HyperLogLog Schemas ====================

/// Request to add elements to a HyperLogLog
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PfAddRequest {
    /// Elements to add
    #[validate(length(min = 1, message = "At least one element is required"))]
    pub elements: Vec<String>,
}

/// Response for PFADD
#[derive(Debug, Serialize, ToSchema)]
pub struct PfAddResponse {
    /// Key name
    pub key: String,
    /// Whether the cardinality estimate changed
    pub changed: bool,
}

impl From<PfAddResult> for PfAddResponse {
    fn from(result: PfAddResult) -> Self {
        Self {
            key: result.key,
            changed: result.changed,
        }
    }
}

/// Request to count unique elements in HyperLogLog(s)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PfCountRequest {
    /// Keys to count (can be multiple for union count)
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
}

/// Response for PFCOUNT
#[derive(Debug, Serialize, ToSchema)]
pub struct PfCountResponse {
    /// Keys that were counted
    pub keys: Vec<String>,
    /// Estimated unique element count
    pub count: u64,
}

impl From<PfCountResult> for PfCountResponse {
    fn from(result: PfCountResult) -> Self {
        Self {
            keys: result.keys,
            count: result.count,
        }
    }
}

/// Request to merge multiple HyperLogLogs
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PfMergeRequest {
    /// Source keys to merge
    #[validate(length(min = 1, message = "At least one source is required"))]
    pub sources: Vec<String>,
}

/// Response for PFMERGE
#[derive(Debug, Serialize, ToSchema)]
pub struct PfMergeResponse {
    /// Destination key name
    pub dest_key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

impl From<PfMergeResult> for PfMergeResponse {
    fn from(result: PfMergeResult) -> Self {
        Self {
            dest_key: result.dest_key,
            success: result.success,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cms_init_by_dim_request() {
        let json = r#"{"width": 2000, "depth": 5}"#;
        let req: CmsInitByDimRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.width, 2000);
        assert_eq!(req.depth, 5);
    }

    #[test]
    fn test_cms_incr_by_item_default() {
        let json = r#"{"item": "test"}"#;
        let item: CmsIncrByItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.item, "test");
        assert_eq!(item.increment, 1);
    }

    #[test]
    fn test_topk_reserve_request() {
        let json = r#"{"k": 10}"#;
        let req: TopKReserveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.k, 10);
        assert!(req.width.is_none());
    }

    #[test]
    fn test_pf_add_request() {
        let json = r#"{"elements": ["a", "b", "c"]}"#;
        let req: PfAddRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.elements.len(), 3);
    }

    #[test]
    fn test_topk_list_query_default() {
        let json = r#"{}"#;
        let query: TopKListQuery = serde_json::from_str(json).unwrap();
        assert!(!query.with_count);
    }

    #[test]
    fn test_probabilistic_response_conversions() {
        let cms_init = CmsInitResponse::from(CmsInitResult {
            key: "cms:init".to_string(),
            success: true,
        });
        assert!(cms_init.success);

        let cms_incr = CmsIncrByResponse::from(CmsIncrByResult {
            key: "cms:incr".to_string(),
            counts: vec![1, 2],
        });
        assert_eq!(cms_incr.counts.len(), 2);

        let cms_query = CmsQueryResponse::from(CmsQueryResult {
            key: "cms:query".to_string(),
            counts: vec![3],
        });
        assert_eq!(cms_query.counts, vec![3]);

        let cms_merge = CmsMergeResponse::from(CmsMergeResult {
            key: "cms:merge".to_string(),
            success: true,
        });
        assert!(cms_merge.success);

        let cms_info = CmsInfoResponse::from(CmsInfo {
            width: 10,
            depth: 2,
            count: 5,
        });
        assert_eq!(cms_info.count, 5);

        let topk_reserve = TopKReserveResponse::from(TopKReserveResult {
            key: "topk:reserve".to_string(),
            success: true,
        });
        assert!(topk_reserve.success);

        let topk_add = TopKAddResponse::from(TopKAddResult {
            key: "topk:add".to_string(),
            dropped: vec![None],
        });
        assert_eq!(topk_add.dropped.len(), 1);

        let topk_incr = TopKIncrByResponse::from(TopKIncrByResult {
            key: "topk:incr".to_string(),
            dropped: vec![Some("old".to_string())],
        });
        assert_eq!(topk_incr.dropped.len(), 1);

        let topk_query = TopKQueryResponse::from(TopKQueryResult {
            key: "topk:query".to_string(),
            results: vec![true, false],
        });
        assert_eq!(topk_query.results.len(), 2);

        let topk_count = TopKCountResponse::from(TopKCountResult {
            key: "topk:count".to_string(),
            counts: vec![10, 20],
        });
        assert_eq!(topk_count.counts.len(), 2);

        let topk_item = TopKListItem::from(TopKItem {
            item: "item".to_string(),
            count: 7,
        });
        assert_eq!(topk_item.count, 7);

        let topk_list = TopKListResponse::from(TopKListResult {
            key: "topk:list".to_string(),
            items: vec![TopKItem {
                item: "a".to_string(),
                count: 1,
            }],
        });
        assert_eq!(topk_list.items.len(), 1);

        let topk_info = TopKInfoResponse::from(TopKInfo {
            k: 10,
            width: 20,
            depth: 3,
            decay: 0.9,
        });
        assert_eq!(topk_info.width, 20);

        let pf_add = PfAddResponse::from(PfAddResult {
            key: "hll:add".to_string(),
            changed: true,
        });
        assert!(pf_add.changed);

        let pf_count = PfCountResponse::from(PfCountResult {
            keys: vec!["hll:1".to_string()],
            count: 42,
        });
        assert_eq!(pf_count.count, 42);

        let pf_merge = PfMergeResponse::from(PfMergeResult {
            dest_key: "hll:dest".to_string(),
            success: true,
        });
        assert!(pf_merge.success);
    }

    #[test]
    fn test_tdigest_create_request_defaults() {
        let json = r#"{}"#;
        let req: TDigestCreateRequest = serde_json::from_str(json).unwrap();
        assert!(req.compression.is_none());
    }

    #[test]
    fn test_tdigest_add_request() {
        let json = r#"{"values":[1.0,2.0,3.0]}"#;
        let req: TDigestAddRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.values.len(), 3);
    }

    #[test]
    fn test_tdigest_response_conversions() {
        let ack = TDigestAckResponse::from(TDigestAckResult {
            key: "td:test".to_string(),
            success: true,
        });
        assert!(ack.success);

        let values = TDigestValuesResponse::from(TDigestValuesResult {
            key: "td:test".to_string(),
            values: vec![Some(0.5), None],
        });
        assert_eq!(values.values.len(), 2);
        assert!(values.values[1].is_none());

        let ranks = TDigestRanksResponse::from(TDigestRanksResult {
            key: "td:test".to_string(),
            ranks: vec![0, -1, -2],
        });
        assert_eq!(ranks.ranks, vec![0, -1, -2]);

        let scalar = TDigestScalarResponse::from(TDigestScalarResult {
            key: "td:test".to_string(),
            value: Some(3.14),
        });
        assert_eq!(scalar.value, Some(3.14));

        let info = TDigestInfoResponse::from(TDigestInfo {
            compression: 100,
            capacity: 610,
            merged_nodes: 10,
            unmerged_nodes: 2,
            merged_weight: 100.0,
            unmerged_weight: 5.0,
            observations: 105,
            total_compressions: 1,
            memory_usage: 2048,
        });
        assert_eq!(info.compression, 100);
    }

    #[test]
    fn test_tdigest_values_response_serializes_null_for_nan() {
        let values = TDigestValuesResponse::from(TDigestValuesResult {
            key: "td:test".to_string(),
            values: vec![Some(0.5), None, Some(1.0)],
        });
        let json = serde_json::to_string(&values).unwrap();
        assert!(json.contains("null"));
        assert!(json.contains("0.5"));
    }

    #[test]
    fn test_tdigest_trimmed_mean_request() {
        let json = r#"{"low_cut_quantile":0.1,"high_cut_quantile":0.9}"#;
        let req: TDigestTrimmedMeanRequest = serde_json::from_str(json).unwrap();
        req.validate().unwrap();
        assert_eq!(req.low_cut_quantile, 0.1);
        assert_eq!(req.high_cut_quantile, 0.9);
    }

    #[test]
    fn test_tdigest_trimmed_mean_request_rejects_out_of_range() {
        let json = r#"{"low_cut_quantile":-0.1,"high_cut_quantile":0.9}"#;
        let req: TDigestTrimmedMeanRequest = serde_json::from_str(json).unwrap();
        assert!(req.validate().is_err());
    }
}
