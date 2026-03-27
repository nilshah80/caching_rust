use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::errors::CacheError;

/// Condition for hash field expiration commands (NX, XX, GT, LT).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpireCondition {
    /// Set expiry only when the field has no expiry
    NX,
    /// Set expiry only when the field has an existing expiry
    XX,
    /// Set expiry only when the new expiry is greater than current one
    GT,
    /// Set expiry only when the new expiry is less than current one
    LT,
}

impl ExpireCondition {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExpireCondition::NX => "NX",
            ExpireCondition::XX => "XX",
            ExpireCondition::GT => "GT",
            ExpireCondition::LT => "LT",
        }
    }
}

#[async_trait]
pub trait HashRepository: Send + Sync {
    async fn hget(&self, key: &str, field: &str) -> Result<Option<String>, CacheError>;
    async fn hset(&self, key: &str, pairs: Vec<(String, String)>) -> Result<i64, CacheError>;
    async fn hset_nx(&self, key: &str, field: &str, value: &str) -> Result<bool, CacheError>;
    async fn hgetall(&self, key: &str) -> Result<HashMap<String, String>, CacheError>;
    async fn hmget(&self, key: &str, fields: &[String]) -> Result<Vec<Option<String>>, CacheError>;
    async fn hmset(&self, key: &str, pairs: Vec<(String, String)>) -> Result<(), CacheError>;
    async fn hdel(&self, key: &str, fields: &[String]) -> Result<i64, CacheError>;
    async fn hexists(&self, key: &str, field: &str) -> Result<bool, CacheError>;
    async fn hkeys(&self, key: &str) -> Result<Vec<String>, CacheError>;
    async fn hvals(&self, key: &str) -> Result<Vec<String>, CacheError>;
    async fn hlen(&self, key: &str) -> Result<i64, CacheError>;
    async fn hincr_by(&self, key: &str, field: &str, delta: i64) -> Result<i64, CacheError>;
    async fn hincr_by_float(&self, key: &str, field: &str, delta: f64) -> Result<f64, CacheError>;
    async fn hstr_len(&self, key: &str, field: &str) -> Result<i64, CacheError>;
    async fn hrand_field(
        &self,
        key: &str,
        count: Option<i64>,
        with_values: bool,
    ) -> Result<Vec<String>, CacheError>;
    async fn hscan(
        &self,
        key: &str,
        cursor: u64,
        pattern: Option<String>,
        count: Option<u64>,
    ) -> Result<(u64, Vec<String>), CacheError>;

    // Hash field expiration commands (Redis 7.4+)
    async fn hexpire(
        &self,
        key: &str,
        seconds: i64,
        fields: &[String],
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError>;
    async fn hpexpire(
        &self,
        key: &str,
        milliseconds: i64,
        fields: &[String],
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError>;
    async fn hexpire_at(
        &self,
        key: &str,
        unix_time: i64,
        fields: &[String],
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError>;
    async fn hpexpire_at(
        &self,
        key: &str,
        unix_time_ms: i64,
        fields: &[String],
        condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError>;
    async fn hexpire_time(&self, key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError>;
    async fn hpexpire_time(&self, key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError>;
    async fn httl(&self, key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError>;
    async fn hpttl(&self, key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError>;
    async fn hpersist(&self, key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError>;
}
