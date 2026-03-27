//! List Repository Trait
//!
//! Abstract interface for list operations.

use async_trait::async_trait;
use std::time::Duration;

use crate::domain::errors::CacheError;

/// Direction for list move operations (LMOVE, BLMOVE)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListDirection {
    Left,
    Right,
}

impl ListDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            ListDirection::Left => "LEFT",
            ListDirection::Right => "RIGHT",
        }
    }
}

/// Position for LINSERT command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertPosition {
    Before,
    After,
}

impl InsertPosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            InsertPosition::Before => "BEFORE",
            InsertPosition::After => "AFTER",
        }
    }
}

/// Options for LPOS command
#[derive(Debug, Clone, Default)]
pub struct LPosOptions {
    /// Return up to COUNT matching indices
    pub count: Option<i64>,
    /// Starting rank for the search (0 = first, negative = from end)
    pub rank: Option<i64>,
    /// Maximum comparisons to perform
    pub max_len: Option<i64>,
}

/// Result from blocking pop operations
#[derive(Debug, Clone)]
pub struct BlockingPopResult {
    pub key: String,
    pub value: String,
}

/// Result from LMPOP/BLMPOP operations (multi-element pop)
#[derive(Debug, Clone)]
pub struct LMPopResult {
    pub key: String,
    pub elements: Vec<String>,
}

/// Repository trait for Redis list operations
#[async_trait]
pub trait ListRepository: Send + Sync {
    // ========== Non-blocking operations ==========

    /// LPUSH - Insert values at the head of the list
    async fn lpush(&self, key: &str, values: &[String]) -> Result<i64, CacheError>;

    /// RPUSH - Insert values at the tail of the list
    async fn rpush(&self, key: &str, values: &[String]) -> Result<i64, CacheError>;

    /// LPUSHX - Insert value at head only if list exists
    async fn lpush_x(&self, key: &str, values: &[String]) -> Result<i64, CacheError>;

    /// RPUSHX - Insert value at tail only if list exists
    async fn rpush_x(&self, key: &str, values: &[String]) -> Result<i64, CacheError>;

    /// LPOP - Remove and return elements from the head
    async fn lpop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError>;

    /// RPOP - Remove and return elements from the tail
    async fn rpop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError>;

    /// LRANGE - Get a range of elements from the list
    async fn lrange(&self, key: &str, start: i64, stop: i64) -> Result<Vec<String>, CacheError>;

    /// LLEN - Get the length of the list
    async fn llen(&self, key: &str) -> Result<i64, CacheError>;

    /// LINDEX - Get element at index
    async fn lindex(&self, key: &str, index: i64) -> Result<Option<String>, CacheError>;

    /// LSET - Set element at index
    async fn lset(&self, key: &str, index: i64, value: &str) -> Result<(), CacheError>;

    /// LINSERT - Insert element before or after pivot
    async fn linsert(
        &self,
        key: &str,
        position: InsertPosition,
        pivot: &str,
        value: &str,
    ) -> Result<i64, CacheError>;

    /// LREM - Remove elements equal to value
    async fn lrem(&self, key: &str, count: i64, value: &str) -> Result<i64, CacheError>;

    /// LTRIM - Trim list to specified range
    async fn ltrim(&self, key: &str, start: i64, stop: i64) -> Result<(), CacheError>;

    /// LPOS - Get index of element in list
    async fn lpos(
        &self,
        key: &str,
        element: &str,
        options: LPosOptions,
    ) -> Result<Vec<i64>, CacheError>;

    /// LMOVE - Move element from source to destination
    async fn lmove(
        &self,
        source: &str,
        destination: &str,
        src_dir: ListDirection,
        dst_dir: ListDirection,
    ) -> Result<Option<String>, CacheError>;

    /// RPOPLPUSH - Pop from source tail and push to destination head (deprecated, use LMOVE)
    async fn rpop_lpush(
        &self,
        source: &str,
        destination: &str,
    ) -> Result<Option<String>, CacheError>;

    // ========== Blocking operations ==========

    /// BLPOP - Blocking pop from head of list(s)
    /// Returns None if timeout is reached
    async fn blpop(
        &self,
        keys: &[String],
        timeout: Duration,
    ) -> Result<Option<BlockingPopResult>, CacheError>;

    /// BRPOP - Blocking pop from tail of list(s)
    /// Returns None if timeout is reached
    async fn brpop(
        &self,
        keys: &[String],
        timeout: Duration,
    ) -> Result<Option<BlockingPopResult>, CacheError>;

    /// BLMOVE - Blocking move from source to destination
    /// Returns None if timeout is reached
    async fn blmove(
        &self,
        source: &str,
        destination: &str,
        src_dir: ListDirection,
        dst_dir: ListDirection,
        timeout: Duration,
    ) -> Result<Option<String>, CacheError>;

    /// BRPOPLPUSH - Blocking pop from source tail and push to destination head (deprecated, use BLMOVE)
    /// Returns None if timeout is reached
    async fn brpop_lpush(
        &self,
        source: &str,
        destination: &str,
        timeout: Duration,
    ) -> Result<Option<String>, CacheError>;

    /// LMPOP - Atomically pop elements from the first non-empty list (Redis 7.0+)
    async fn lmpop(
        &self,
        keys: &[String],
        direction: ListDirection,
        count: Option<u32>,
    ) -> Result<Option<LMPopResult>, CacheError>;

    /// BLMPOP - Blocking pop from the first non-empty list (Redis 7.0+)
    /// Returns None if timeout is reached
    async fn blmpop(
        &self,
        keys: &[String],
        direction: ListDirection,
        timeout: Duration,
        count: Option<u32>,
    ) -> Result<Option<LMPopResult>, CacheError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_direction_as_str() {
        assert_eq!(ListDirection::Left.as_str(), "LEFT");
        assert_eq!(ListDirection::Right.as_str(), "RIGHT");
    }

    #[test]
    fn test_insert_position_as_str() {
        assert_eq!(InsertPosition::Before.as_str(), "BEFORE");
        assert_eq!(InsertPosition::After.as_str(), "AFTER");
    }
}
