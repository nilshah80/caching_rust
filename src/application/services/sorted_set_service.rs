//! Sorted Set Service
//!
//! Business logic for Redis sorted set (ZSET) operations.

use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    LexRange, ScoreRange, ScoredMember, SortedSetRepository, ZAddOptions, ZAddResult,
    ZPopDirection, ZPopResult, ZRangeOptions, ZScanResult, ZSetAlgebraOptions,
};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisSortedSetRepository;
use crate::shared::blocking::BlockingTimeoutEnforcer;

/// Service for sorted set operations
pub struct SortedSetService {
    repository: Arc<dyn SortedSetRepository>,
    timeout_enforcer: BlockingTimeoutEnforcer,
}

impl SortedSetService {
    /// Create a new SortedSetService with default Redis repository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisSortedSetRepository::new(pool)))
    }

    /// Create a new SortedSetService with custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn SortedSetRepository>) -> Self {
        Self {
            repository,
            timeout_enforcer: BlockingTimeoutEnforcer::new(),
        }
    }

    /// Enforce timeout bounds for blocking operations
    fn enforce_timeout(&self, requested: f64) -> f64 {
        self.timeout_enforcer.enforce_secs_f64(requested)
    }

    // ========== Basic operations ==========

    /// ZADD - Add members with scores to a sorted set
    pub async fn zadd(
        &self,
        key: &str,
        members: Vec<ScoredMember>,
        options: Option<ZAddOptions>,
    ) -> Result<ZAddResult, CacheError> {
        if members.is_empty() {
            return Err(CacheError::InvalidInput(
                "Members cannot be empty".to_string(),
            ));
        }

        // Validate options - NX and XX are mutually exclusive
        if let Some(ref opts) = options {
            if opts.nx && opts.xx {
                return Err(CacheError::InvalidInput(
                    "NX and XX options are mutually exclusive".to_string(),
                ));
            }
            // GT and LT are mutually exclusive
            if opts.gt && opts.lt {
                return Err(CacheError::InvalidInput(
                    "GT and LT options are mutually exclusive".to_string(),
                ));
            }
            // NX cannot be combined with GT or LT
            if opts.nx && (opts.gt || opts.lt) {
                return Err(CacheError::InvalidInput(
                    "NX cannot be combined with GT or LT".to_string(),
                ));
            }
        }

        self.repository.zadd(key, &members, options).await
    }

    /// ZADD with INCR option - Increment the score of a member
    pub async fn zadd_incr(
        &self,
        key: &str,
        member: &str,
        score: f64,
        options: Option<ZAddOptions>,
    ) -> Result<Option<f64>, CacheError> {
        self.repository.zadd_incr(key, member, score, options).await
    }

    /// ZREM - Remove members from a sorted set
    pub async fn zrem(&self, key: &str, members: Vec<String>) -> Result<i64, CacheError> {
        if members.is_empty() {
            return Err(CacheError::InvalidInput(
                "Members cannot be empty".to_string(),
            ));
        }
        self.repository.zrem(key, &members).await
    }

    /// ZSCORE - Get the score of a member
    pub async fn zscore(&self, key: &str, member: &str) -> Result<Option<f64>, CacheError> {
        self.repository.zscore(key, member).await
    }

    /// ZMSCORE - Get scores of multiple members
    pub async fn zmscore(
        &self,
        key: &str,
        members: Vec<String>,
    ) -> Result<Vec<Option<f64>>, CacheError> {
        if members.is_empty() {
            return Err(CacheError::InvalidInput(
                "Members cannot be empty".to_string(),
            ));
        }
        self.repository.zmscore(key, &members).await
    }

    /// ZINCRBY - Increment the score of a member
    pub async fn zincrby(
        &self,
        key: &str,
        member: &str,
        increment: f64,
    ) -> Result<f64, CacheError> {
        self.repository.zincrby(key, member, increment).await
    }

    /// ZCARD - Get the number of members in a sorted set
    pub async fn zcard(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.zcard(key).await
    }

    /// ZCOUNT - Count members with scores in a range
    pub async fn zcount(&self, key: &str, range: ScoreRange) -> Result<i64, CacheError> {
        self.repository.zcount(key, &range).await
    }

    /// ZLEXCOUNT - Count members in a lexicographical range
    pub async fn zlexcount(&self, key: &str, range: LexRange) -> Result<i64, CacheError> {
        self.repository.zlexcount(key, &range).await
    }

    // ========== Rank operations ==========

    /// ZRANK - Get the rank of a member (0-based, lowest score first)
    pub async fn zrank(&self, key: &str, member: &str) -> Result<Option<i64>, CacheError> {
        self.repository.zrank(key, member).await
    }

    /// ZREVRANK - Get the reverse rank of a member (0-based, highest score first)
    pub async fn zrevrank(&self, key: &str, member: &str) -> Result<Option<i64>, CacheError> {
        self.repository.zrevrank(key, member).await
    }

    // ========== Range operations ==========

    /// ZRANGE - Get members in a range by index
    pub async fn zrange(
        &self,
        key: &str,
        start: i64,
        stop: i64,
        options: Option<ZRangeOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        self.repository.zrange(key, start, stop, options).await
    }

    /// ZRANGEBYSCORE - Get members with scores in a range
    pub async fn zrangebyscore(
        &self,
        key: &str,
        range: ScoreRange,
        options: Option<ZRangeOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        self.repository.zrangebyscore(key, &range, options).await
    }

    /// ZRANGEBYLEX - Get members in a lexicographical range
    pub async fn zrangebylex(
        &self,
        key: &str,
        range: LexRange,
        options: Option<ZRangeOptions>,
    ) -> Result<Vec<String>, CacheError> {
        self.repository.zrangebylex(key, &range, options).await
    }

    /// ZRANGESTORE - Store a range in a new key
    pub async fn zrangestore(
        &self,
        destination: &str,
        source: &str,
        start: i64,
        stop: i64,
        options: Option<ZRangeOptions>,
    ) -> Result<i64, CacheError> {
        self.repository
            .zrangestore(destination, source, start, stop, options)
            .await
    }

    // ========== Remove range operations ==========

    /// ZREMRANGEBYRANK - Remove members by rank range
    pub async fn zremrangebyrank(
        &self,
        key: &str,
        start: i64,
        stop: i64,
    ) -> Result<i64, CacheError> {
        self.repository.zremrangebyrank(key, start, stop).await
    }

    /// ZREMRANGEBYSCORE - Remove members by score range
    pub async fn zremrangebyscore(&self, key: &str, range: ScoreRange) -> Result<i64, CacheError> {
        self.repository.zremrangebyscore(key, &range).await
    }

    /// ZREMRANGEBYLEX - Remove members by lexicographical range
    pub async fn zremrangebylex(&self, key: &str, range: LexRange) -> Result<i64, CacheError> {
        self.repository.zremrangebylex(key, &range).await
    }

    // ========== Pop operations ==========

    /// ZPOPMIN - Remove and return members with lowest scores
    pub async fn zpopmin(
        &self,
        key: &str,
        count: Option<i64>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        self.repository.zpopmin(key, count).await
    }

    /// ZPOPMAX - Remove and return members with highest scores
    pub async fn zpopmax(
        &self,
        key: &str,
        count: Option<i64>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        self.repository.zpopmax(key, count).await
    }

    /// BZPOPMIN - Blocking pop of member with lowest score
    pub async fn bzpopmin(
        &self,
        keys: Vec<String>,
        timeout_seconds: u32,
    ) -> Result<Option<ZPopResult>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        let timeout = self.enforce_timeout(timeout_seconds as f64);
        self.repository.bzpopmin(&keys, timeout).await
    }

    /// BZPOPMAX - Blocking pop of member with highest score
    pub async fn bzpopmax(
        &self,
        keys: Vec<String>,
        timeout_seconds: u32,
    ) -> Result<Option<ZPopResult>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        let timeout = self.enforce_timeout(timeout_seconds as f64);
        self.repository.bzpopmax(&keys, timeout).await
    }

    /// ZMPOP - Pop members from multiple keys
    pub async fn zmpop(
        &self,
        keys: Vec<String>,
        direction: ZPopDirection,
        count: Option<i64>,
    ) -> Result<Option<ZPopResult>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.zmpop(&keys, direction, count).await
    }

    /// BZMPOP - Blocking pop from multiple keys
    pub async fn bzmpop(
        &self,
        keys: Vec<String>,
        direction: ZPopDirection,
        timeout_seconds: u32,
        count: Option<i64>,
    ) -> Result<Option<ZPopResult>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        let timeout = self.enforce_timeout(timeout_seconds as f64);
        self.repository
            .bzmpop(&keys, direction, timeout, count)
            .await
    }

    // ========== Random access ==========

    /// ZRANDMEMBER - Get random members
    pub async fn zrandmember(
        &self,
        key: &str,
        count: Option<i64>,
        with_scores: bool,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        self.repository.zrandmember(key, count, with_scores).await
    }

    // ========== Set algebra operations ==========

    /// ZUNION - Get the union of multiple sorted sets
    pub async fn zunion(
        &self,
        keys: Vec<String>,
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.validate_algebra_options(&keys, &options)?;
        self.repository.zunion(&keys, options).await
    }

    /// ZUNIONSTORE - Store the union of multiple sorted sets
    pub async fn zunionstore(
        &self,
        destination: &str,
        keys: Vec<String>,
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<i64, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.validate_algebra_options(&keys, &options)?;
        self.repository
            .zunionstore(destination, &keys, options)
            .await
    }

    /// ZINTER - Get the intersection of multiple sorted sets
    pub async fn zinter(
        &self,
        keys: Vec<String>,
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.validate_algebra_options(&keys, &options)?;
        self.repository.zinter(&keys, options).await
    }

    /// ZINTERSTORE - Store the intersection of multiple sorted sets
    pub async fn zinterstore(
        &self,
        destination: &str,
        keys: Vec<String>,
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<i64, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.validate_algebra_options(&keys, &options)?;
        self.repository
            .zinterstore(destination, &keys, options)
            .await
    }

    /// ZINTERCARD - Get the cardinality of the intersection
    pub async fn zintercard(
        &self,
        keys: Vec<String>,
        limit: Option<u64>,
    ) -> Result<i64, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.zintercard(&keys, limit).await
    }

    /// ZDIFF - Get the difference of sorted sets
    pub async fn zdiff(
        &self,
        keys: Vec<String>,
        with_scores: bool,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.zdiff(&keys, with_scores).await
    }

    /// ZDIFFSTORE - Store the difference of sorted sets
    pub async fn zdiffstore(
        &self,
        destination: &str,
        keys: Vec<String>,
    ) -> Result<i64, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.zdiffstore(destination, &keys).await
    }

    // ========== Scan operation ==========

    /// ZSCAN - Incrementally iterate sorted set members
    pub async fn zscan(
        &self,
        key: &str,
        cursor: u64,
        pattern: Option<&str>,
        count: Option<u64>,
    ) -> Result<ZScanResult, CacheError> {
        self.repository.zscan(key, cursor, pattern, count).await
    }

    // ========== Helper methods ==========

    /// Validate algebra options - weights must match number of keys
    fn validate_algebra_options(
        &self,
        keys: &[String],
        options: &Option<ZSetAlgebraOptions>,
    ) -> Result<(), CacheError> {
        if let Some(opts) = options
            && let Some(weights) = &opts.weights
            && weights.len() != keys.len()
        {
            return Err(CacheError::InvalidInput(format!(
                "Number of weights ({}) must match number of keys ({})",
                weights.len(),
                keys.len()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::ZAggregate;
    use crate::test_support::MockSortedSetRepository;

    #[tokio::test]
    async fn test_zadd_validation() {
        let repo = Arc::new(MockSortedSetRepository::new());
        let service = SortedSetService::new_with_repository(repo);

        // Empty members should fail
        let err = service.zadd("zset", Vec::new(), None).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // NX and XX together should fail
        let options = ZAddOptions {
            nx: true,
            xx: true,
            ..Default::default()
        };
        let err = service
            .zadd(
                "zset",
                vec![ScoredMember::new("a".to_string(), 1.0)],
                Some(options),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // GT and LT together should fail
        let options = ZAddOptions {
            gt: true,
            lt: true,
            ..Default::default()
        };
        let err = service
            .zadd(
                "zset",
                vec![ScoredMember::new("a".to_string(), 1.0)],
                Some(options),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // NX with GT should fail
        let options = ZAddOptions {
            nx: true,
            gt: true,
            ..Default::default()
        };
        let err = service
            .zadd(
                "zset",
                vec![ScoredMember::new("a".to_string(), 1.0)],
                Some(options),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_zrem_validation() {
        let repo = Arc::new(MockSortedSetRepository::new());
        let service = SortedSetService::new_with_repository(repo);

        let err = service.zrem("zset", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_zmscore_validation() {
        let repo = Arc::new(MockSortedSetRepository::new());
        let service = SortedSetService::new_with_repository(repo);

        let err = service.zmscore("zset", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_blocking_operations_validation() {
        let repo = Arc::new(MockSortedSetRepository::new());
        let service = SortedSetService::new_with_repository(repo);

        // Empty keys
        let err = service.bzpopmin(Vec::new(), 1).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.bzpopmax(Vec::new(), 1).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .bzmpop(Vec::new(), ZPopDirection::Min, 1, None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_zmpop_validation() {
        let repo = Arc::new(MockSortedSetRepository::new());
        let service = SortedSetService::new_with_repository(repo);

        let err = service
            .zmpop(Vec::new(), ZPopDirection::Min, None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_algebra_operations_validation() {
        let repo = Arc::new(MockSortedSetRepository::new());
        let service = SortedSetService::new_with_repository(repo);

        // Empty keys
        let err = service.zunion(Vec::new(), None).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .zunionstore("dest", Vec::new(), None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.zinter(Vec::new(), None).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .zinterstore("dest", Vec::new(), None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.zintercard(Vec::new(), None).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.zdiff(Vec::new(), false).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.zdiffstore("dest", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Weights mismatch
        let options = ZSetAlgebraOptions {
            weights: Some(vec![1.0]), // Only 1 weight for 2 keys
            ..Default::default()
        };
        let err = service
            .zunion(
                vec!["key1".to_string(), "key2".to_string()],
                Some(options.clone()),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .zunionstore(
                "dest",
                vec!["key1".to_string(), "key2".to_string()],
                Some(options.clone()),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .zinter(
                vec!["key1".to_string(), "key2".to_string()],
                Some(options.clone()),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .zinterstore(
                "dest",
                vec!["key1".to_string(), "key2".to_string()],
                Some(options),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_sorted_set_operations() {
        let repo = Arc::new(MockSortedSetRepository::new());
        let service = SortedSetService::new_with_repository(repo.clone());

        // Test ZADD
        let result = service
            .zadd(
                "zset",
                vec![
                    ScoredMember::new("a".to_string(), 1.0),
                    ScoredMember::new("b".to_string(), 2.0),
                    ScoredMember::new("c".to_string(), 3.0),
                ],
                None,
            )
            .await
            .unwrap();
        assert_eq!(result.count, 3);

        // Test ZCARD
        let card = service.zcard("zset").await.unwrap();
        assert_eq!(card, 3);

        // Test ZSCORE
        let score = service.zscore("zset", "b").await.unwrap();
        assert_eq!(score, Some(2.0));

        // Test ZRANK
        let rank = service.zrank("zset", "a").await.unwrap();
        assert_eq!(rank, Some(0));

        // Test ZRANGE
        let members = service.zrange("zset", 0, -1, None).await.unwrap();
        assert_eq!(members.len(), 3);

        // Test ZREM
        let removed = service.zrem("zset", vec!["a".to_string()]).await.unwrap();
        assert_eq!(removed, 1);

        let card = service.zcard("zset").await.unwrap();
        assert_eq!(card, 2);
    }

    #[tokio::test]
    async fn test_zincrby() {
        let repo = Arc::new(MockSortedSetRepository::new());
        let service = SortedSetService::new_with_repository(repo.clone());

        service
            .zadd("zset", vec![ScoredMember::new("a".to_string(), 1.0)], None)
            .await
            .unwrap();

        let new_score = service.zincrby("zset", "a", 2.5).await.unwrap();
        assert_eq!(new_score, 3.5);
    }

    #[tokio::test]
    async fn test_zpop_operations() {
        let repo = Arc::new(MockSortedSetRepository::new());
        let service = SortedSetService::new_with_repository(repo.clone());

        service
            .zadd(
                "zset",
                vec![
                    ScoredMember::new("a".to_string(), 1.0),
                    ScoredMember::new("b".to_string(), 2.0),
                    ScoredMember::new("c".to_string(), 3.0),
                ],
                None,
            )
            .await
            .unwrap();

        // Test ZPOPMIN
        let popped = service.zpopmin("zset", Some(1)).await.unwrap();
        assert_eq!(popped.len(), 1);
        assert_eq!(popped[0].member, "a");

        // Test ZPOPMAX
        let popped = service.zpopmax("zset", Some(1)).await.unwrap();
        assert_eq!(popped.len(), 1);
        assert_eq!(popped[0].member, "c");
    }

    #[tokio::test]
    async fn test_zscan() {
        let repo = Arc::new(MockSortedSetRepository::new());
        let service = SortedSetService::new_with_repository(repo.clone());

        service
            .zadd(
                "zset",
                vec![
                    ScoredMember::new("a".to_string(), 1.0),
                    ScoredMember::new("b".to_string(), 2.0),
                ],
                None,
            )
            .await
            .unwrap();

        let result = service.zscan("zset", 0, None, None).await.unwrap();
        assert_eq!(result.cursor, 0);
        assert_eq!(result.members.len(), 2);
    }

    #[tokio::test]
    async fn test_sorted_set_algebra_success() {
        let repo = Arc::new(MockSortedSetRepository::new());
        let service = SortedSetService::new_with_repository(repo.clone());

        service
            .zadd("zset1", vec![ScoredMember::new("a".to_string(), 1.0)], None)
            .await
            .unwrap();
        service
            .zadd("zset2", vec![ScoredMember::new("a".to_string(), 2.0)], None)
            .await
            .unwrap();

        let options = Some(ZSetAlgebraOptions {
            weights: Some(vec![1.0, 1.0]),
            aggregate: ZAggregate::Sum,
            with_scores: true,
        });

        let count = service
            .zunionstore(
                "dest",
                vec!["zset1".to_string(), "zset2".to_string()],
                options.clone(),
            )
            .await
            .unwrap();
        assert_eq!(count, 1);

        let members = service
            .zinter(
                vec!["zset1".to_string(), "zset2".to_string()],
                options.clone(),
            )
            .await
            .unwrap();
        assert_eq!(members.len(), 1);

        let count = service
            .zinterstore(
                "dest2",
                vec!["zset1".to_string(), "zset2".to_string()],
                options,
            )
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_sorted_set_service_new() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = SortedSetService::new(pool);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::Arc;
    use testcontainers::ContainerAsync;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::redis::{REDIS_PORT, Redis};

    async fn start_redis() -> (ContainerAsync<Redis>, String) {
        let container = Redis::default().start().await.unwrap();
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
        let url = format!("redis://{host}:{port}");
        (container, url)
    }

    async fn pool_with_redis() -> (ContainerAsync<Redis>, Arc<InstrumentedPool>) {
        let (container, redis_url) = start_redis().await;
        let pool = Arc::new(InstrumentedPool::new_for_tests_with_url(&redis_url).unwrap());
        (container, pool)
    }

    /// Test BZPOPMIN on a non-existent key with a 1-second timeout.
    ///
    /// The testcontainers Redis module uses Redis 5.0 which only accepts
    /// integer timeouts for BZPOPMIN. We call the command directly through
    /// the pool to verify blocking behavior, since the service layer passes
    /// f64 timeouts via enforce_timeout.
    #[tokio::test]
    async fn test_bzpopmin_returns_none_on_timeout() {
        let (_container, pool) = pool_with_redis().await;

        let mut conn = pool.get().await.unwrap();

        let start = std::time::Instant::now();
        let result: Option<(String, String, f64)> = redis::cmd("BZPOPMIN")
            .arg("nonexistent_key")
            .arg(1u32) // integer timeout for Redis 5.0 compatibility
            .query_async(&mut *conn)
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert!(result.is_none());
        assert!(
            elapsed.as_millis() >= 900,
            "Expected ~1s wait, got {}ms",
            elapsed.as_millis()
        );
    }

    /// Test BZPOPMIN returns data immediately when the sorted set has members.
    ///
    /// Uses the service for ZADD (which works on all Redis versions) and then
    /// calls BZPOPMIN directly through the pool with an integer timeout for
    /// Redis 5.0 compatibility.
    #[tokio::test]
    async fn test_bzpopmin_returns_data_when_available() {
        let (_container, pool) = pool_with_redis().await;
        let service = SortedSetService::new(pool.clone());

        // Add members to a sorted set via the service
        service
            .zadd(
                "myzset",
                vec![
                    ScoredMember::new("alice".to_string(), 1.0),
                    ScoredMember::new("bob".to_string(), 2.0),
                    ScoredMember::new("charlie".to_string(), 3.0),
                ],
                None,
            )
            .await
            .unwrap();

        // BZPOPMIN should immediately return the member with the lowest score
        let mut conn = pool.get().await.unwrap();
        let result: Option<(String, String, f64)> = redis::cmd("BZPOPMIN")
            .arg("myzset")
            .arg(1u32) // integer timeout for Redis 5.0 compatibility
            .query_async(&mut *conn)
            .await
            .unwrap();

        assert!(result.is_some());
        let (key, member, score) = result.unwrap();
        assert_eq!(key, "myzset");
        assert_eq!(member, "alice");
        assert_eq!(score, 1.0);
    }
}
