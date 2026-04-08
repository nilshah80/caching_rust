//! Vector Repository Trait
//!
//! Defines the interface for Vector Sets operations.

use async_trait::async_trait;

use crate::domain::entities::{VectorAddResult, VectorInfo, VectorRangeResult, VectorSimResult};
use crate::domain::errors::CacheError;

/// Repository trait for Vector Sets operations
#[async_trait]
pub trait VectorRepository: Send + Sync {
    /// Add element(s) with vectors (VADD)
    async fn vadd(
        &self,
        key: &str,
        items: Vec<(String, Vec<f32>)>,
    ) -> Result<VectorAddResult, CacheError>;

    /// Remove element from vector set (VREM)
    async fn vrem(&self, key: &str, items: Vec<String>) -> Result<u64, CacheError>;

    /// Query by vector similarity (VSIM)
    async fn vsim(
        &self,
        key: &str,
        vector: Vec<f32>,
        k: u64,
    ) -> Result<VectorSimResult, CacheError>;

    /// Count elements in vector set (VCARD)
    async fn vcard(&self, key: &str) -> Result<u64, CacheError>;

    /// Get vector dimensionality (VDIM)
    async fn vdim(&self, key: &str) -> Result<u64, CacheError>;

    /// Get element's embedding vector (VEMB)
    async fn vemb(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<Vec<Option<Vec<f32>>>, CacheError>;

    /// Check element membership (VISMEMBER)
    async fn vismember(&self, key: &str, items: Vec<String>) -> Result<Vec<bool>, CacheError>;

    /// Get HNSW graph neighbors grouped by layer (VLINKS)
    async fn vlinks(&self, key: &str, item: &str) -> Result<Vec<Vec<String>>, CacheError>;

    /// Get random member(s) (VRANDMEMBER)
    async fn vrandmember(&self, key: &str, count: i64) -> Result<Vec<String>, CacheError>;

    /// Range query (VRANGE)
    async fn vrange(
        &self,
        key: &str,
        start: &str,
        end: &str,
        count: Option<i64>,
    ) -> Result<VectorRangeResult, CacheError>;

    /// Get vector set metadata (VINFO)
    async fn vinfo(&self, key: &str) -> Result<VectorInfo, CacheError>;

    /// Get JSON attributes (VGETATTR)
    async fn vgetattr(&self, key: &str, item: &str) -> Result<Option<String>, CacheError>;

    /// Set JSON attributes (VSETATTR)
    async fn vsetattr(&self, key: &str, item: &str, attributes: &str) -> Result<bool, CacheError>;
}
