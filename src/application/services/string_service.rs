//! String Service
//!
//! Business logic layer for string operations.

use std::sync::Arc;
use std::time::Duration;

use crate::domain::entities::{
    AppendResult, ExpiryMode, GetExOptions, MGetResult, RangeResult,
    SetOptions, SetRangeResult, SetResult, StringValue,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::StringRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisStringRepository;

/// Service for string operations
pub struct StringService {
    repository: RedisStringRepository,
}

impl StringService {
    /// Create a new StringService
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self {
            repository: RedisStringRepository::new(pool),
        }
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
    pub async fn set_nx(&self, key: &str, value: &str, ttl_seconds: Option<u64>) -> Result<bool, CacheError> {
        let ttl = ttl_seconds.map(Duration::from_secs);
        self.repository.set_nx(key, value, ttl).await
    }

    /// Set a key with expiration
    pub async fn set_ex(&self, key: &str, value: &str, ttl_seconds: u64) -> Result<(), CacheError> {
        self.repository.set_ex(key, value, Duration::from_secs(ttl_seconds)).await
    }

    /// Get multiple keys at once
    pub async fn mget(&self, keys: Vec<String>) -> Result<MGetResult, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys list cannot be empty".to_string()));
        }
        self.repository.mget(&keys).await
    }

    /// Set multiple key-value pairs at once
    pub async fn mset(&self, pairs: Vec<(String, String)>) -> Result<usize, CacheError> {
        if pairs.is_empty() {
            return Err(CacheError::InvalidInput("Pairs list cannot be empty".to_string()));
        }
        let count = pairs.len();
        self.repository.mset(&pairs).await?;
        Ok(count)
    }

    /// Set multiple key-value pairs only if none exist
    pub async fn mset_nx(&self, pairs: Vec<(String, String)>) -> Result<bool, CacheError> {
        if pairs.is_empty() {
            return Err(CacheError::InvalidInput("Pairs list cannot be empty".to_string()));
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
    pub async fn get_range(&self, key: &str, start: i64, end: i64) -> Result<RangeResult, CacheError> {
        self.repository.get_range(key, start, end).await
    }

    /// Set a substring at a specific offset
    pub async fn set_range(&self, key: &str, offset: i64, value: &str) -> Result<SetRangeResult, CacheError> {
        if offset < 0 {
            return Err(CacheError::InvalidInput("Offset cannot be negative".to_string()));
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
}
