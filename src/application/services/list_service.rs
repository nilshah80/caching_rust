//! List Service
//!
//! Business logic for Redis list operations.

use std::sync::Arc;
use std::time::Duration;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    BlockingPopResult, InsertPosition, LMPopResult, LPosOptions, ListDirection, ListRepository,
};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisListRepository;
use crate::shared::blocking::BlockingTimeoutEnforcer;

/// Service for list operations
pub struct ListService {
    repository: Arc<dyn ListRepository>,
    timeout_enforcer: BlockingTimeoutEnforcer,
}

impl ListService {
    /// Create a new ListService with default Redis repository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisListRepository::new(pool)))
    }

    /// Create a new ListService with custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn ListRepository>) -> Self {
        Self {
            repository,
            timeout_enforcer: BlockingTimeoutEnforcer::new(),
        }
    }

    /// Set custom max blocking timeout (for testing or configuration)
    pub fn with_max_blocking_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_enforcer = BlockingTimeoutEnforcer::with_max(timeout.as_secs());
        self
    }

    /// Enforce timeout bounds for blocking operations
    fn enforce_timeout(&self, requested: Duration) -> Duration {
        self.timeout_enforcer.enforce(requested)
    }

    // ========== Non-blocking operations ==========

    /// LPUSH - Insert values at the head of the list
    pub async fn lpush(&self, key: &str, values: Vec<String>) -> Result<i64, CacheError> {
        if values.is_empty() {
            return Err(CacheError::InvalidInput(
                "Values cannot be empty".to_string(),
            ));
        }
        self.repository.lpush(key, &values).await
    }

    /// RPUSH - Insert values at the tail of the list
    pub async fn rpush(&self, key: &str, values: Vec<String>) -> Result<i64, CacheError> {
        if values.is_empty() {
            return Err(CacheError::InvalidInput(
                "Values cannot be empty".to_string(),
            ));
        }
        self.repository.rpush(key, &values).await
    }

    /// LPUSHX - Insert value at head only if list exists
    pub async fn lpush_x(&self, key: &str, values: Vec<String>) -> Result<i64, CacheError> {
        if values.is_empty() {
            return Err(CacheError::InvalidInput(
                "Values cannot be empty".to_string(),
            ));
        }
        self.repository.lpush_x(key, &values).await
    }

    /// RPUSHX - Insert value at tail only if list exists
    pub async fn rpush_x(&self, key: &str, values: Vec<String>) -> Result<i64, CacheError> {
        if values.is_empty() {
            return Err(CacheError::InvalidInput(
                "Values cannot be empty".to_string(),
            ));
        }
        self.repository.rpush_x(key, &values).await
    }

    /// LPOP - Remove and return elements from the head
    pub async fn lpop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError> {
        self.repository.lpop(key, count).await
    }

    /// RPOP - Remove and return elements from the tail
    pub async fn rpop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError> {
        self.repository.rpop(key, count).await
    }

    /// LRANGE - Get a range of elements from the list
    pub async fn lrange(
        &self,
        key: &str,
        start: i64,
        stop: i64,
    ) -> Result<Vec<String>, CacheError> {
        self.repository.lrange(key, start, stop).await
    }

    /// LLEN - Get the length of the list
    pub async fn llen(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.llen(key).await
    }

    /// LINDEX - Get element at index
    pub async fn lindex(&self, key: &str, index: i64) -> Result<Option<String>, CacheError> {
        self.repository.lindex(key, index).await
    }

    /// LSET - Set element at index
    pub async fn lset(&self, key: &str, index: i64, value: &str) -> Result<(), CacheError> {
        self.repository.lset(key, index, value).await
    }

    /// LINSERT - Insert element before or after pivot
    pub async fn linsert(
        &self,
        key: &str,
        position: InsertPosition,
        pivot: &str,
        value: &str,
    ) -> Result<i64, CacheError> {
        self.repository.linsert(key, position, pivot, value).await
    }

    /// LREM - Remove elements equal to value
    pub async fn lrem(&self, key: &str, count: i64, value: &str) -> Result<i64, CacheError> {
        self.repository.lrem(key, count, value).await
    }

    /// LTRIM - Trim list to specified range
    pub async fn ltrim(&self, key: &str, start: i64, stop: i64) -> Result<(), CacheError> {
        self.repository.ltrim(key, start, stop).await
    }

    /// LPOS - Get index of element in list
    pub async fn lpos(
        &self,
        key: &str,
        element: &str,
        options: LPosOptions,
    ) -> Result<Vec<i64>, CacheError> {
        self.repository.lpos(key, element, options).await
    }

    /// LMOVE - Move element from source to destination
    pub async fn lmove(
        &self,
        source: &str,
        destination: &str,
        src_dir: ListDirection,
        dst_dir: ListDirection,
    ) -> Result<Option<String>, CacheError> {
        self.repository
            .lmove(source, destination, src_dir, dst_dir)
            .await
    }

    /// RPOPLPUSH - Pop from source tail and push to destination head (deprecated, use LMOVE)
    pub async fn rpop_lpush(
        &self,
        source: &str,
        destination: &str,
    ) -> Result<Option<String>, CacheError> {
        self.repository.rpop_lpush(source, destination).await
    }

    // ========== Blocking operations ==========

    /// BLPOP - Blocking pop from head of list(s)
    /// Returns None if timeout is reached
    pub async fn blpop(
        &self,
        keys: Vec<String>,
        timeout_seconds: u32,
    ) -> Result<Option<BlockingPopResult>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        let timeout = self.enforce_timeout(Duration::from_secs(timeout_seconds as u64));
        self.repository.blpop(&keys, timeout).await
    }

    /// BRPOP - Blocking pop from tail of list(s)
    /// Returns None if timeout is reached
    pub async fn brpop(
        &self,
        keys: Vec<String>,
        timeout_seconds: u32,
    ) -> Result<Option<BlockingPopResult>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        let timeout = self.enforce_timeout(Duration::from_secs(timeout_seconds as u64));
        self.repository.brpop(&keys, timeout).await
    }

    /// BLMOVE - Blocking move from source to destination
    /// Returns None if timeout is reached
    pub async fn blmove(
        &self,
        source: &str,
        destination: &str,
        src_dir: ListDirection,
        dst_dir: ListDirection,
        timeout_seconds: u32,
    ) -> Result<Option<String>, CacheError> {
        let timeout = self.enforce_timeout(Duration::from_secs(timeout_seconds as u64));
        self.repository
            .blmove(source, destination, src_dir, dst_dir, timeout)
            .await
    }

    /// BRPOPLPUSH - Blocking pop from source tail and push to destination head (deprecated, use BLMOVE)
    /// Returns None if timeout is reached
    pub async fn brpop_lpush(
        &self,
        source: &str,
        destination: &str,
        timeout_seconds: u32,
    ) -> Result<Option<String>, CacheError> {
        let timeout = self.enforce_timeout(Duration::from_secs(timeout_seconds as u64));
        self.repository
            .brpop_lpush(source, destination, timeout)
            .await
    }

    /// LMPOP - Atomically pop elements from the first non-empty list (Redis 7.0+)
    pub async fn lmpop(
        &self,
        keys: Vec<String>,
        direction: ListDirection,
        count: Option<u32>,
    ) -> Result<Option<LMPopResult>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.lmpop(&keys, direction, count).await
    }

    /// BLMPOP - Blocking pop from the first non-empty list (Redis 7.0+)
    /// Returns None if timeout is reached
    pub async fn blmpop(
        &self,
        keys: Vec<String>,
        direction: ListDirection,
        timeout_seconds: u32,
        count: Option<u32>,
    ) -> Result<Option<LMPopResult>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        let timeout = self.enforce_timeout(Duration::from_secs(timeout_seconds as u64));
        self.repository
            .blmpop(&keys, direction, timeout, count)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockListRepository;

    #[tokio::test]
    async fn test_list_service_validations() {
        let repo = Arc::new(MockListRepository::new());
        let service = ListService::new_with_repository(repo);

        let err = service.lpush("list", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.rpush("list", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.lpush_x("list", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.rpush_x("list", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.blpop(Vec::new(), 5).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.brpop(Vec::new(), 5).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_list_service_timeout_enforcement() {
        let repo = Arc::new(MockListRepository::new());
        let service = ListService::new_with_repository(repo)
            .with_max_blocking_timeout(Duration::from_secs(10));

        // Test that enforced timeout is capped
        let timeout = service.enforce_timeout(Duration::from_secs(60));
        assert_eq!(timeout, Duration::from_secs(10));

        // Test that smaller timeout is not modified
        let timeout = service.enforce_timeout(Duration::from_secs(5));
        assert_eq!(timeout, Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_list_service_operations() {
        let repo = Arc::new(MockListRepository::new());
        let service = ListService::new_with_repository(repo.clone());

        // Test LPUSH
        let len = service
            .lpush("mylist", vec!["a".to_string(), "b".to_string()])
            .await
            .unwrap();
        assert_eq!(len, 2);

        // Test RPUSH
        let len = service
            .rpush("mylist", vec!["c".to_string()])
            .await
            .unwrap();
        assert_eq!(len, 3);

        // Test LLEN
        let len = service.llen("mylist").await.unwrap();
        assert_eq!(len, 3);

        // Test LRANGE
        let values = service.lrange("mylist", 0, -1).await.unwrap();
        assert_eq!(values, vec!["b", "a", "c"]);

        // Test LPOP
        let values = service.lpop("mylist", Some(1)).await.unwrap();
        assert_eq!(values, vec!["b"]);

        // Test RPOP
        let values = service.rpop("mylist", Some(1)).await.unwrap();
        assert_eq!(values, vec!["c"]);

        // Test LINDEX
        let value = service.lindex("mylist", 0).await.unwrap();
        assert_eq!(value.as_deref(), Some("a"));

        // Test LSET
        service.lset("mylist", 0, "z").await.unwrap();
        let value = service.lindex("mylist", 0).await.unwrap();
        assert_eq!(value.as_deref(), Some("z"));

        // Test LTRIM
        service.ltrim("mylist", 0, 0).await.unwrap();
        let len = service.llen("mylist").await.unwrap();
        assert_eq!(len, 1);
    }

    #[tokio::test]
    async fn test_list_service_additional_operations() {
        let repo = Arc::new(MockListRepository::new());
        let service = ListService::new_with_repository(repo.clone());

        let len = service
            .rpush(
                "list",
                vec!["a".to_string(), "b".to_string(), "a".to_string()],
            )
            .await
            .unwrap();
        assert_eq!(len, 3);

        let len = service
            .lpush_x("list", vec!["z".to_string()])
            .await
            .unwrap();
        assert_eq!(len, 4);

        let len = service
            .rpush_x("list", vec!["y".to_string()])
            .await
            .unwrap();
        assert_eq!(len, 5);

        let indices = service
            .lpos(
                "list",
                "a",
                LPosOptions {
                    count: Some(2),
                    rank: Some(1),
                    max_len: Some(10),
                },
            )
            .await
            .unwrap();
        assert_eq!(indices, vec![1, 3]);

        let len = service
            .linsert("list", InsertPosition::Before, "b", "x")
            .await
            .unwrap();
        assert_eq!(len, 6);

        let removed = service.lrem("list", 1, "a").await.unwrap();
        assert_eq!(removed, 1);

        let moved = service
            .lmove("list", "dest", ListDirection::Left, ListDirection::Right)
            .await
            .unwrap();
        assert_eq!(moved.as_deref(), Some("z"));

        let moved = service.rpop_lpush("list", "dest2").await.unwrap();
        assert_eq!(moved.as_deref(), Some("y"));

        repo.insert("blocklist", vec!["one".to_string(), "two".to_string()]);
        let result = service
            .blpop(vec!["blocklist".to_string()], 1)
            .await
            .unwrap();
        assert_eq!(result.unwrap().value, "one");

        repo.insert("blocklist2", vec!["three".to_string(), "four".to_string()]);
        let result = service
            .brpop(vec!["blocklist2".to_string()], 1)
            .await
            .unwrap();
        assert_eq!(result.unwrap().value, "four");

        repo.insert("movesrc", vec!["m1".to_string()]);
        let moved = service
            .blmove(
                "movesrc",
                "movedst",
                ListDirection::Left,
                ListDirection::Right,
                1,
            )
            .await
            .unwrap();
        assert_eq!(moved.as_deref(), Some("m1"));

        repo.insert("movesrc2", vec!["m2".to_string()]);
        let moved = service
            .brpop_lpush("movesrc2", "movedst2", 1)
            .await
            .unwrap();
        assert_eq!(moved.as_deref(), Some("m2"));
    }

    #[test]
    fn test_list_service_new() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = ListService::new(pool);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::test_support::start_redis_container;
    use std::sync::Arc;
    use testcontainers::ContainerAsync;
    use testcontainers_modules::redis::Redis;

    async fn start_redis() -> Option<(ContainerAsync<Redis>, String)> {
        start_redis_container().await
    }

    async fn create_service() -> Option<(ContainerAsync<Redis>, ListService)> {
        let (container, redis_url) = start_redis().await?;
        let pool = Arc::new(InstrumentedPool::new_for_tests_with_url(&redis_url).unwrap());
        let service = ListService::new(pool);
        Some((container, service))
    }

    #[tokio::test]
    async fn test_blpop_returns_none_on_timeout() {
        let Some((_container, service)) = create_service().await else {
            return;
        };

        let start = std::time::Instant::now();
        let result = service.blpop(vec!["nonexistent_key".to_string()], 1).await;
        let elapsed = start.elapsed();

        // BLPOP timeout returns Ok(None) or a timeout error in some connection modes.
        // In multiplexed mode, the response may arrive before the full BLPOP wait.
        match result {
            Ok(val) => {
                assert!(val.is_none());
                assert!(
                    elapsed.as_millis() >= 900,
                    "Expected ~1s wait, got {}ms",
                    elapsed.as_millis()
                );
            }
            Err(CacheError::RedisError(_)) => {} // Connection-level timeout is acceptable
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_blpop_returns_data_when_available() {
        let Some((_container, service)) = create_service().await else {
            return;
        };

        // LPUSH pushes elements to the head; order in list will be [c, b, a]
        service
            .lpush(
                "mylist",
                vec!["a".to_string(), "b".to_string(), "c".to_string()],
            )
            .await
            .unwrap();

        // BLPOP pops from the head, so it should return "c"
        let result = service.blpop(vec!["mylist".to_string()], 1).await.unwrap();

        assert!(result.is_some());
        let pop = result.unwrap();
        assert_eq!(pop.key, "mylist");
        assert_eq!(pop.value, "c");
    }

    #[tokio::test]
    async fn test_brpop_returns_data_when_available() {
        let Some((_container, service)) = create_service().await else {
            return;
        };

        // RPUSH appends to the tail; order in list will be [x, y, z]
        service
            .rpush(
                "mylist",
                vec!["x".to_string(), "y".to_string(), "z".to_string()],
            )
            .await
            .unwrap();

        // BRPOP pops from the tail, so it should return "z"
        let result = service.brpop(vec!["mylist".to_string()], 1).await.unwrap();

        assert!(result.is_some());
        let pop = result.unwrap();
        assert_eq!(pop.key, "mylist");
        assert_eq!(pop.value, "z");
    }

    /// BLMOVE requires Redis 6.2+. The testcontainers default image may be older,
    /// so these tests skip gracefully if the command is not available.
    #[tokio::test]
    async fn test_blmove_returns_none_on_timeout() {
        let Some((_container, service)) = create_service().await else {
            return;
        };

        let start = std::time::Instant::now();
        let result = service
            .blmove(
                "empty_source",
                "dest",
                ListDirection::Left,
                ListDirection::Right,
                1,
            )
            .await;
        let elapsed = start.elapsed();

        match result {
            Ok(val) => {
                assert!(val.is_none());
                assert!(
                    elapsed.as_millis() >= 900,
                    "Expected ~1s wait, got {}ms",
                    elapsed.as_millis()
                );
            }
            Err(CacheError::RedisError(ref e)) if e.to_string().contains("unknown command") => {
                // BLMOVE not available on this Redis version — skip
                return;
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    /// BLMOVE requires Redis 6.2+.
    #[tokio::test]
    async fn test_blmove_moves_element() {
        let Some((_container, service)) = create_service().await else {
            return;
        };

        service
            .lpush("src_list", vec!["alpha".to_string(), "beta".to_string()])
            .await
            .unwrap();

        let result = service
            .blmove(
                "src_list",
                "dst_list",
                ListDirection::Left,
                ListDirection::Right,
                1,
            )
            .await;

        match result {
            Ok(val) => {
                assert_eq!(val.as_deref(), Some("beta"));
                let dest_values = service.lrange("dst_list", 0, -1).await.unwrap();
                assert_eq!(dest_values, vec!["beta"]);
                let src_values = service.lrange("src_list", 0, -1).await.unwrap();
                assert_eq!(src_values, vec!["alpha"]);
            }
            Err(CacheError::RedisError(ref e)) if e.to_string().contains("unknown command") => {
                // BLMOVE not available on this Redis version — skip
                return;
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }
}
