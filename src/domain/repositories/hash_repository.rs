use std::collections::HashMap;

use async_trait::async_trait;

use crate::domain::errors::CacheError;

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
}
