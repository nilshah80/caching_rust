//! Probabilistic Data Structure Domain Entities
//!
//! Domain types for Count-Min Sketch, Top-K, and HyperLogLog operations.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ==================== Count-Min Sketch Types ====================

/// Result of CMS.INITBYDIM operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CmsInitResult {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

/// Result of CMS.INCRBY operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CmsIncrByResult {
    /// Key name
    pub key: String,
    /// New counts for each item after increment
    pub counts: Vec<u64>,
}

/// Result of CMS.QUERY operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CmsQueryResult {
    /// Key name
    pub key: String,
    /// Estimated counts for each queried item
    pub counts: Vec<u64>,
}

/// Result of CMS.MERGE operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CmsMergeResult {
    /// Destination key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

/// Result of CMS.INFO operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CmsInfo {
    /// Width (number of counters per row)
    pub width: u64,
    /// Depth (number of hash functions)
    pub depth: u64,
    /// Total count of all increments
    pub count: u64,
}

// ==================== Top-K Types ====================

/// Result of TOPK.RESERVE operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopKReserveResult {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

/// Result of TOPK.ADD operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopKAddResult {
    /// Key name
    pub key: String,
    /// Items that were dropped from top-k (None if item didn't cause a drop)
    pub dropped: Vec<Option<String>>,
}

/// Result of TOPK.INCRBY operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopKIncrByResult {
    /// Key name
    pub key: String,
    /// Items that were dropped from top-k (None if increment didn't cause a drop)
    pub dropped: Vec<Option<String>>,
}

/// Result of TOPK.QUERY operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopKQueryResult {
    /// Key name
    pub key: String,
    /// Whether each item is in the top-k
    pub results: Vec<bool>,
}

/// Result of TOPK.COUNT operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopKCountResult {
    /// Key name
    pub key: String,
    /// Estimated counts for each queried item
    pub counts: Vec<u64>,
}

/// A top-k item with its count
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopKItem {
    /// Item value
    pub item: String,
    /// Estimated count
    pub count: u64,
}

/// Result of TOPK.LIST operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopKListResult {
    /// Key name
    pub key: String,
    /// Items in the top-k (with optional counts if WITHCOUNT was used)
    pub items: Vec<TopKItem>,
}

/// Result of TOPK.INFO operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopKInfo {
    /// Number of top items to track
    pub k: u64,
    /// Width of the underlying Count-Min Sketch
    pub width: u64,
    /// Depth of the underlying Count-Min Sketch
    pub depth: u64,
    /// Decay constant
    pub decay: f64,
}

// ==================== T-Digest Types ====================

/// Simple acknowledgement result for T-Digest commands that only return OK.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TDigestAckResult {
    /// Key name
    pub key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

/// Result of TDIGEST.QUANTILE / TDIGEST.BYRANK / TDIGEST.BYREVRANK.
/// Each Redis Bloom build returns `nan` for empty sketches; we preserve that as `None`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TDigestValuesResult {
    /// Key name
    pub key: String,
    /// Values returned in the same order as the input (None = nan)
    pub values: Vec<Option<f64>>,
}

/// Result of TDIGEST.RANK / TDIGEST.REVRANK. Redis returns -2 when key is empty
/// and -1 when the value is below/above all observations; we pass the raw signed
/// integers through so callers can distinguish those cases.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TDigestRanksResult {
    /// Key name
    pub key: String,
    /// Rank values in the same order as the input
    pub ranks: Vec<i64>,
}

/// Result of TDIGEST.MIN / TDIGEST.MAX / TDIGEST.TRIMMED_MEAN.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TDigestScalarResult {
    /// Key name
    pub key: String,
    /// Scalar value (None = nan, e.g. empty sketch)
    pub value: Option<f64>,
}

