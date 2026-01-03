//! Set Service
//!
//! Business logic for Redis set operations.

use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{SetRepository, SetScanResult};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisSetRepository;

/// Service for set operations
pub struct SetService {
    repository: Arc<dyn SetRepository>,
}

impl SetService {
    /// Create a new SetService with default Redis repository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisSetRepository::new(pool)))
    }

    /// Create a new SetService with custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn SetRepository>) -> Self {
        Self { repository }
    }

    // ========== Basic operations ==========

    /// SADD - Add members to a set
    pub async fn sadd(&self, key: &str, members: Vec<String>) -> Result<i64, CacheError> {
        if members.is_empty() {
            return Err(CacheError::InvalidInput("Members cannot be empty".to_string()));
        }
        self.repository.sadd(key, &members).await
    }

    /// SREM - Remove members from a set
    pub async fn srem(&self, key: &str, members: Vec<String>) -> Result<i64, CacheError> {
        if members.is_empty() {
            return Err(CacheError::InvalidInput("Members cannot be empty".to_string()));
        }
        self.repository.srem(key, &members).await
    }

    /// SMEMBERS - Get all members of a set
    pub async fn smembers(&self, key: &str) -> Result<Vec<String>, CacheError> {
        self.repository.smembers(key).await
    }

    /// SISMEMBER - Check if a member exists in a set
    pub async fn sismember(&self, key: &str, member: &str) -> Result<bool, CacheError> {
        self.repository.sismember(key, member).await
    }

    /// SMISMEMBER - Check if multiple members exist in a set
    pub async fn smismember(&self, key: &str, members: Vec<String>) -> Result<Vec<bool>, CacheError> {
        if members.is_empty() {
            return Err(CacheError::InvalidInput("Members cannot be empty".to_string()));
        }
        self.repository.smismember(key, &members).await
    }

    /// SCARD - Get the number of members in a set
    pub async fn scard(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.scard(key).await
    }

    // ========== Random access operations ==========

    /// SRANDMEMBER - Get random members from a set without removing them
    pub async fn srandmember(&self, key: &str, count: Option<i64>) -> Result<Vec<String>, CacheError> {
        self.repository.srandmember(key, count).await
    }

    /// SPOP - Remove and return random members from a set
    pub async fn spop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError> {
        self.repository.spop(key, count).await
    }

    /// SMOVE - Move a member from one set to another
    pub async fn smove(&self, source: &str, destination: &str, member: &str) -> Result<bool, CacheError> {
        self.repository.smove(source, destination, member).await
    }

    // ========== Set algebra operations ==========

    /// SINTER - Get the intersection of multiple sets
    pub async fn sinter(&self, keys: Vec<String>) -> Result<Vec<String>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.sinter(&keys).await
    }

    /// SINTERSTORE - Store the intersection of multiple sets in a destination key
    pub async fn sinterstore(&self, destination: &str, keys: Vec<String>) -> Result<i64, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.sinterstore(destination, &keys).await
    }

    /// SINTERCARD - Get the cardinality of the intersection
    pub async fn sintercard(&self, keys: Vec<String>, limit: Option<u64>) -> Result<i64, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.sintercard(&keys, limit).await
    }

    /// SUNION - Get the union of multiple sets
    pub async fn sunion(&self, keys: Vec<String>) -> Result<Vec<String>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.sunion(&keys).await
    }

    /// SUNIONSTORE - Store the union of multiple sets in a destination key
    pub async fn sunionstore(&self, destination: &str, keys: Vec<String>) -> Result<i64, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.sunionstore(destination, &keys).await
    }

    /// SDIFF - Get the difference of sets (members in first set but not in others)
    pub async fn sdiff(&self, keys: Vec<String>) -> Result<Vec<String>, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.sdiff(&keys).await
    }

    /// SDIFFSTORE - Store the difference of sets in a destination key
    pub async fn sdiffstore(&self, destination: &str, keys: Vec<String>) -> Result<i64, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        self.repository.sdiffstore(destination, &keys).await
    }

    // ========== Scan operation ==========

    /// SSCAN - Incrementally iterate set members
    pub async fn sscan(
        &self,
        key: &str,
        cursor: u64,
        pattern: Option<&str>,
        count: Option<u64>,
    ) -> Result<SetScanResult, CacheError> {
        self.repository.sscan(key, cursor, pattern, count).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockSetRepository;

    #[tokio::test]
    async fn test_set_service_validations() {
        let repo = Arc::new(MockSetRepository::new());
        let service = SetService::new_with_repository(repo);

        let err = service.sadd("set", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.srem("set", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.smismember("set", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.sinter(Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.sinterstore("dest", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.sintercard(Vec::new(), None).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.sunion(Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.sunionstore("dest", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.sdiff(Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.sdiffstore("dest", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_set_service_operations() {
        let repo = Arc::new(MockSetRepository::new());
        let service = SetService::new_with_repository(repo.clone());

        // Test SADD
        let added = service.sadd("myset", vec!["a".to_string(), "b".to_string(), "c".to_string()]).await.unwrap();
        assert_eq!(added, 3);

        // Test SCARD
        let card = service.scard("myset").await.unwrap();
        assert_eq!(card, 3);

        // Test SISMEMBER
        let is_member = service.sismember("myset", "a").await.unwrap();
        assert!(is_member);

        let is_member = service.sismember("myset", "z").await.unwrap();
        assert!(!is_member);

        // Test SMEMBERS
        let members = service.smembers("myset").await.unwrap();
        assert_eq!(members.len(), 3);

        // Test SREM
        let removed = service.srem("myset", vec!["a".to_string()]).await.unwrap();
        assert_eq!(removed, 1);

        let card = service.scard("myset").await.unwrap();
        assert_eq!(card, 2);

        // Test SMISMEMBER
        let results = service.smismember("myset", vec!["b".to_string(), "z".to_string()]).await.unwrap();
        assert_eq!(results, vec![true, false]);
    }

    #[tokio::test]
    async fn test_set_service_random_operations() {
        let repo = Arc::new(MockSetRepository::new());
        let service = SetService::new_with_repository(repo.clone());

        service.sadd("myset", vec!["a".to_string(), "b".to_string(), "c".to_string()]).await.unwrap();

        // Test SRANDMEMBER
        let random = service.srandmember("myset", Some(2)).await.unwrap();
        assert_eq!(random.len(), 2);

        // Test SPOP
        let popped = service.spop("myset", Some(1)).await.unwrap();
        assert_eq!(popped.len(), 1);

        let card = service.scard("myset").await.unwrap();
        assert_eq!(card, 2);

        // Test SMOVE
        service.sadd("other", vec!["x".to_string()]).await.unwrap();
        let moved = service.smove("myset", "other", "a").await.unwrap();
        assert!(moved);

        let other_card = service.scard("other").await.unwrap();
        assert_eq!(other_card, 2);
    }

    #[tokio::test]
    async fn test_set_service_algebra_operations() {
        let repo = Arc::new(MockSetRepository::new());
        let service = SetService::new_with_repository(repo.clone());

        service.sadd("set1", vec!["a".to_string(), "b".to_string(), "c".to_string()]).await.unwrap();
        service.sadd("set2", vec!["b".to_string(), "c".to_string(), "d".to_string()]).await.unwrap();

        // Test SINTER
        let inter = service.sinter(vec!["set1".to_string(), "set2".to_string()]).await.unwrap();
        assert_eq!(inter.len(), 2);
        assert!(inter.contains(&"b".to_string()));
        assert!(inter.contains(&"c".to_string()));

        // Test SUNION
        let union = service.sunion(vec!["set1".to_string(), "set2".to_string()]).await.unwrap();
        assert_eq!(union.len(), 4);

        // Test SDIFF
        let diff = service.sdiff(vec!["set1".to_string(), "set2".to_string()]).await.unwrap();
        assert_eq!(diff.len(), 1);
        assert!(diff.contains(&"a".to_string()));

        // Test SINTERSTORE
        let count = service.sinterstore("inter_result", vec!["set1".to_string(), "set2".to_string()]).await.unwrap();
        assert_eq!(count, 2);

        // Test SUNIONSTORE
        let count = service.sunionstore("union_result", vec!["set1".to_string(), "set2".to_string()]).await.unwrap();
        assert_eq!(count, 4);

        // Test SDIFFSTORE
        let count = service.sdiffstore("diff_result", vec!["set1".to_string(), "set2".to_string()]).await.unwrap();
        assert_eq!(count, 1);

        // Test SINTERCARD
        let card = service.sintercard(vec!["set1".to_string(), "set2".to_string()], None).await.unwrap();
        assert_eq!(card, 2);
    }

    #[tokio::test]
    async fn test_set_service_sscan() {
        let repo = Arc::new(MockSetRepository::new());
        let service = SetService::new_with_repository(repo.clone());

        service.sadd("myset", vec!["a".to_string(), "b".to_string(), "c".to_string()]).await.unwrap();

        let result = service.sscan("myset", 0, None, None).await.unwrap();
        assert_eq!(result.cursor, 0); // Mock returns 0 indicating complete
        assert_eq!(result.members.len(), 3);
    }

    #[test]
    fn test_set_service_new() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = SetService::new(pool);
    }
}
