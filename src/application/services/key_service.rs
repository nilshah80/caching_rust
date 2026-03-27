//! Key Service
//!
//! Business logic layer for key management operations.

use std::sync::Arc;

use crate::domain::entities::{
    CopyOptions, CopyResult, DeleteResult, DumpResult, ExistsResult, ExpireOptions, ExpireResult,
    KeyInfo, PersistResult, RandomKeyResult, RenameResult, ScanResult, TouchResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::{KeyRepository, SortOptions};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisKeyRepository;

/// Service for key management operations
pub struct KeyService {
    repository: Arc<dyn KeyRepository>,
}

impl KeyService {
    /// Create a new KeyService
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisKeyRepository::new(pool)))
    }

    /// Create a KeyService with a custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn KeyRepository>) -> Self {
        Self { repository }
    }

    /// Delete one or more keys
    pub async fn delete(&self, keys: Vec<String>) -> Result<DeleteResult, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput(
                "Keys list cannot be empty".to_string(),
            ));
        }
        self.repository.delete(&keys).await
    }

    /// Check if one or more keys exist
    pub async fn exists(&self, keys: Vec<String>) -> Result<ExistsResult, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput(
                "Keys list cannot be empty".to_string(),
            ));
        }
        self.repository.exists(&keys).await
    }

    /// Set expiration on a key in seconds
    pub async fn expire(
        &self,
        key: &str,
        seconds: i64,
        nx: bool,
        xx: bool,
        gt: bool,
        lt: bool,
    ) -> Result<ExpireResult, CacheError> {
        if seconds < 0 {
            return Err(CacheError::InvalidInput(
                "Seconds cannot be negative".to_string(),
            ));
        }
        let options = ExpireOptions { nx, xx, gt, lt };
        self.repository.expire(key, seconds, options).await
    }

    /// Set expiration on a key at a Unix timestamp (seconds)
    pub async fn expire_at(
        &self,
        key: &str,
        timestamp: i64,
        nx: bool,
        xx: bool,
        gt: bool,
        lt: bool,
    ) -> Result<ExpireResult, CacheError> {
        let options = ExpireOptions { nx, xx, gt, lt };
        self.repository.expire_at(key, timestamp, options).await
    }

    /// Set expiration on a key in milliseconds
    pub async fn pexpire(
        &self,
        key: &str,
        milliseconds: i64,
        nx: bool,
        xx: bool,
        gt: bool,
        lt: bool,
    ) -> Result<ExpireResult, CacheError> {
        if milliseconds < 0 {
            return Err(CacheError::InvalidInput(
                "Milliseconds cannot be negative".to_string(),
            ));
        }
        let options = ExpireOptions { nx, xx, gt, lt };
        self.repository.pexpire(key, milliseconds, options).await
    }

    /// Set expiration on a key at a Unix timestamp (milliseconds)
    pub async fn pexpire_at(
        &self,
        key: &str,
        timestamp: i64,
        nx: bool,
        xx: bool,
        gt: bool,
        lt: bool,
    ) -> Result<ExpireResult, CacheError> {
        let options = ExpireOptions { nx, xx, gt, lt };
        self.repository.pexpire_at(key, timestamp, options).await
    }

    /// Get time-to-live in seconds
    pub async fn ttl(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.ttl(key).await
    }

    /// Get time-to-live in milliseconds
    pub async fn pttl(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.pttl(key).await
    }

    /// Remove expiration from a key
    pub async fn persist(&self, key: &str) -> Result<PersistResult, CacheError> {
        self.repository.persist(key).await
    }

    /// Get the type of a key
    pub async fn key_type(&self, key: &str) -> Result<String, CacheError> {
        self.repository.key_type(key).await
    }

    /// Rename a key
    pub async fn rename(&self, key: &str, new_key: &str) -> Result<RenameResult, CacheError> {
        if key == new_key {
            return Err(CacheError::InvalidInput(
                "New key must be different from old key".to_string(),
            ));
        }
        self.repository.rename(key, new_key).await
    }

    /// Rename a key only if new key doesn't exist
    pub async fn rename_nx(&self, key: &str, new_key: &str) -> Result<RenameResult, CacheError> {
        if key == new_key {
            return Err(CacheError::InvalidInput(
                "New key must be different from old key".to_string(),
            ));
        }
        self.repository.rename_nx(key, new_key).await
    }

    /// Copy a key to a new key
    pub async fn copy(
        &self,
        source: &str,
        destination: &str,
        db: Option<i64>,
        replace: bool,
    ) -> Result<CopyResult, CacheError> {
        let options = CopyOptions { db, replace };
        self.repository.copy(source, destination, options).await
    }

    /// Scan keys matching a pattern
    pub async fn scan(
        &self,
        cursor: u64,
        pattern: Option<String>,
        count: Option<u64>,
        key_type: Option<String>,
    ) -> Result<ScanResult, CacheError> {
        self.repository
            .scan(cursor, pattern.as_deref(), count, key_type.as_deref())
            .await
    }

    /// Find all keys matching a pattern (use with caution)
    pub async fn keys(&self, pattern: &str) -> Result<Vec<String>, CacheError> {
        if pattern.is_empty() {
            return Err(CacheError::InvalidInput(
                "Pattern cannot be empty".to_string(),
            ));
        }
        self.repository.keys(pattern).await
    }

    /// Return a random key from the database
    pub async fn random_key(&self) -> Result<RandomKeyResult, CacheError> {
        self.repository.random_key().await
    }

    /// Update the last access time of keys
    pub async fn touch(&self, keys: Vec<String>) -> Result<TouchResult, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput(
                "Keys list cannot be empty".to_string(),
            ));
        }
        self.repository.touch(&keys).await
    }

    /// Delete keys asynchronously (non-blocking)
    pub async fn unlink(&self, keys: Vec<String>) -> Result<DeleteResult, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput(
                "Keys list cannot be empty".to_string(),
            ));
        }
        self.repository.unlink(&keys).await
    }

    /// Serialize a key's value for backup/migration
    pub async fn dump(&self, key: &str) -> Result<DumpResult, CacheError> {
        self.repository.dump(key).await
    }

    /// Restore a serialized value to a key
    pub async fn restore(
        &self,
        key: &str,
        ttl: i64,
        data: &[u8],
        replace: bool,
    ) -> Result<bool, CacheError> {
        self.repository.restore(key, ttl, data, replace).await
    }

    /// Get comprehensive information about a key
    pub async fn key_info(&self, key: &str) -> Result<KeyInfo, CacheError> {
        self.repository.key_info(key).await
    }

    /// Get the absolute Unix timestamp when a key will expire (Redis 7.0+)
    pub async fn expire_time(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.expire_time(key).await
    }

    /// Get the absolute Unix timestamp in milliseconds when a key will expire (Redis 7.0+)
    pub async fn pexpire_time(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.pexpire_time(key).await
    }

    /// Get object encoding
    pub async fn object_encoding(&self, key: &str) -> Result<Option<String>, CacheError> {
        self.repository.object_encoding(key).await
    }

    /// Get object idle time
    pub async fn object_idletime(&self, key: &str) -> Result<Option<u64>, CacheError> {
        self.repository.object_idletime(key).await
    }

    /// Get object reference count
    pub async fn object_refcount(&self, key: &str) -> Result<Option<u64>, CacheError> {
        self.repository.object_refcount(key).await
    }

    /// Get object frequency (LFU)
    pub async fn object_freq(&self, key: &str) -> Result<Option<u64>, CacheError> {
        self.repository.object_freq(key).await
    }

    /// Sort elements in a list, set, or sorted set
    pub async fn sort(
        &self,
        key: &str,
        options: SortOptions,
    ) -> Result<Vec<Option<String>>, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        self.repository.sort(key, options).await
    }

    /// Sort elements and store the result
    pub async fn sort_store(
        &self,
        key: &str,
        destination: &str,
        options: SortOptions,
    ) -> Result<i64, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if destination.is_empty() {
            return Err(CacheError::InvalidInput(
                "Destination key cannot be empty".to_string(),
            ));
        }
        self.repository.sort_store(key, destination, options).await
    }

    /// Read-only sort (Redis 7.0+)
    pub async fn sort_ro(
        &self,
        key: &str,
        options: SortOptions,
    ) -> Result<Vec<Option<String>>, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        self.repository.sort_ro(key, options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockKeyRepository {
        keys: Mutex<std::collections::HashMap<String, String>>,
    }

    #[async_trait]
    impl KeyRepository for MockKeyRepository {
        async fn delete(&self, keys: &[String]) -> Result<DeleteResult, CacheError> {
            let mut store = self.keys.lock().unwrap();
            let mut deleted = Vec::new();
            let mut not_found = Vec::new();
            for key in keys {
                if store.remove(key).is_some() {
                    deleted.push(key.clone());
                } else {
                    not_found.push(key.clone());
                }
            }
            let count = deleted.len();
            Ok(DeleteResult {
                deleted,
                not_found,
                count,
            })
        }

        async fn exists(&self, keys: &[String]) -> Result<ExistsResult, CacheError> {
            let store = self.keys.lock().unwrap();
            let mut existing = Vec::new();
            let mut missing = Vec::new();
            for key in keys {
                if store.contains_key(key) {
                    existing.push(key.clone());
                } else {
                    missing.push(key.clone());
                }
            }
            let count = existing.len();
            Ok(ExistsResult {
                existing,
                missing,
                count,
            })
        }

        async fn expire(
            &self,
            key: &str,
            seconds: i64,
            _options: ExpireOptions,
        ) -> Result<ExpireResult, CacheError> {
            let store = self.keys.lock().unwrap();
            let success = store.contains_key(key);
            Ok(ExpireResult {
                key: key.to_string(),
                success,
                new_ttl: if success { Some(seconds) } else { None },
            })
        }

        async fn expire_at(
            &self,
            key: &str,
            _timestamp: i64,
            _options: ExpireOptions,
        ) -> Result<ExpireResult, CacheError> {
            let store = self.keys.lock().unwrap();
            let success = store.contains_key(key);
            Ok(ExpireResult {
                key: key.to_string(),
                success,
                new_ttl: None,
            })
        }

        async fn pexpire(
            &self,
            key: &str,
            milliseconds: i64,
            _options: ExpireOptions,
        ) -> Result<ExpireResult, CacheError> {
            let store = self.keys.lock().unwrap();
            let success = store.contains_key(key);
            Ok(ExpireResult {
                key: key.to_string(),
                success,
                new_ttl: if success {
                    Some(milliseconds / 1000)
                } else {
                    None
                },
            })
        }

        async fn pexpire_at(
            &self,
            key: &str,
            _timestamp: i64,
            _options: ExpireOptions,
        ) -> Result<ExpireResult, CacheError> {
            let store = self.keys.lock().unwrap();
            let success = store.contains_key(key);
            Ok(ExpireResult {
                key: key.to_string(),
                success,
                new_ttl: None,
            })
        }

        async fn ttl(&self, key: &str) -> Result<i64, CacheError> {
            let store = self.keys.lock().unwrap();
            Ok(if store.contains_key(key) { -1 } else { -2 })
        }

        async fn pttl(&self, key: &str) -> Result<i64, CacheError> {
            let store = self.keys.lock().unwrap();
            Ok(if store.contains_key(key) { -1 } else { -2 })
        }

        async fn persist(&self, key: &str) -> Result<PersistResult, CacheError> {
            let store = self.keys.lock().unwrap();
            Ok(PersistResult {
                key: key.to_string(),
                success: store.contains_key(key),
            })
        }

        async fn key_type(&self, key: &str) -> Result<String, CacheError> {
            let store = self.keys.lock().unwrap();
            Ok(if store.contains_key(key) {
                "string".to_string()
            } else {
                "none".to_string()
            })
        }

        async fn rename(&self, key: &str, new_key: &str) -> Result<RenameResult, CacheError> {
            let mut store = self.keys.lock().unwrap();
            if let Some(value) = store.remove(key) {
                store.insert(new_key.to_string(), value);
                Ok(RenameResult {
                    old_key: key.to_string(),
                    new_key: new_key.to_string(),
                    success: true,
                })
            } else {
                Err(CacheError::KeyNotFound(key.to_string()))
            }
        }

        async fn rename_nx(&self, key: &str, new_key: &str) -> Result<RenameResult, CacheError> {
            let mut store = self.keys.lock().unwrap();
            if !store.contains_key(key) {
                return Err(CacheError::KeyNotFound(key.to_string()));
            }
            if store.contains_key(new_key) {
                return Ok(RenameResult {
                    old_key: key.to_string(),
                    new_key: new_key.to_string(),
                    success: false,
                });
            }
            let value = store.remove(key).unwrap();
            store.insert(new_key.to_string(), value);
            Ok(RenameResult {
                old_key: key.to_string(),
                new_key: new_key.to_string(),
                success: true,
            })
        }

        async fn copy(
            &self,
            source: &str,
            destination: &str,
            options: CopyOptions,
        ) -> Result<CopyResult, CacheError> {
            let mut store = self.keys.lock().unwrap();
            if let Some(value) = store.get(source).cloned() {
                if store.contains_key(destination) && !options.replace {
                    return Ok(CopyResult {
                        source: source.to_string(),
                        destination: destination.to_string(),
                        success: false,
                    });
                }
                store.insert(destination.to_string(), value);
                Ok(CopyResult {
                    source: source.to_string(),
                    destination: destination.to_string(),
                    success: true,
                })
            } else {
                Ok(CopyResult {
                    source: source.to_string(),
                    destination: destination.to_string(),
                    success: false,
                })
            }
        }

        async fn scan(
            &self,
            _cursor: u64,
            pattern: Option<&str>,
            count: Option<u64>,
            _key_type: Option<&str>,
        ) -> Result<ScanResult, CacheError> {
            let store = self.keys.lock().unwrap();
            let keys: Vec<String> = store
                .keys()
                .filter(|k| pattern.is_none_or(|p| k.contains(&p.replace("*", ""))))
                .take(count.unwrap_or(10) as usize)
                .cloned()
                .collect();
            Ok(ScanResult {
                cursor: 0,
                count: keys.len(),
                keys,
            })
        }

        async fn keys(&self, pattern: &str) -> Result<Vec<String>, CacheError> {
            let store = self.keys.lock().unwrap();
            let keys: Vec<String> = store
                .keys()
                .filter(|k| k.contains(&pattern.replace("*", "")))
                .cloned()
                .collect();
            Ok(keys)
        }

        async fn random_key(&self) -> Result<RandomKeyResult, CacheError> {
            let store = self.keys.lock().unwrap();
            Ok(RandomKeyResult {
                key: store.keys().next().cloned(),
            })
        }

        async fn touch(&self, keys: &[String]) -> Result<TouchResult, CacheError> {
            let store = self.keys.lock().unwrap();
            let count = keys.iter().filter(|k| store.contains_key(*k)).count();
            Ok(TouchResult { count })
        }

        async fn unlink(&self, keys: &[String]) -> Result<DeleteResult, CacheError> {
            self.delete(keys).await
        }

        async fn dump(&self, key: &str) -> Result<DumpResult, CacheError> {
            let store = self.keys.lock().unwrap();
            Ok(DumpResult {
                key: key.to_string(),
                data: store.get(key).map(|_| "serialized".to_string()),
            })
        }

        async fn restore(
            &self,
            key: &str,
            _ttl: i64,
            _data: &[u8],
            replace: bool,
        ) -> Result<bool, CacheError> {
            let mut store = self.keys.lock().unwrap();
            if store.contains_key(key) && !replace {
                return Ok(false);
            }
            store.insert(key.to_string(), "restored".to_string());
            Ok(true)
        }

        async fn object_encoding(&self, key: &str) -> Result<Option<String>, CacheError> {
            let store = self.keys.lock().unwrap();
            Ok(if store.contains_key(key) {
                Some("embstr".to_string())
            } else {
                None
            })
        }

        async fn object_idletime(&self, key: &str) -> Result<Option<u64>, CacheError> {
            let store = self.keys.lock().unwrap();
            Ok(if store.contains_key(key) {
                Some(0)
            } else {
                None
            })
        }

        async fn object_refcount(&self, key: &str) -> Result<Option<u64>, CacheError> {
            let store = self.keys.lock().unwrap();
            Ok(if store.contains_key(key) {
                Some(1)
            } else {
                None
            })
        }

        async fn object_freq(&self, key: &str) -> Result<Option<u64>, CacheError> {
            let store = self.keys.lock().unwrap();
            Ok(if store.contains_key(key) {
                Some(0)
            } else {
                None
            })
        }

        async fn key_info(&self, key: &str) -> Result<KeyInfo, CacheError> {
            let store = self.keys.lock().unwrap();
            if store.contains_key(key) {
                Ok(KeyInfo::new(key.to_string(), "string".to_string(), -1))
            } else {
                Ok(KeyInfo::not_found(key.to_string()))
            }
        }

        async fn expire_time(&self, key: &str) -> Result<i64, CacheError> {
            let store = self.keys.lock().unwrap();
            if store.contains_key(key) {
                Ok(-1)
            } else {
                Ok(-2)
            }
        }

        async fn pexpire_time(&self, key: &str) -> Result<i64, CacheError> {
            let store = self.keys.lock().unwrap();
            if store.contains_key(key) {
                Ok(-1)
            } else {
                Ok(-2)
            }
        }

        async fn sort(
            &self,
            _key: &str,
            _options: SortOptions,
        ) -> Result<Vec<Option<String>>, CacheError> {
            Ok(vec![])
        }

        async fn sort_store(
            &self,
            _key: &str,
            _destination: &str,
            _options: SortOptions,
        ) -> Result<i64, CacheError> {
            Ok(0)
        }

        async fn sort_ro(
            &self,
            _key: &str,
            _options: SortOptions,
        ) -> Result<Vec<Option<String>>, CacheError> {
            Ok(vec![])
        }
    }

    fn mock_service() -> (KeyService, Arc<MockKeyRepository>) {
        let repo = Arc::new(MockKeyRepository::default());
        let service = KeyService::new_with_repository(repo.clone());
        (service, repo)
    }

    #[tokio::test]
    async fn test_key_service_validations() {
        let (service, _) = mock_service();

        // Empty keys list
        let err = service.delete(vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.exists(vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.touch(vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.unlink(vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Negative seconds
        let err = service
            .expire("key", -1, false, false, false, false)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .pexpire("key", -1, false, false, false, false)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Same key rename
        let err = service.rename("key", "key").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.rename_nx("key", "key").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Empty pattern
        let err = service.keys("").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_key_service_operations() {
        let (service, repo) = mock_service();

        // Add some keys
        repo.keys
            .lock()
            .unwrap()
            .insert("key1".to_string(), "value1".to_string());
        repo.keys
            .lock()
            .unwrap()
            .insert("key2".to_string(), "value2".to_string());

        // exists
        let result = service
            .exists(vec!["key1".to_string(), "key3".to_string()])
            .await
            .unwrap();
        assert_eq!(result.count, 1);
        assert!(result.existing.contains(&"key1".to_string()));
        assert!(result.missing.contains(&"key3".to_string()));

        // ttl
        let ttl = service.ttl("key1").await.unwrap();
        assert_eq!(ttl, -1);

        // key_type
        let key_type = service.key_type("key1").await.unwrap();
        assert_eq!(key_type, "string");

        // rename
        let result = service.rename("key1", "key1_renamed").await.unwrap();
        assert!(result.success);

        // delete
        let result = service
            .delete(vec!["key1_renamed".to_string()])
            .await
            .unwrap();
        assert_eq!(result.count, 1);

        // key_info
        let info = service.key_info("key2").await.unwrap();
        assert!(info.exists);
        assert_eq!(info.key_type, "string");
    }

    #[tokio::test]
    async fn test_key_service_additional_operations() {
        let (service, repo) = mock_service();

        repo.keys
            .lock()
            .unwrap()
            .insert("alpha".to_string(), "a".to_string());
        repo.keys
            .lock()
            .unwrap()
            .insert("beta".to_string(), "b".to_string());

        let result = service
            .expire_at("alpha", 100, false, false, false, false)
            .await
            .unwrap();
        assert!(result.success);

        let result = service
            .pexpire_at("alpha", 1000, false, false, false, false)
            .await
            .unwrap();
        assert!(result.success);

        let pttl = service.pttl("alpha").await.unwrap();
        assert_eq!(pttl, -1);

        let persist = service.persist("alpha").await.unwrap();
        assert!(persist.success);

        let rename = service.rename_nx("alpha", "beta").await.unwrap();
        assert!(!rename.success);

        let copy = service.copy("alpha", "gamma", None, false).await.unwrap();
        assert!(copy.success);

        let scan = service
            .scan(0, Some("a*".to_string()), Some(10), None)
            .await
            .unwrap();
        assert!(scan.keys.iter().any(|key| key.contains('a')));

        let keys = service.keys("a*").await.unwrap();
        assert!(!keys.is_empty());

        let random = service.random_key().await.unwrap();
        assert!(random.key.is_some());

        let touched = service
            .touch(vec!["alpha".to_string(), "missing".to_string()])
            .await
            .unwrap();
        assert_eq!(touched.count, 1);

        let unlink = service.unlink(vec!["gamma".to_string()]).await.unwrap();
        assert_eq!(unlink.count, 1);

        let dump = service.dump("beta").await.unwrap();
        assert!(dump.data.is_some());

        let restored = service.restore("beta", 0, b"value", false).await.unwrap();
        assert!(!restored);
        let restored = service.restore("beta", 0, b"value", true).await.unwrap();
        assert!(restored);

        let info = service.key_info("missing").await.unwrap();
        assert!(!info.exists);

        let expire_time = service.expire_time("alpha").await.unwrap();
        assert_eq!(expire_time, -1);

        let pexpire_time = service.pexpire_time("missing").await.unwrap();
        assert_eq!(pexpire_time, -2);

        let encoding = service.object_encoding("alpha").await.unwrap();
        assert_eq!(encoding.as_deref(), Some("embstr"));

        let idle_time = service.object_idletime("alpha").await.unwrap();
        assert_eq!(idle_time, Some(0));

        let ref_count = service.object_refcount("alpha").await.unwrap();
        assert_eq!(ref_count, Some(1));

        let freq = service.object_freq("alpha").await.unwrap();
        assert_eq!(freq, Some(0));
    }

    #[test]
    fn test_key_service_new() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = KeyService::new(pool);
    }
}