/// Result of TDIGEST.INFO parsed into structured fields.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TDigestInfo {
    /// Compression parameter (tracks accuracy/memory trade-off)
    pub compression: u64,
    /// Maximum capacity of the sketch
    pub capacity: u64,
    /// Number of merged nodes
    pub merged_nodes: u64,
    /// Number of unmerged nodes
    pub unmerged_nodes: u64,
    /// Total weight of merged nodes
    pub merged_weight: f64,
    /// Total weight of unmerged nodes
    pub unmerged_weight: f64,
    /// Count of observations added
    pub observations: u64,
    /// Number of compressions performed
    pub total_compressions: u64,
    /// Estimated memory footprint in bytes
    pub memory_usage: u64,
}

// ==================== HyperLogLog Types ====================

/// Result of PFADD operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PfAddResult {
    /// Key name
    pub key: String,
    /// Whether the cardinality estimate changed (1) or not (0)
    pub changed: bool,
}

/// Result of PFCOUNT operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PfCountResult {
    /// Key name(s) counted
    pub keys: Vec<String>,
    /// Estimated cardinality
    pub count: u64,
}

/// Result of PFMERGE operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PfMergeResult {
    /// Destination key name
    pub dest_key: String,
    /// Whether the operation succeeded
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cms_info_serialization() {
        let info = CmsInfo {
            width: 2000,
            depth: 5,
            count: 1000,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"width\":2000"));
        assert!(json.contains("\"depth\":5"));
        assert!(json.contains("\"count\":1000"));
    }

    #[test]
    fn test_topk_info_serialization() {
        let info = TopKInfo {
            k: 10,
            width: 2000,
            depth: 7,
            decay: 0.9,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"k\":10"));
        assert!(json.contains("\"decay\":0.9"));
    }

    #[test]
    fn test_topk_item() {
        let item = TopKItem {
            item: "test".to_string(),
            count: 42,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"item\":\"test\""));
        assert!(json.contains("\"count\":42"));
    }

    #[test]
    fn test_pf_add_result() {
        let result = PfAddResult {
            key: "hll:test".to_string(),
            changed: true,
        };
        assert!(result.changed);
    }

    #[test]
    fn test_pf_count_result() {
        let result = PfCountResult {
            keys: vec!["hll:1".to_string(), "hll:2".to_string()],
            count: 1000,
        };
        assert_eq!(result.keys.len(), 2);
        assert_eq!(result.count, 1000);
    }

    #[test]
    fn test_cms_query_result() {
        let result = CmsQueryResult {
            key: "cms:test".to_string(),
            counts: vec![10, 20, 30],
        };
        assert_eq!(result.counts.len(), 3);
    }

    #[test]
    fn test_tdigest_info_serialization() {
        let info = TDigestInfo {
            compression: 100,
            capacity: 610,
            merged_nodes: 10,
            unmerged_nodes: 2,
            merged_weight: 100.0,
            unmerged_weight: 5.0,
            observations: 105,
            total_compressions: 1,
            memory_usage: 2048,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"compression\":100"));
        assert!(json.contains("\"memory_usage\":2048"));
    }

    #[test]
    fn test_tdigest_values_result() {
        let result = TDigestValuesResult {
            key: "td:test".to_string(),
            values: vec![Some(0.5), None, Some(1.0)],
        };
        assert_eq!(result.values.len(), 3);
        assert_eq!(result.values[1], None);
    }

    #[test]
    fn test_tdigest_ranks_result() {
        let result = TDigestRanksResult {
            key: "td:test".to_string(),
            ranks: vec![0, -1, -2],
        };
        assert_eq!(result.ranks, vec![0, -1, -2]);
    }

    #[test]
    fn test_tdigest_scalar_result() {
        let result = TDigestScalarResult {
            key: "td:test".to_string(),
            value: Some(3.14),
        };
        assert_eq!(result.value, Some(3.14));
    }

    #[test]
    fn test_tdigest_ack_result() {
        let result = TDigestAckResult {
            key: "td:test".to_string(),
            success: true,
        };
        assert!(result.success);
    }

    #[test]
    fn test_topk_add_result() {
        let result = TopKAddResult {
            key: "topk:test".to_string(),
            dropped: vec![None, Some("old_item".to_string()), None],
        };
        assert_eq!(result.dropped.len(), 3);
        assert!(result.dropped[0].is_none());
        assert_eq!(result.dropped[1], Some("old_item".to_string()));
    }
}
