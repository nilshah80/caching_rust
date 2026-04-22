//! Probabilistic Repository Trait
//!
//! Defines the interface for Count-Min Sketch, Top-K, and HyperLogLog operations.

use async_trait::async_trait;

use crate::domain::entities::{
    CmsIncrByResult, CmsInfo, CmsInitResult, CmsMergeResult, CmsQueryResult, PfAddResult,
    PfCountResult, PfMergeResult, TDigestAckResult, TDigestInfo, TDigestRanksResult,
    TDigestScalarResult, TDigestValuesResult, TopKAddResult, TopKCountResult, TopKIncrByResult,
    TopKInfo, TopKListResult, TopKQueryResult, TopKReserveResult,
};
use crate::domain::errors::CacheError;

/// Repository trait for probabilistic data structure operations
#[async_trait]
pub trait ProbabilisticRepository: Send + Sync {
    // ==================== Count-Min Sketch Operations ====================

    /// Initialize a Count-Min Sketch by dimensions (CMS.INITBYDIM)
    async fn cms_init_by_dim(
        &self,
        key: &str,
        width: u64,
        depth: u64,
    ) -> Result<CmsInitResult, CacheError>;

    /// Initialize a Count-Min Sketch by probability (CMS.INITBYPROB)
    async fn cms_init_by_prob(
        &self,
        key: &str,
        error: f64,
        probability: f64,
    ) -> Result<CmsInitResult, CacheError>;

    /// Increment item counts in a Count-Min Sketch (CMS.INCRBY)
    async fn cms_incr_by(
        &self,
        key: &str,
        items: Vec<(String, u64)>,
    ) -> Result<CmsIncrByResult, CacheError>;

    /// Query item counts in a Count-Min Sketch (CMS.QUERY)
    async fn cms_query(&self, key: &str, items: Vec<String>) -> Result<CmsQueryResult, CacheError>;

    /// Merge multiple Count-Min Sketches into a destination (CMS.MERGE)
    async fn cms_merge(
        &self,
        dest: &str,
        sources: Vec<String>,
        weights: Option<Vec<u64>>,
    ) -> Result<CmsMergeResult, CacheError>;

    /// Get information about a Count-Min Sketch (CMS.INFO)
    async fn cms_info(&self, key: &str) -> Result<CmsInfo, CacheError>;

    // ==================== Top-K Operations ====================

    /// Reserve a Top-K filter (TOPK.RESERVE)
    async fn topk_reserve(
        &self,
        key: &str,
        k: u64,
        width: Option<u64>,
        depth: Option<u64>,
        decay: Option<f64>,
    ) -> Result<TopKReserveResult, CacheError>;

    /// Add items to a Top-K filter (TOPK.ADD)
    async fn topk_add(&self, key: &str, items: Vec<String>) -> Result<TopKAddResult, CacheError>;

    /// Increment item counts in a Top-K filter (TOPK.INCRBY)
    async fn topk_incr_by(
        &self,
        key: &str,
        items: Vec<(String, u64)>,
    ) -> Result<TopKIncrByResult, CacheError>;

    /// Query if items are in the Top-K (TOPK.QUERY)
    async fn topk_query(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<TopKQueryResult, CacheError>;

    /// Get counts of items in a Top-K filter (TOPK.COUNT)
    async fn topk_count(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<TopKCountResult, CacheError>;

    /// List items in a Top-K filter (TOPK.LIST)
    async fn topk_list(&self, key: &str, with_count: bool) -> Result<TopKListResult, CacheError>;

    /// Get information about a Top-K filter (TOPK.INFO)
    async fn topk_info(&self, key: &str) -> Result<TopKInfo, CacheError>;

    // ==================== T-Digest Operations ====================

    /// Create a new t-digest sketch (TDIGEST.CREATE)
    async fn tdigest_create(
        &self,
        key: &str,
        compression: Option<u64>,
    ) -> Result<TDigestAckResult, CacheError>;

    /// Add one or more observations to a sketch (TDIGEST.ADD)
    async fn tdigest_add(
        &self,
        key: &str,
        values: Vec<f64>,
    ) -> Result<TDigestAckResult, CacheError>;

    /// Estimate quantiles (TDIGEST.QUANTILE)
    async fn tdigest_quantile(
        &self,
        key: &str,
        quantiles: Vec<f64>,
    ) -> Result<TDigestValuesResult, CacheError>;

    /// Estimate the CDF at one or more values (TDIGEST.CDF)
    async fn tdigest_cdf(
        &self,
        key: &str,
        values: Vec<f64>,
    ) -> Result<TDigestValuesResult, CacheError>;

    /// Estimate ranks (TDIGEST.RANK)
    async fn tdigest_rank(
        &self,
        key: &str,
        values: Vec<f64>,
    ) -> Result<TDigestRanksResult, CacheError>;

    /// Estimate reverse ranks (TDIGEST.REVRANK)
    async fn tdigest_revrank(
        &self,
        key: &str,
        values: Vec<f64>,
    ) -> Result<TDigestRanksResult, CacheError>;

    /// Lookup values by rank (TDIGEST.BYRANK)
    async fn tdigest_byrank(
        &self,
        key: &str,
        ranks: Vec<u64>,
    ) -> Result<TDigestValuesResult, CacheError>;

    /// Lookup values by reverse rank (TDIGEST.BYREVRANK)
    async fn tdigest_byrevrank(
        &self,
        key: &str,
        ranks: Vec<u64>,
    ) -> Result<TDigestValuesResult, CacheError>;

    /// Get the smallest observation (TDIGEST.MIN)
    async fn tdigest_min(&self, key: &str) -> Result<TDigestScalarResult, CacheError>;

    /// Get the largest observation (TDIGEST.MAX)
    async fn tdigest_max(&self, key: &str) -> Result<TDigestScalarResult, CacheError>;

    /// Get sketch metadata (TDIGEST.INFO)
    async fn tdigest_info(&self, key: &str) -> Result<TDigestInfo, CacheError>;

    /// Merge one or more sketches into `dest` (TDIGEST.MERGE)
    async fn tdigest_merge(
        &self,
        dest: &str,
        sources: Vec<String>,
        compression: Option<u64>,
        override_existing: bool,
    ) -> Result<TDigestAckResult, CacheError>;

    /// Empty an existing sketch while preserving its parameters (TDIGEST.RESET)
    async fn tdigest_reset(&self, key: &str) -> Result<TDigestAckResult, CacheError>;

    /// Compute a trimmed mean between two quantiles (TDIGEST.TRIMMED_MEAN)
    async fn tdigest_trimmed_mean(
        &self,
        key: &str,
        low_cut_quantile: f64,
        high_cut_quantile: f64,
    ) -> Result<TDigestScalarResult, CacheError>;

    // ==================== HyperLogLog Operations ====================

    /// Add elements to a HyperLogLog (PFADD)
    async fn pf_add(&self, key: &str, elements: Vec<String>) -> Result<PfAddResult, CacheError>;

    /// Count unique elements in HyperLogLog(s) (PFCOUNT)
    async fn pf_count(&self, keys: Vec<String>) -> Result<PfCountResult, CacheError>;

    /// Merge multiple HyperLogLogs into a destination (PFMERGE)
    async fn pf_merge(&self, dest: &str, sources: Vec<String>)
    -> Result<PfMergeResult, CacheError>;
}
