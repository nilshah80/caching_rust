//! String Service
//!
//! Business logic layer for string operations.

use std::sync::Arc;
use std::time::Duration;

use crate::domain::entities::{
    AppendResult, ExpiryMode, GetExOptions, MGetResult, RangeResult, SetOptions, SetRangeResult,
    SetResult, StringValue,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::{LcsOptions, LcsResult, StringRepository};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisStringRepository;

/// Service for string operations
pub struct StringService {
    repository: Arc<dyn StringRepository>,
}

impl StringService {
    /// Create a new StringService
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisStringRepository::new(pool)))
    }

    /// Create a StringService with a custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn StringRepository>) -> Self {
        Self { repository }
    }

    /// Get a string value by key
    pub async fn get(&self, key: &str) -> Result<Option<StringValue>, CacheError> {
        self.repository.get(key).await
    }

    /// Set a string value with options
    #[allow(clippy::too_many_arguments)] // Mirrors Redis SET command options
    pub async fn set(
        &self,
        key: &str,
        value: &str,
        ttl_seconds: Option<u64>,
        ttl_ms: Option<u64>,
        nx: bool,
        xx: bool,
        get: bool,
        keep_ttl: bool,
    ) -> Result<SetResult, CacheError> {
        // Determine expiry mode and value
        let (expiry_mode, expiry_value) = if let Some(ms) = ttl_ms {
            (Some(ExpiryMode::Px), Some(ms))
        } else if let Some(secs) = ttl_seconds {
            (Some(ExpiryMode::Ex), Some(secs))
        } else {
            (None, None)
        };

        let options = SetOptions {
            nx,
            xx,
            get,
            expiry_mode,
            expiry_value,
            keep_ttl,
        };

        self.repository.set(key, value, options).await
    }

    /// Set a key only if it doesn't exist
    pub async fn set_nx(
        &self,
        key: &str,
        value: &str,
        ttl_seconds: Option<u64>,
    ) -> Result<bool, CacheError> {
        let ttl = ttl_seconds.map(Duration::from_secs);
        self.repository.set_nx(key, value, ttl).await
    }

    /// Set a key with expiration
    pub async fn set_ex(&self, key: &str, value: &str, ttl_seconds: u64) -> Result<(), CacheError> {
        self.repository
            .set_ex(key, value, Duration::from_secs(ttl_seconds))
            .await
    }

    /// Get multiple keys at once
    pub async fn mget(&self, keys: Vec<String>) -> Result<MGetResult, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput(
                "Keys list cannot be empty".to_string(),
            ));
        }
        self.repository.mget(&keys).await
    }

    /// Set multiple key-value pairs at once
    pub async fn mset(&self, pairs: Vec<(String, String)>) -> Result<usize, CacheError> {
        if pairs.is_empty() {
            return Err(CacheError::InvalidInput(
                "Pairs list cannot be empty".to_string(),
            ));
        }
        let count = pairs.len();
        self.repository.mset(&pairs).await?;
        Ok(count)
    }

    /// Set multiple key-value pairs only if none exist
    pub async fn mset_nx(&self, pairs: Vec<(String, String)>) -> Result<bool, CacheError> {
        if pairs.is_empty() {
            return Err(CacheError::InvalidInput(
                "Pairs list cannot be empty".to_string(),
            ));
        }
        self.repository.mset_nx(&pairs).await
    }

    /// Increment a key by 1
    pub async fn incr(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.incr(key).await
    }

    /// Increment a key by a specific amount
    pub async fn incr_by(&self, key: &str, delta: i64) -> Result<i64, CacheError> {
        self.repository.incr_by(key, delta).await
    }

    /// Increment a key by a float amount
    pub async fn incr_by_float(&self, key: &str, delta: f64) -> Result<f64, CacheError> {
        self.repository.incr_by_float(key, delta).await
    }

    /// Decrement a key by 1
    pub async fn decr(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.decr(key).await
    }

    /// Decrement a key by a specific amount
    pub async fn decr_by(&self, key: &str, delta: i64) -> Result<i64, CacheError> {
        self.repository.decr_by(key, delta).await
    }

    /// Append a value to an existing key
    pub async fn append(&self, key: &str, value: &str) -> Result<AppendResult, CacheError> {
        self.repository.append(key, value).await
    }

    /// Get the length of a string value
    pub async fn str_len(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.str_len(key).await
    }

    /// Get a substring of a string value
    pub async fn get_range(
        &self,
        key: &str,
        start: i64,
        end: i64,
    ) -> Result<RangeResult, CacheError> {
        self.repository.get_range(key, start, end).await
    }

    /// Set a substring at a specific offset
    pub async fn set_range(
        &self,
        key: &str,
        offset: i64,
        value: &str,
    ) -> Result<SetRangeResult, CacheError> {
        if offset < 0 {
            return Err(CacheError::InvalidInput(
                "Offset cannot be negative".to_string(),
            ));
        }
        self.repository.set_range(key, offset, value).await
    }

    /// Get a value and optionally update its expiration
    pub async fn get_ex(
        &self,
        key: &str,
        ttl_seconds: Option<u64>,
        ttl_ms: Option<u64>,
        persist: bool,
    ) -> Result<Option<String>, CacheError> {
        let (expiry_mode, expiry_value) = if persist {
            (None, None)
        } else if let Some(ms) = ttl_ms {
            (Some(ExpiryMode::Px), Some(ms))
        } else if let Some(secs) = ttl_seconds {
            (Some(ExpiryMode::Ex), Some(secs))
        } else {
            (None, None)
        };

        let options = GetExOptions {
            expiry_mode,
            expiry_value,
            persist,
        };

        self.repository.get_ex(key, options).await
    }

    /// Get a value and delete the key
    pub async fn get_del(&self, key: &str) -> Result<Option<String>, CacheError> {
        self.repository.get_del(key).await
    }

    /// Compute the Longest Common Subsequence of two string keys (Redis 7.0+)
    pub async fn lcs(
        &self,
        key1: &str,
        key2: &str,
        options: LcsOptions,
    ) -> Result<LcsResult, CacheError> {
        if key1.is_empty() {
            return Err(CacheError::InvalidInput(
                "key1 must not be empty".to_string(),
            ));
        }
        if key2.is_empty() {
            return Err(CacheError::InvalidInput(
                "key2 must not be empty".to_string(),
            ));
        }

        // If idx is false, ignore idx-only options
        let effective_options = if options.idx {
            options
        } else {
            LcsOptions {
                len: options.len,
                idx: false,
                min_match_len: None,
                with_match_len: false,
            }
        };

        self.repository.lcs(key1, key2, effective_options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::redis::connection::InstrumentedPool;
    use crate::test_support::MockStringRepository;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[tokio::test]
    async fn test_string_service_validations() {
        let repo = Arc::new(MockStringRepository::new());
        let service = StringService::new_with_repository(repo);

        let err = service.mget(vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.mset(vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.mset_nx(vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.set_range("k", -1, "v").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[derive(Default)]
    struct CaptureRepo {
        last_set: Mutex<Option<SetOptions>>,
        last_getex: Mutex<Option<GetExOptions>>,
        fail_mset: Mutex<bool>,
    }

    #[async_trait]
    impl StringRepository for CaptureRepo {
        async fn get(&self, _key: &str) -> Result<Option<StringValue>, CacheError> {
            Ok(None)
        }

        async fn set(
            &self,
            _key: &str,
            _value: &str,
            options: SetOptions,
        ) -> Result<SetResult, CacheError> {
            *self.last_set.lock().expect("lock") = Some(options);
            Ok(SetResult {
                key: "k".to_string(),
                success: true,
                previous_value: None,
            })
        }

        async fn set_nx(
            &self,
            _key: &str,
            _value: &str,
            _ttl: Option<Duration>,
        ) -> Result<bool, CacheError> {
            Ok(true)
        }

        async fn set_ex(&self, _key: &str, _value: &str, _ttl: Duration) -> Result<(), CacheError> {
            Ok(())
        }

        async fn mget(&self, _keys: &[String]) -> Result<MGetResult, CacheError> {
            Ok(MGetResult {
                found: std::collections::HashMap::new(),
                missing: Vec::new(),
            })
        }

        async fn mset(&self, _pairs: &[(String, String)]) -> Result<(), CacheError> {
            if *self.fail_mset.lock().expect("lock") {
                return Err(CacheError::Internal("mset failed".to_string()));
            }
            Ok(())
        }

        async fn mset_nx(&self, _pairs: &[(String, String)]) -> Result<bool, CacheError> {
            Ok(true)
        }

        async fn incr(&self, _key: &str) -> Result<i64, CacheError> {
            Ok(1)
        }

        async fn incr_by(&self, _key: &str, _delta: i64) -> Result<i64, CacheError> {
            Ok(1)
        }

        async fn incr_by_float(&self, _key: &str, _delta: f64) -> Result<f64, CacheError> {
            Ok(1.0)
        }

        async fn decr(&self, _key: &str) -> Result<i64, CacheError> {
            Ok(1)
        }

        async fn decr_by(&self, _key: &str, _delta: i64) -> Result<i64, CacheError> {
            Ok(1)
        }

        async fn append(&self, _key: &str, _value: &str) -> Result<AppendResult, CacheError> {
            Ok(AppendResult {
                key: "k".to_string(),
                new_length: 1,
            })
        }

        async fn str_len(&self, _key: &str) -> Result<i64, CacheError> {
            Ok(1)
        }

        async fn get_range(
            &self,
            _key: &str,
            start: i64,
            end: i64,
        ) -> Result<RangeResult, CacheError> {
            Ok(RangeResult {
                key: "k".to_string(),
                value: "v".to_string(),
                start,
                end,
            })
        }

        async fn set_range(
            &self,
            _key: &str,
            _offset: i64,
            _value: &str,
        ) -> Result<SetRangeResult, CacheError> {
            Ok(SetRangeResult {
                key: "k".to_string(),
                new_length: 1,
            })
        }

        async fn get_ex(
            &self,
            _key: &str,
            options: GetExOptions,
        ) -> Result<Option<String>, CacheError> {
            *self.last_getex.lock().expect("lock") = Some(options);
            Ok(Some("v".to_string()))
        }

        async fn get_del(&self, _key: &str) -> Result<Option<String>, CacheError> {
            Ok(Some("v".to_string()))
        }

        async fn lcs(
            &self,
            _key1: &str,
            _key2: &str,
            _options: LcsOptions,
        ) -> Result<LcsResult, CacheError> {
            Ok(LcsResult::String("abc".to_string()))
        }
    }

    #[tokio::test]
    async fn test_string_service_option_building() {
        let repo = Arc::new(CaptureRepo::default());
        let service = StringService::new_with_repository(repo.clone());

        service
            .set("k", "v", Some(10), Some(5), false, false, false, false)
            .await
            .expect("set");
        let options = repo
            .last_set
            .lock()
            .expect("lock")
            .clone()
            .expect("set options");
        assert!(matches!(options.expiry_mode, Some(ExpiryMode::Px)));
        assert_eq!(options.expiry_value, Some(5));

        service
            .get_ex("k", Some(10), None, true)
            .await
            .expect("get_ex");
        let options = repo
            .last_getex
            .lock()
            .expect("lock")
            .clone()
            .expect("getex options");
        assert!(options.persist);
        assert!(options.expiry_mode.is_none());
    }

    #[tokio::test]
    async fn test_string_service_mset_error() {
        let repo = Arc::new(CaptureRepo::default());
        *repo.fail_mset.lock().expect("lock") = true;
        let service = StringService::new_with_repository(repo);

        let err = service
            .mset(vec![("k".to_string(), "v".to_string())])
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::Internal(_)));
    }

    #[tokio::test]
    async fn test_string_service_basic_operations() {
        let repo = Arc::new(MockStringRepository::new());
        let service = StringService::new_with_repository(repo.clone());

        let first = service.set_nx("k1", "v1", Some(5)).await.expect("set_nx");
        assert!(first);
        let second = service.set_nx("k1", "v2", None).await.expect("set_nx");
        assert!(!second);

        service.set_ex("k2", "v2", 10).await.expect("set_ex");
        let value = service.get("k2").await.expect("get").expect("value");
        assert_eq!(value.value, "v2");

        let count = service
            .mset(vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ])
            .await
            .expect("mset");
        assert_eq!(count, 2);

        let inc = service.incr("counter").await.expect("incr");
        let dec = service.decr("counter").await.expect("decr");
        assert_eq!(inc, 1);
        assert_eq!(dec, 0);
    }

    #[test]
    fn test_string_service_new() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = StringService::new(pool);
    }

    #[tokio::test]
    async fn test_lcs_empty_key1() {
        let repo = Arc::new(MockStringRepository::new());
        let service = StringService::new_with_repository(repo);

        let err = service
            .lcs("", "key2", LcsOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(ref msg) if msg.contains("key1")));
    }

    #[tokio::test]
    async fn test_lcs_empty_key2() {
        let repo = Arc::new(MockStringRepository::new());
        let service = StringService::new_with_repository(repo);

        let err = service
            .lcs("key1", "", LcsOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(ref msg) if msg.contains("key2")));
    }

    #[tokio::test]
    async fn test_lcs_valid_call_delegates() {
        let repo = Arc::new(MockStringRepository::new());
        repo.insert("k1", "ohmytext");
        repo.insert("k2", "mynewtext");
        let service = StringService::new_with_repository(repo);

        let result = service
            .lcs("k1", "k2", LcsOptions::default())
            .await
            .expect("lcs");
        match result {
            LcsResult::String(s) => assert_eq!(s, "mytext"),
            _ => panic!("Expected LcsResult::String"),
        }
    }

    #[tokio::test]
    async fn test_lcs_len_mode() {
        let repo = Arc::new(MockStringRepository::new());
        repo.insert("k1", "ohmytext");
        repo.insert("k2", "mynewtext");
        let service = StringService::new_with_repository(repo);

        let result = service
            .lcs(
                "k1",
                "k2",
                LcsOptions {
                    len: true,
                    ..Default::default()
                },
            )
            .await
            .expect("lcs");
        match result {
            LcsResult::Length(n) => assert_eq!(n, 6),
            _ => panic!("Expected LcsResult::Length"),
        }
    }

    #[tokio::test]
    async fn test_lcs_idx_mode() {
        let repo = Arc::new(MockStringRepository::new());
        repo.insert("k1", "ohmytext");
        repo.insert("k2", "mynewtext");
        let service = StringService::new_with_repository(repo);

        let result = service
            .lcs(
                "k1",
                "k2",
                LcsOptions {
                    idx: true,
                    with_match_len: true,
                    ..Default::default()
                },
            )
            .await
            .expect("lcs");
        match result {
            LcsResult::Matches(m) => {
                assert_eq!(m.len, 6);
                assert!(!m.matches.is_empty());
                // First match should be "my" at positions k1[2..3], k2[0..1]
                assert!(m.matches[0].match_len.is_some());
            }
            _ => panic!("Expected LcsResult::Matches"),
        }
    }

    #[tokio::test]
    async fn test_lcs_idx_strips_options_when_not_idx() {
        // When idx is false, min_match_len and with_match_len should be ignored
        let repo = Arc::new(MockStringRepository::new());
        repo.insert("k1", "abc");
        repo.insert("k2", "abc");
        let service = StringService::new_with_repository(repo);

        let result = service
            .lcs(
                "k1",
                "k2",
                LcsOptions {
                    idx: false,
                    min_match_len: Some(5),
                    with_match_len: true,
                    ..Default::default()
                },
            )
            .await
            .expect("lcs");
        // Should return string mode, not matches
        match result {
            LcsResult::String(s) => assert_eq!(s, "abc"),
            _ => panic!("Expected LcsResult::String when idx is false"),
        }
    }
}
