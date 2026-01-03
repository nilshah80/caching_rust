//! Set Repository Trait
//!
//! Abstract interface for Redis set operations.

use async_trait::async_trait;

use crate::domain::errors::CacheError;

/// Result from SSCAN operation
#[derive(Debug, Clone)]
pub struct SetScanResult {
    /// Cursor for next iteration (0 = iteration complete)
    pub cursor: u64,
    /// Members returned in this batch
    pub members: Vec<String>,
}

/// Repository trait for Redis set operations
#[async_trait]
pub trait SetRepository: Send + Sync {
    // ========== Basic operations ==========

    /// SADD - Add members to a set
    /// Returns the number of members that were added (not including already existing members)
    async fn sadd(&self, key: &str, members: &[String]) -> Result<i64, CacheError>;

    /// SREM - Remove members from a set
    /// Returns the number of members that were removed
    async fn srem(&self, key: &str, members: &[String]) -> Result<i64, CacheError>;

    /// SMEMBERS - Get all members of a set
    async fn smembers(&self, key: &str) -> Result<Vec<String>, CacheError>;

    /// SISMEMBER - Check if a member exists in a set
    async fn sismember(&self, key: &str, member: &str) -> Result<bool, CacheError>;

    /// SMISMEMBER - Check if multiple members exist in a set
    /// Returns a vector of booleans in the same order as the input members
    async fn smismember(&self, key: &str, members: &[String]) -> Result<Vec<bool>, CacheError>;

    /// SCARD - Get the number of members in a set
    async fn scard(&self, key: &str) -> Result<i64, CacheError>;

    // ========== Random access operations ==========

    /// SRANDMEMBER - Get random members from a set without removing them
    /// If count is positive, returns up to count distinct members
    /// If count is negative, returns abs(count) members (may include duplicates)
    async fn srandmember(&self, key: &str, count: Option<i64>) -> Result<Vec<String>, CacheError>;

    /// SPOP - Remove and return random members from a set
    async fn spop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError>;

    /// SMOVE - Move a member from one set to another
    /// Returns true if the member was moved, false if it didn't exist in source
    async fn smove(&self, source: &str, destination: &str, member: &str) -> Result<bool, CacheError>;

    // ========== Set algebra operations ==========

    /// SINTER - Get the intersection of multiple sets
    async fn sinter(&self, keys: &[String]) -> Result<Vec<String>, CacheError>;

    /// SINTERSTORE - Store the intersection of multiple sets in a destination key
    /// Returns the number of members in the resulting set
    async fn sinterstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError>;

    /// SINTERCARD - Get the cardinality of the intersection (without storing)
    /// limit: If > 0, stop early when reaching this count
    async fn sintercard(&self, keys: &[String], limit: Option<u64>) -> Result<i64, CacheError>;

    /// SUNION - Get the union of multiple sets
    async fn sunion(&self, keys: &[String]) -> Result<Vec<String>, CacheError>;

    /// SUNIONSTORE - Store the union of multiple sets in a destination key
    /// Returns the number of members in the resulting set
    async fn sunionstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError>;

    /// SDIFF - Get the difference of sets (members in first set but not in others)
    async fn sdiff(&self, keys: &[String]) -> Result<Vec<String>, CacheError>;

    /// SDIFFSTORE - Store the difference of sets in a destination key
    /// Returns the number of members in the resulting set
    async fn sdiffstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError>;

    // ========== Scan operation ==========

    /// SSCAN - Incrementally iterate set members
    async fn sscan(
        &self,
        key: &str,
        cursor: u64,
        pattern: Option<&str>,
        count: Option<u64>,
    ) -> Result<SetScanResult, CacheError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_scan_result() {
        let result = SetScanResult {
            cursor: 42,
            members: vec!["a".to_string(), "b".to_string()],
        };
        assert_eq!(result.cursor, 42);
        assert_eq!(result.members.len(), 2);
    }
}
