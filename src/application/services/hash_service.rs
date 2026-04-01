use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    ExpireCondition, HSetExCondition, HashExpiration, HashRepository,
};
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

    // --- Hash field expiration methods (Redis 7.4+) ---

    fn validate_key_and_fields(key: &str, fields: &[String]) -> Result<(), CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if fields.is_empty() {
            return Err(CacheError::InvalidInput(
                "Fields cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn hexpire(
        &self,
        key: &str,
        seconds: i64,
        fields: Vec<String>,
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        Self::validate_key_and_fields(key, &fields)?;
        if seconds <= 0 {
            return Err(CacheError::InvalidInput(
                "Seconds must be positive".to_string(),
            ));
        }
        self.repository
            .hexpire(key, seconds, &fields, condition)
            .await
    }

    pub async fn hpexpire(
        &self,
        key: &str,
        milliseconds: i64,
        fields: Vec<String>,
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        Self::validate_key_and_fields(key, &fields)?;
        if milliseconds <= 0 {
            return Err(CacheError::InvalidInput(
                "Milliseconds must be positive".to_string(),
            ));
        }
        self.repository
            .hpexpire(key, milliseconds, &fields, condition)
            .await
    }

    pub async fn hexpire_at(
        &self,
        key: &str,
        unix_time: i64,
        fields: Vec<String>,
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        Self::validate_key_and_fields(key, &fields)?;
        if unix_time <= 0 {
            return Err(CacheError::InvalidInput(
                "Unix time must be positive".to_string(),
            ));
        }
        self.repository
            .hexpire_at(key, unix_time, &fields, condition)
            .await
    }

    pub async fn hpexpire_at(
        &self,
        key: &str,
        unix_time_ms: i64,
        fields: Vec<String>,
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        Self::validate_key_and_fields(key, &fields)?;
        if unix_time_ms <= 0 {
            return Err(CacheError::InvalidInput(
                "Unix time in milliseconds must be positive".to_string(),
            ));
        }
        self.repository
            .hpexpire_at(key, unix_time_ms, &fields, condition)
            .await
    }

    pub async fn hexpire_time(
        &self,
        key: &str,
        fields: Vec<String>,
    ) -> Result<Vec<i64>, CacheError> {
        Self::validate_key_and_fields(key, &fields)?;
        self.repository.hexpire_time(key, &fields).await
    }

    pub async fn hpexpire_time(
        &self,
        key: &str,
        fields: Vec<String>,
    ) -> Result<Vec<i64>, CacheError> {
        Self::validate_key_and_fields(key, &fields)?;
        self.repository.hpexpire_time(key, &fields).await
    }

    pub async fn httl(&self, key: &str, fields: Vec<String>) -> Result<Vec<i64>, CacheError> {
        Self::validate_key_and_fields(key, &fields)?;
        self.repository.httl(key, &fields).await
    }

    pub async fn hpttl(&self, key: &str, fields: Vec<String>) -> Result<Vec<i64>, CacheError> {
        Self::validate_key_and_fields(key, &fields)?;
        self.repository.hpttl(key, &fields).await
    }

    pub async fn hpersist(&self, key: &str, fields: Vec<String>) -> Result<Vec<i64>, CacheError> {
        Self::validate_key_and_fields(key, &fields)?;
        self.repository.hpersist(key, &fields).await
    }

    // --- Redis 8.0+ hash commands ---

    pub async fn hgetex(
        &self,
        key: &str,
        fields: Vec<String>,
        expiration: Option<HashExpiration>,
    ) -> Result<Vec<Option<String>>, CacheError> {
        Self::validate_key_and_fields(key, &fields)?;
        self.repository.hgetex(key, &fields, expiration).await
    }

    pub async fn hsetex(
        &self,
        key: &str,
        field_values: Vec<(String, String)>,
        condition: Option<HSetExCondition>,
        expiration: Option<HashExpiration>,
    ) -> Result<i64, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if field_values.is_empty() {
            return Err(CacheError::InvalidInput(
                "Fields cannot be empty".to_string(),
            ));
        }
        self.repository
            .hsetex(key, &field_values, condition, expiration)
            .await
    }

    pub async fn hgetdel(
        &self,
        key: &str,
        fields: Vec<String>,
    ) -> Result<Vec<Option<String>>, CacheError> {
        Self::validate_key_and_fields(key, &fields)?;
        self.repository.hgetdel(key, &fields).await
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

    #[tokio::test]
    async fn test_hexpire_empty_key() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let err = service
            .hexpire("", 10, vec!["f1".into()], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hexpire_empty_fields() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let err = service.hexpire("key", 10, vec![], None).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hexpire_negative_seconds() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let err = service
            .hexpire("key", -1, vec!["f1".into()], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hexpire_zero_seconds() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let err = service
            .hexpire("key", 0, vec!["f1".into()], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hexpire_success() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let result = service
            .hexpire("key", 10, vec!["f1".into(), "f2".into()], None)
            .await
            .unwrap();
        assert_eq!(result, vec![1, 1]);
    }

    #[tokio::test]
    async fn test_hpexpire_validation_and_success() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);

        let err = service
            .hpexpire("", 1000, vec!["f1".into()], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .hpexpire("key", 1000, vec![], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .hpexpire("key", 0, vec!["f1".into()], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let result = service
            .hpexpire("key", 1000, vec!["f1".into()], None)
            .await
            .unwrap();
        assert_eq!(result, vec![1]);
    }

    #[tokio::test]
    async fn test_hexpire_at_validation_and_success() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);

        let err = service
            .hexpire_at("", 1000, vec!["f1".into()], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .hexpire_at("key", 1000, vec![], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .hexpire_at("key", -1, vec!["f1".into()], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let result = service
            .hexpire_at("key", 1000, vec!["f1".into()], None)
            .await
            .unwrap();
        assert_eq!(result, vec![1]);
    }

    #[tokio::test]
    async fn test_hpexpire_at_validation_and_success() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);

        let err = service
            .hpexpire_at("", 1000, vec!["f1".into()], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .hpexpire_at("key", 1000, vec![], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .hpexpire_at("key", 0, vec!["f1".into()], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let result = service
            .hpexpire_at("key", 1000, vec!["f1".into()], None)
            .await
            .unwrap();
        assert_eq!(result, vec![1]);
    }

    #[tokio::test]
    async fn test_hexpire_time_validation_and_success() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);

        let err = service
            .hexpire_time("", vec!["f1".into()])
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.hexpire_time("key", vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let result = service
            .hexpire_time("key", vec!["f1".into()])
            .await
            .unwrap();
        assert_eq!(result, vec![-1]);
    }

    #[tokio::test]
    async fn test_hpexpire_time_validation_and_success() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);

        let err = service
            .hpexpire_time("", vec!["f1".into()])
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.hpexpire_time("key", vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let result = service
            .hpexpire_time("key", vec!["f1".into()])
            .await
            .unwrap();
        assert_eq!(result, vec![-1]);
    }

    #[tokio::test]
    async fn test_httl_delegates_to_mock() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);

        let err = service.httl("", vec!["f1".into()]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.httl("key", vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let result = service
            .httl("key", vec!["f1".into(), "f2".into()])
            .await
            .unwrap();
        assert_eq!(result, vec![-1, -1]);
    }

    #[tokio::test]
    async fn test_hpttl_delegates_to_mock() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);

        let err = service.hpttl("", vec!["f1".into()]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.hpttl("key", vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let result = service.hpttl("key", vec!["f1".into()]).await.unwrap();
        assert_eq!(result, vec![-1]);
    }

    #[tokio::test]
    async fn test_hpersist_delegates_to_mock() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);

        let err = service.hpersist("", vec!["f1".into()]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.hpersist("key", vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let result = service.hpersist("key", vec!["f1".into()]).await.unwrap();
        assert_eq!(result, vec![1]);
    }

    // --- Redis 8.0+ command tests ---

    #[tokio::test]
    async fn test_hgetex_empty_key() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let err = service
            .hgetex("", vec!["f1".into()], None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hgetex_empty_fields() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let err = service.hgetex("key", vec![], None).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hgetex_success() {
        let repo = Arc::new(MockHashRepository::new());
        repo.insert("key", "f1", "v1");
        let service = HashService::new_with_repository(repo);
        let result = service
            .hgetex("key", vec!["f1".into(), "missing".into()], None)
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].as_deref(), Some("v1"));
        assert!(result[1].is_none());
    }

    #[tokio::test]
    async fn test_hgetex_with_expiration() {
        use crate::domain::repositories::HashExpiration;
        let repo = Arc::new(MockHashRepository::new());
        repo.insert("key", "f1", "v1");
        let service = HashService::new_with_repository(repo);
        let result = service
            .hgetex("key", vec!["f1".into()], Some(HashExpiration::Ex(60)))
            .await
            .unwrap();
        assert_eq!(result[0].as_deref(), Some("v1"));
    }

    #[tokio::test]
    async fn test_hsetex_empty_key() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let err = service
            .hsetex("", vec![("f1".into(), "v1".into())], None, None)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hsetex_empty_fields() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let err = service.hsetex("key", vec![], None, None).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hsetex_success() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let result = service
            .hsetex(
                "key",
                vec![("f1".into(), "v1".into()), ("f2".into(), "v2".into())],
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(result, 2);
    }

    #[tokio::test]
    async fn test_hsetex_with_fnx_condition() {
        use crate::domain::repositories::HSetExCondition;
        let repo = Arc::new(MockHashRepository::new());
        repo.insert("key", "f1", "old");
        let service = HashService::new_with_repository(repo);
        // FNX: only set if field does NOT exist
        let result = service
            .hsetex(
                "key",
                vec![("f1".into(), "new".into()), ("f2".into(), "v2".into())],
                Some(HSetExCondition::FNX),
                None,
            )
            .await
            .unwrap();
        // f1 exists so not set, f2 doesn't exist so set
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_hsetex_with_fxx_condition() {
        use crate::domain::repositories::HSetExCondition;
        let repo = Arc::new(MockHashRepository::new());
        repo.insert("key", "f1", "old");
        let service = HashService::new_with_repository(repo);
        // FXX: only set if field DOES exist
        let result = service
            .hsetex(
                "key",
                vec![("f1".into(), "new".into()), ("f2".into(), "v2".into())],
                Some(HSetExCondition::FXX),
                None,
            )
            .await
            .unwrap();
        // f1 exists so set, f2 doesn't exist so not set
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_hgetdel_empty_key() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let err = service.hgetdel("", vec!["f1".into()]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hgetdel_empty_fields() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        let err = service.hgetdel("key", vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hgetdel_success() {
        let repo = Arc::new(MockHashRepository::new());
        repo.insert("key", "f1", "v1");
        repo.insert("key", "f2", "v2");
        let service = HashService::new_with_repository(repo.clone());
        let result = service
            .hgetdel("key", vec!["f1".into(), "missing".into()])
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].as_deref(), Some("v1"));
        assert!(result[1].is_none());
        // Verify f1 was deleted
        let remaining = service.hgetall("key").await.unwrap();
        assert!(!remaining.contains_key("f1"));
        assert!(remaining.contains_key("f2"));
    }

    #[tokio::test]
    async fn test_hexpire_with_condition() {
        let repo = Arc::new(MockHashRepository::new());
        let service = HashService::new_with_repository(repo);
        use crate::domain::repositories::ExpireCondition;
        let result = service
            .hexpire("key", 10, vec!["f1".into()], Some(ExpireCondition::NX))
            .await
            .unwrap();
        assert_eq!(result, vec![1]);

        let result = service
            .hexpire("key", 10, vec!["f1".into()], Some(ExpireCondition::XX))
            .await
            .unwrap();
        assert_eq!(result, vec![1]);

        let result = service
            .hexpire("key", 10, vec!["f1".into()], Some(ExpireCondition::GT))
            .await
            .unwrap();
        assert_eq!(result, vec![1]);

        let result = service
            .hexpire("key", 10, vec!["f1".into()], Some(ExpireCondition::LT))
            .await
            .unwrap();
        assert_eq!(result, vec![1]);
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

    async fn create_service() -> Option<(ContainerAsync<Redis>, HashService)> {
        let (container, redis_url) = start_redis().await?;
        let pool = Arc::new(
            crate::infrastructure::redis::connection::InstrumentedPool::new_for_tests_with_url(
                &redis_url,
            )
            .unwrap(),
        );
        let service = HashService::new(pool);
        Some((container, service))
    }

    /// Returns true if the error indicates the command is not supported (Redis < 7.4).
    fn is_unsupported_command(err: &CacheError) -> bool {
        let msg = format!("{err:?}");
        msg.contains("unknown command") || msg.contains("ERR")
    }

    #[tokio::test]
    async fn test_hexpire_then_httl() {
        let Some((_container, service)) = create_service().await else {
            return;
        };

        // Set up hash fields
        service
            .hset(
                "myhash",
                vec![
                    ("f1".to_string(), "v1".to_string()),
                    ("f2".to_string(), "v2".to_string()),
                ],
            )
            .await
            .unwrap();

        // Try HEXPIRE - may fail if Redis < 7.4
        let result = service
            .hexpire("myhash", 60, vec!["f1".to_string(), "f2".to_string()], None)
            .await;

        match result {
            Ok(values) => {
                assert_eq!(values.len(), 2);
                // 1 means expiration was set
                assert_eq!(values[0], 1);
                assert_eq!(values[1], 1);

                // Now check TTL
                let ttl_result = service
                    .httl("myhash", vec!["f1".to_string(), "f2".to_string()])
                    .await
                    .unwrap();
                assert_eq!(ttl_result.len(), 2);
                // TTL should be > 0 and <= 60
                assert!(ttl_result[0] > 0 && ttl_result[0] <= 60);
                assert!(ttl_result[1] > 0 && ttl_result[1] <= 60);
            }
            Err(e) if is_unsupported_command(&e) => {
                // Redis < 7.4, skip gracefully
            }
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_hexpire_with_nx_condition() {
        let Some((_container, service)) = create_service().await else {
            return;
        };

        service
            .hset("myhash_nx", vec![("f1".to_string(), "v1".to_string())])
            .await
            .unwrap();

        let result = service
            .hexpire(
                "myhash_nx",
                60,
                vec!["f1".to_string()],
                Some(ExpireCondition::NX),
            )
            .await;

        match result {
            Ok(values) => {
                // NX on field without expiration should succeed
                assert_eq!(values, vec![1]);

                // Try NX again - should fail since expiry already exists
                let result2 = service
                    .hexpire(
                        "myhash_nx",
                        120,
                        vec!["f1".to_string()],
                        Some(ExpireCondition::NX),
                    )
                    .await
                    .unwrap();
                // 0 means condition not met
                assert_eq!(result2, vec![0]);
            }
            Err(e) if is_unsupported_command(&e) => {}
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_hpersist_removes_expiration() {
        let Some((_container, service)) = create_service().await else {
            return;
        };

        service
            .hset("myhash_persist", vec![("f1".to_string(), "v1".to_string())])
            .await
            .unwrap();

        let result = service
            .hexpire("myhash_persist", 60, vec!["f1".to_string()], None)
            .await;

        match result {
            Ok(values) => {
                assert_eq!(values, vec![1]);

                // Now persist (remove expiration)
                let persist_result = service
                    .hpersist("myhash_persist", vec!["f1".to_string()])
                    .await
                    .unwrap();
                assert_eq!(persist_result, vec![1]);

                // TTL should now be -1 (no expiration)
                let ttl_result = service
                    .httl("myhash_persist", vec!["f1".to_string()])
                    .await
                    .unwrap();
                assert_eq!(ttl_result, vec![-1]);
            }
            Err(e) if is_unsupported_command(&e) => {}
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_hpexpire_and_hpttl() {
        let Some((_container, service)) = create_service().await else {
            return;
        };

        service
            .hset("myhash_ms", vec![("f1".to_string(), "v1".to_string())])
            .await
            .unwrap();

        let result = service
            .hpexpire("myhash_ms", 60000, vec!["f1".to_string()], None)
            .await;

        match result {
            Ok(values) => {
                assert_eq!(values, vec![1]);

                let pttl_result = service
                    .hpttl("myhash_ms", vec!["f1".to_string()])
                    .await
                    .unwrap();
                assert_eq!(pttl_result.len(), 1);
                assert!(pttl_result[0] > 0 && pttl_result[0] <= 60000);
            }
            Err(e) if is_unsupported_command(&e) => {}
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_hexpire_at_and_hexpire_time() {
        let Some((_container, service)) = create_service().await else {
            return;
        };

        service
            .hset("myhash_at", vec![("f1".to_string(), "v1".to_string())])
            .await
            .unwrap();

        // Set expiry to a future timestamp
        let future_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + 3600;

        let result = service
            .hexpire_at("myhash_at", future_ts, vec!["f1".to_string()], None)
            .await;

        match result {
            Ok(values) => {
                assert_eq!(values, vec![1]);

                let expire_time_result = service
                    .hexpire_time("myhash_at", vec!["f1".to_string()])
                    .await
                    .unwrap();
                assert_eq!(expire_time_result.len(), 1);
                assert!(expire_time_result[0] > 0);
            }
            Err(e) if is_unsupported_command(&e) => {}
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_hpexpire_at_and_hpexpire_time() {
        let Some((_container, service)) = create_service().await else {
            return;
        };

        service
            .hset("myhash_pat", vec![("f1".to_string(), "v1".to_string())])
            .await
            .unwrap();

        let future_ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
            + 3600000;

        let result = service
            .hpexpire_at("myhash_pat", future_ts_ms, vec!["f1".to_string()], None)
            .await;

        match result {
            Ok(values) => {
                assert_eq!(values, vec![1]);

                let pexpire_time_result = service
                    .hpexpire_time("myhash_pat", vec!["f1".to_string()])
                    .await
                    .unwrap();
                assert_eq!(pexpire_time_result.len(), 1);
                assert!(pexpire_time_result[0] > 0);
            }
            Err(e) if is_unsupported_command(&e) => {}
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }
}
