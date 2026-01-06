//! Probabilistic Service
//!
//! Application layer for Count-Min Sketch, Top-K, and HyperLogLog operations.

use std::sync::Arc;

use crate::domain::entities::{
    CmsIncrByResult, CmsInfo, CmsInitResult, CmsMergeResult, CmsQueryResult,
    PfAddResult, PfCountResult, PfMergeResult,
    TopKAddResult, TopKCountResult, TopKIncrByResult, TopKInfo, TopKListResult,
    TopKQueryResult, TopKReserveResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::ProbabilisticRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisProbabilisticRepository;

/// Service for probabilistic data structure operations
pub struct ProbabilisticService {
    repository: Arc<dyn ProbabilisticRepository>,
}

impl ProbabilisticService {
    /// Create a new ProbabilisticService with a Redis connection pool
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self {
            repository: Arc::new(RedisProbabilisticRepository::new(pool)),
        }
    }

    /// Create a new ProbabilisticService with a custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn ProbabilisticRepository>) -> Self {
        Self { repository }
    }

    // ==================== Count-Min Sketch Operations ====================

    /// Initialize a Count-Min Sketch by dimensions (CMS.INITBYDIM)
    pub async fn cms_init_by_dim(&self, key: &str, width: u64, depth: u64) -> Result<CmsInitResult, CacheError> {
        self.validate_key(key)?;
        self.validate_cms_dimensions(width, depth)?;
        self.repository.cms_init_by_dim(key, width, depth).await
    }

    /// Initialize a Count-Min Sketch by probability (CMS.INITBYPROB)
    pub async fn cms_init_by_prob(&self, key: &str, error: f64, probability: f64) -> Result<CmsInitResult, CacheError> {
        self.validate_key(key)?;
        self.validate_cms_probability(error, probability)?;
        self.repository.cms_init_by_prob(key, error, probability).await
    }

    /// Increment item counts in a Count-Min Sketch (CMS.INCRBY)
    pub async fn cms_incr_by(&self, key: &str, items: Vec<(String, u64)>) -> Result<CmsIncrByResult, CacheError> {
        self.validate_key(key)?;
        self.validate_increment_items(&items)?;
        self.repository.cms_incr_by(key, items).await
    }

    /// Query item counts in a Count-Min Sketch (CMS.QUERY)
    pub async fn cms_query(&self, key: &str, items: Vec<String>) -> Result<CmsQueryResult, CacheError> {
        self.validate_key(key)?;
        self.validate_items(&items)?;
        self.repository.cms_query(key, items).await
    }

    /// Merge multiple Count-Min Sketches (CMS.MERGE)
    pub async fn cms_merge(&self, dest: &str, sources: Vec<String>, weights: Option<Vec<u64>>) -> Result<CmsMergeResult, CacheError> {
        self.validate_key(dest)?;
        self.validate_keys(&sources)?;
        if let Some(ref w) = weights {
            if w.len() != sources.len() {
                return Err(CacheError::InvalidInput(
                    "Number of weights must match number of sources".to_string()
                ));
            }
        }
        self.repository.cms_merge(dest, sources, weights).await
    }

    /// Get information about a Count-Min Sketch (CMS.INFO)
    pub async fn cms_info(&self, key: &str) -> Result<CmsInfo, CacheError> {
        self.validate_key(key)?;
        self.repository.cms_info(key).await
    }

    // ==================== Top-K Operations ====================

    /// Reserve a Top-K filter (TOPK.RESERVE)
    pub async fn topk_reserve(&self, key: &str, k: u64, width: Option<u64>, depth: Option<u64>, decay: Option<f64>) -> Result<TopKReserveResult, CacheError> {
        self.validate_key(key)?;
        self.validate_topk_params(k, width, depth, decay)?;
        self.repository.topk_reserve(key, k, width, depth, decay).await
    }

    /// Add items to a Top-K filter (TOPK.ADD)
    pub async fn topk_add(&self, key: &str, items: Vec<String>) -> Result<TopKAddResult, CacheError> {
        self.validate_key(key)?;
        self.validate_items(&items)?;
        self.repository.topk_add(key, items).await
    }

    /// Increment item counts in a Top-K filter (TOPK.INCRBY)
    pub async fn topk_incr_by(&self, key: &str, items: Vec<(String, u64)>) -> Result<TopKIncrByResult, CacheError> {
        self.validate_key(key)?;
        self.validate_increment_items(&items)?;
        self.repository.topk_incr_by(key, items).await
    }

    /// Query if items are in the Top-K (TOPK.QUERY)
    pub async fn topk_query(&self, key: &str, items: Vec<String>) -> Result<TopKQueryResult, CacheError> {
        self.validate_key(key)?;
        self.validate_items(&items)?;
        self.repository.topk_query(key, items).await
    }

    /// Get counts of items in a Top-K filter (TOPK.COUNT)
    pub async fn topk_count(&self, key: &str, items: Vec<String>) -> Result<TopKCountResult, CacheError> {
        self.validate_key(key)?;
        self.validate_items(&items)?;
        self.repository.topk_count(key, items).await
    }

    /// List items in a Top-K filter (TOPK.LIST)
    pub async fn topk_list(&self, key: &str, with_count: bool) -> Result<TopKListResult, CacheError> {
        self.validate_key(key)?;
        self.repository.topk_list(key, with_count).await
    }

    /// Get information about a Top-K filter (TOPK.INFO)
    pub async fn topk_info(&self, key: &str) -> Result<TopKInfo, CacheError> {
        self.validate_key(key)?;
        self.repository.topk_info(key).await
    }

    // ==================== HyperLogLog Operations ====================

    /// Add elements to a HyperLogLog (PFADD)
    pub async fn pf_add(&self, key: &str, elements: Vec<String>) -> Result<PfAddResult, CacheError> {
        self.validate_key(key)?;
        self.validate_items(&elements)?;
        self.repository.pf_add(key, elements).await
    }

    /// Count unique elements in HyperLogLog(s) (PFCOUNT)
    pub async fn pf_count(&self, keys: Vec<String>) -> Result<PfCountResult, CacheError> {
        self.validate_keys(&keys)?;
        self.repository.pf_count(keys).await
    }

    /// Merge multiple HyperLogLogs (PFMERGE)
    pub async fn pf_merge(&self, dest: &str, sources: Vec<String>) -> Result<PfMergeResult, CacheError> {
        self.validate_key(dest)?;
        self.validate_keys(&sources)?;
        self.repository.pf_merge(dest, sources).await
    }

    // ==================== Validation Helpers ====================

    fn validate_key(&self, key: &str) -> Result<(), CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        Ok(())
    }

    fn validate_keys(&self, keys: &[String]) -> Result<(), CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        for key in keys {
            if key.is_empty() {
                return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
            }
        }
        Ok(())
    }

    fn validate_items(&self, items: &[String]) -> Result<(), CacheError> {
        if items.is_empty() {
            return Err(CacheError::InvalidInput("Items cannot be empty".to_string()));
        }
        Ok(())
    }

    fn validate_increment_items(&self, items: &[(String, u64)]) -> Result<(), CacheError> {
        if items.is_empty() {
            return Err(CacheError::InvalidInput("Items cannot be empty".to_string()));
        }
        for (item, _) in items {
            if item.is_empty() {
                return Err(CacheError::InvalidInput("Item name cannot be empty".to_string()));
            }
        }
        Ok(())
    }

    fn validate_cms_dimensions(&self, width: u64, depth: u64) -> Result<(), CacheError> {
        if width == 0 {
            return Err(CacheError::InvalidInput("Width must be greater than 0".to_string()));
        }
        if depth == 0 {
            return Err(CacheError::InvalidInput("Depth must be greater than 0".to_string()));
        }
        Ok(())
    }

    fn validate_cms_probability(&self, error: f64, probability: f64) -> Result<(), CacheError> {
        if error <= 0.0 || error >= 1.0 {
            return Err(CacheError::InvalidInput("Error rate must be between 0 and 1 (exclusive)".to_string()));
        }
        if probability <= 0.0 || probability >= 1.0 {
            return Err(CacheError::InvalidInput("Probability must be between 0 and 1 (exclusive)".to_string()));
        }
        Ok(())
    }

    fn validate_topk_params(&self, k: u64, width: Option<u64>, depth: Option<u64>, decay: Option<f64>) -> Result<(), CacheError> {
        if k == 0 {
            return Err(CacheError::InvalidInput("K must be greater than 0".to_string()));
        }

        // RedisBloom TOPK.RESERVE requires width, depth, and decay as an all-or-nothing group
        let has_width = width.is_some();
        let has_depth = depth.is_some();
        let has_decay = decay.is_some();

        if has_width || has_depth || has_decay {
            // If any optional param is provided, all must be provided
            if !has_width || !has_depth || !has_decay {
                return Err(CacheError::InvalidInput(
                    "TOPK.RESERVE optional parameters (width, depth, decay) must be provided together or not at all".to_string()
                ));
            }

            // Validate individual values
            let w = width.unwrap();
            let d = depth.unwrap();
            let decay_val = decay.unwrap();

            if w == 0 {
                return Err(CacheError::InvalidInput("Width must be greater than 0".to_string()));
            }
            if d == 0 {
                return Err(CacheError::InvalidInput("Depth must be greater than 0".to_string()));
            }
            if decay_val <= 0.0 || decay_val > 1.0 {
                return Err(CacheError::InvalidInput("Decay must be between 0 (exclusive) and 1 (inclusive)".to_string()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock repository for testing
    struct MockProbabilisticRepository;

    #[async_trait::async_trait]
    impl ProbabilisticRepository for MockProbabilisticRepository {
        async fn cms_init_by_dim(&self, key: &str, _width: u64, _depth: u64) -> Result<CmsInitResult, CacheError> {
            Ok(CmsInitResult { key: key.to_string(), success: true })
        }
        async fn cms_init_by_prob(&self, key: &str, _error: f64, _probability: f64) -> Result<CmsInitResult, CacheError> {
            Ok(CmsInitResult { key: key.to_string(), success: true })
        }
        async fn cms_incr_by(&self, key: &str, items: Vec<(String, u64)>) -> Result<CmsIncrByResult, CacheError> {
            Ok(CmsIncrByResult { key: key.to_string(), counts: items.iter().map(|(_, c)| *c).collect() })
        }
        async fn cms_query(&self, key: &str, items: Vec<String>) -> Result<CmsQueryResult, CacheError> {
            Ok(CmsQueryResult { key: key.to_string(), counts: vec![0; items.len()] })
        }
        async fn cms_merge(&self, dest: &str, _sources: Vec<String>, _weights: Option<Vec<u64>>) -> Result<CmsMergeResult, CacheError> {
            Ok(CmsMergeResult { key: dest.to_string(), success: true })
        }
        async fn cms_info(&self, _key: &str) -> Result<CmsInfo, CacheError> {
            Ok(CmsInfo { width: 2000, depth: 5, count: 0 })
        }
        async fn topk_reserve(&self, key: &str, _k: u64, _width: Option<u64>, _depth: Option<u64>, _decay: Option<f64>) -> Result<TopKReserveResult, CacheError> {
            Ok(TopKReserveResult { key: key.to_string(), success: true })
        }
        async fn topk_add(&self, key: &str, items: Vec<String>) -> Result<TopKAddResult, CacheError> {
            Ok(TopKAddResult { key: key.to_string(), dropped: vec![None; items.len()] })
        }
        async fn topk_incr_by(&self, key: &str, items: Vec<(String, u64)>) -> Result<TopKIncrByResult, CacheError> {
            Ok(TopKIncrByResult { key: key.to_string(), dropped: vec![None; items.len()] })
        }
        async fn topk_query(&self, key: &str, items: Vec<String>) -> Result<TopKQueryResult, CacheError> {
            Ok(TopKQueryResult { key: key.to_string(), results: vec![false; items.len()] })
        }
        async fn topk_count(&self, key: &str, items: Vec<String>) -> Result<TopKCountResult, CacheError> {
            Ok(TopKCountResult { key: key.to_string(), counts: vec![0; items.len()] })
        }
        async fn topk_list(&self, key: &str, _with_count: bool) -> Result<TopKListResult, CacheError> {
            Ok(TopKListResult { key: key.to_string(), items: vec![] })
        }
        async fn topk_info(&self, _key: &str) -> Result<TopKInfo, CacheError> {
            Ok(TopKInfo { k: 10, width: 2000, depth: 7, decay: 0.9 })
        }
        async fn pf_add(&self, key: &str, _elements: Vec<String>) -> Result<PfAddResult, CacheError> {
            Ok(PfAddResult { key: key.to_string(), changed: true })
        }
        async fn pf_count(&self, keys: Vec<String>) -> Result<PfCountResult, CacheError> {
            Ok(PfCountResult { keys, count: 0 })
        }
        async fn pf_merge(&self, dest: &str, _sources: Vec<String>) -> Result<PfMergeResult, CacheError> {
            Ok(PfMergeResult { dest_key: dest.to_string(), success: true })
        }
    }

    fn create_test_service() -> ProbabilisticService {
        ProbabilisticService::new_with_repository(Arc::new(MockProbabilisticRepository))
    }

    #[tokio::test]
    async fn test_cms_init_by_dim() {
        let service = create_test_service();
        let result = service.cms_init_by_dim("cms:test", 2000, 5).await.unwrap();
        assert!(result.success);
        assert_eq!(result.key, "cms:test");
    }

    #[tokio::test]
    async fn test_cms_init_by_dim_invalid_width() {
        let service = create_test_service();
        let result = service.cms_init_by_dim("cms:test", 0, 5).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_cms_init_by_prob() {
        let service = create_test_service();
        let result = service.cms_init_by_prob("cms:test", 0.01, 0.001).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_cms_init_by_prob_invalid() {
        let service = create_test_service();
        let result = service.cms_init_by_prob("cms:test", 1.5, 0.001).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_topk_reserve() {
        let service = create_test_service();
        let result = service.topk_reserve("topk:test", 10, None, None, None).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_topk_reserve_invalid_k() {
        let service = create_test_service();
        let result = service.topk_reserve("topk:test", 0, None, None, None).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_topk_reserve_with_all_params() {
        let service = create_test_service();
        // When all optional params are provided, should succeed
        let result = service.topk_reserve("topk:test", 10, Some(2000), Some(7), Some(0.9)).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_topk_reserve_partial_params_width_only() {
        let service = create_test_service();
        // Providing only width should fail (all-or-nothing)
        let result = service.topk_reserve("topk:test", 10, Some(2000), None, None).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(msg)) if msg.contains("together or not at all")));
    }

    #[tokio::test]
    async fn test_topk_reserve_partial_params_width_depth() {
        let service = create_test_service();
        // Providing width and depth but not decay should fail
        let result = service.topk_reserve("topk:test", 10, Some(2000), Some(7), None).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(msg)) if msg.contains("together or not at all")));
    }

    #[tokio::test]
    async fn test_topk_reserve_partial_params_decay_only() {
        let service = create_test_service();
        // Providing only decay should fail
        let result = service.topk_reserve("topk:test", 10, None, None, Some(0.9)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(msg)) if msg.contains("together or not at all")));
    }

    #[tokio::test]
    async fn test_topk_reserve_invalid_width() {
        let service = create_test_service();
        let result = service.topk_reserve("topk:test", 10, Some(0), Some(7), Some(0.9)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(msg)) if msg.contains("Width")));
    }

    #[tokio::test]
    async fn test_topk_reserve_invalid_depth() {
        let service = create_test_service();
        let result = service.topk_reserve("topk:test", 10, Some(2000), Some(0), Some(0.9)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(msg)) if msg.contains("Depth")));
    }

    #[tokio::test]
    async fn test_topk_reserve_invalid_decay() {
        let service = create_test_service();
        let result = service.topk_reserve("topk:test", 10, Some(2000), Some(7), Some(1.5)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(msg)) if msg.contains("Decay")));
    }

    #[tokio::test]
    async fn test_cms_incr_by_and_query() {
        let service = create_test_service();
        let items = vec![("item1".to_string(), 2), ("item2".to_string(), 3)];
        let incr_result = service.cms_incr_by("cms:test", items.clone()).await.unwrap();
        assert_eq!(incr_result.counts, vec![2, 3]);

        let query_result = service
            .cms_query("cms:test", vec!["item1".to_string(), "item2".to_string()])
            .await
            .unwrap();
        assert_eq!(query_result.counts.len(), 2);
    }

    #[tokio::test]
    async fn test_cms_merge_and_info() {
        let service = create_test_service();
        let merge_result = service
            .cms_merge("cms:dest", vec!["cms:src1".to_string()], None)
            .await
            .unwrap();
        assert!(merge_result.success);

        let info_result = service.cms_info("cms:test").await.unwrap();
        assert_eq!(info_result.width, 2000);
    }

    #[tokio::test]
    async fn test_topk_operations() {
        let service = create_test_service();

        let add_result = service
            .topk_add("topk:test", vec!["a".to_string(), "b".to_string()])
            .await
            .unwrap();
        assert_eq!(add_result.dropped.len(), 2);

        let incr_result = service
            .topk_incr_by("topk:test", vec![("a".to_string(), 2)])
            .await
            .unwrap();
        assert_eq!(incr_result.dropped.len(), 1);

        let query_result = service
            .topk_query("topk:test", vec!["a".to_string()])
            .await
            .unwrap();
        assert_eq!(query_result.results.len(), 1);

        let count_result = service
            .topk_count("topk:test", vec!["a".to_string(), "b".to_string()])
            .await
            .unwrap();
        assert_eq!(count_result.counts.len(), 2);

        let list_result = service.topk_list("topk:test", true).await.unwrap();
        assert!(list_result.items.is_empty());

        let info_result = service.topk_info("topk:test").await.unwrap();
        assert_eq!(info_result.k, 10);
    }

    #[tokio::test]
    async fn test_pf_merge() {
        let service = create_test_service();
        let result = service
            .pf_merge("hll:dest", vec!["hll:1".to_string(), "hll:2".to_string()])
            .await
            .unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_pf_count_empty_keys() {
        let service = create_test_service();
        let result = service.pf_count(vec![]).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_pf_count_empty_key_value() {
        let service = create_test_service();
        let result = service.pf_count(vec!["".to_string()]).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_cms_incr_by_empty_items() {
        let service = create_test_service();
        let result = service.cms_incr_by("cms:test", vec![]).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_cms_incr_by_empty_item_name() {
        let service = create_test_service();
        let result = service
            .cms_incr_by("cms:test", vec![("".to_string(), 1)])
            .await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_cms_init_by_dim_invalid_depth() {
        let service = create_test_service();
        let result = service.cms_init_by_dim("cms:test", 10, 0).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_cms_init_by_prob_invalid_probability() {
        let service = create_test_service();
        let result = service.cms_init_by_prob("cms:test", 0.01, 1.5).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_pf_add() {
        let service = create_test_service();
        let result = service.pf_add("hll:test", vec!["a".to_string(), "b".to_string()]).await.unwrap();
        assert!(result.changed);
    }

    #[tokio::test]
    async fn test_pf_count() {
        let service = create_test_service();
        let result = service.pf_count(vec!["hll:test".to_string()]).await.unwrap();
        assert_eq!(result.count, 0);
    }

    #[tokio::test]
    async fn test_cms_merge_weights_mismatch() {
        let service = create_test_service();
        let result = service.cms_merge(
            "dest",
            vec!["src1".to_string(), "src2".to_string()],
            Some(vec![1]), // Only 1 weight for 2 sources
        ).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_empty_key_validation() {
        let service = create_test_service();
        let result = service.cms_info("").await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_empty_items_validation() {
        let service = create_test_service();
        let result = service.pf_add("hll:test", vec![]).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }
}
