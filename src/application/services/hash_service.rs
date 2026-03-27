use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::HashRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisHashRepository;

pub struct HashService {
    repository: Arc<dyn HashRepository>,
}

impl HashService {
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisHashRepository::new(pool)))
    }

    pub fn new_with_repository(repository: Arc<dyn HashRepository>) -> Self {
        Self { repository }
    }

    pub async fn hget(&self, key: &str, field: &str) -> Result<Option<String>, CacheError> {
        self.repository.hget(key, field).await
    }

    pub async fn hset(&self, key: &str, pairs: Vec<(String, String)>) -> Result<i64, CacheError> {
        if pairs.is_empty() {
            return Err(CacheError::InvalidInput(
                "Pairs cannot be empty".to_string(),
            ));
        }
        self.repository.hset(key, pairs).await
    }

    pub async fn hset_nx(&self, key: &str, field: &str, value: &str) -> Result<bool, CacheError> {
        self.repository.hset_nx(key, field, value).await
    }

    pub async fn hgetall(&self, key: &str) -> Result<HashMap<String, String>, CacheError> {
        self.repository.hgetall(key).await
    }

    pub async fn hmget(
        &self,
        key: &str,
        fields: Vec<String>,
    ) -> Result<Vec<Option<String>>, CacheError> {
        if fields.is_empty() {
            return Err(CacheError::InvalidInput(
                "Fields cannot be empty".to_string(),
            ));
        }
        self.repository.hmget(key, &fields).await
    }

    pub async fn hmset(&self, key: &str, pairs: Vec<(String, String)>) -> Result<(), CacheError> {
        if pairs.is_empty() {
            return Err(CacheError::InvalidInput(
                "Pairs cannot be empty".to_string(),
            ));
        }
        self.repository.hmset(key, pairs).await
    }

    pub async fn hdel(&self, key: &str, fields: Vec<String>) -> Result<i64, CacheError> {
        if fields.is_empty() {
            return Err(CacheError::InvalidInput(
                "Fields cannot be empty".to_string(),
            ));
        }
        self.repository.hdel(key, &fields).await
    }

    pub async fn hexists(&self, key: &str, field: &str) -> Result<bool, CacheError> {
        self.repository.hexists(key, field).await
    }

    pub async fn hkeys(&self, key: &str) -> Result<Vec<String>, CacheError> {
        self.repository.hkeys(key).await
    }

    pub async fn hvals(&self, key: &str) -> Result<Vec<String>, CacheError> {
        self.repository.hvals(key).await
    }

    pub async fn hlen(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.hlen(key).await
    }

    pub async fn hincr_by(&self, key: &str, field: &str, delta: i64) -> Result<i64, CacheError> {
        self.repository.hincr_by(key, field, delta).await
    }

    pub async fn hincr_by_float(
        &self,
        key: &str,
        field: &str,
        delta: f64,
    ) -> Result<f64, CacheError> {
        self.repository.hincr_by_float(key, field, delta).await
    }

    pub async fn hstr_len(&self, key: &str, field: &str) -> Result<i64, CacheError> {
        self.repository.hstr_len(key, field).await
    }

    pub async fn hrand_field(
        &self,
        key: &str,
        count: Option<i64>,
        with_values: bool,
    ) -> Result<Vec<String>, CacheError> {
        if with_values && count.is_none() {
            return Err(CacheError::InvalidInput(
                "Count is required when with_values is true".to_string(),
            ));
        }
        self.repository.hrand_field(key, count, with_values).await
    }

    pub async fn hscan(
        &self,
        key: &str,
        cursor: u64,
        pattern: Option<String>,
        count: Option<u64>,
    ) -> Result<(u64, Vec<String>), CacheError> {
        self.repository.hscan(key, cursor, pattern, count).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::redis::connection::InstrumentedPool;
    use crate::test_support::MockHashRepository;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_hash_service_validations() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);

        let err = service.hset("hash", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.hmget("hash", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.hmset("hash", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.hdel("hash", Vec::new()).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.hrand_field("hash", None, true).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hash_service_operations() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo.clone());

        let count = service
            .hset(
                "hash",
                vec![
                    ("field1".to_string(), "1".to_string()),
                    ("field2".to_string(), "2".to_string()),
                ],
            )
            .await
            .unwrap();
        assert_eq!(count, 2);

        let value = service.hget("hash", "field1").await.unwrap();
        assert_eq!(value.as_deref(), Some("1"));

        let result = service.hset_nx("hash", "field1", "3").await.unwrap();
        assert!(!result);

        let all = service.hgetall("hash").await.unwrap();
        assert_eq!(all.len(), 2);

        let values = service
            .hmget("hash", vec!["field1".to_string(), "missing".to_string()])
            .await
            .unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0].as_deref(), Some("1"));

        service
            .hmset("hash", vec![("field3".to_string(), "3".to_string())])
            .await
            .unwrap();

        let deleted = service
            .hdel("hash", vec!["field2".to_string()])
            .await
            .unwrap();
        assert_eq!(deleted, 1);

        let exists = service.hexists("hash", "field1").await.unwrap();
        assert!(exists);

        let keys = service.hkeys("hash").await.unwrap();
        assert!(keys.contains(&"field1".to_string()));

        let values = service.hvals("hash").await.unwrap();
        assert!(values.contains(&"1".to_string()));

        let length = service.hlen("hash").await.unwrap();
        assert_eq!(length, 2);

        let incr = service.hincr_by("hash", "counter", 5).await.unwrap();
        assert_eq!(incr, 5);

        let incr_float = service.hincr_by_float("hash", "float", 1.5).await.unwrap();
        assert_eq!(incr_float, 1.5);

        let len = service.hstr_len("hash", "field1").await.unwrap();
        assert_eq!(len, 1);

        let random = service.hrand_field("hash", Some(1), false).await.unwrap();
        assert_eq!(random.len(), 1);

        let (cursor, entries) = service
            .hscan("hash", 0, Some("field".to_string()), Some(10))
            .await
            .unwrap();
        assert_eq!(cursor, 0);
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_hash_service_new() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = HashService::new(pool);
    }
}
