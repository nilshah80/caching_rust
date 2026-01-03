//! Sorted Set Repository Trait
//!
//! Abstract interface for Redis sorted set (ZSET) operations.

use async_trait::async_trait;

use crate::domain::errors::CacheError;

/// A member with its score in a sorted set
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredMember {
    /// The member value
    pub member: String,
    /// The score associated with the member
    pub score: f64,
}

impl ScoredMember {
    /// Create a new scored member
    pub fn new(member: String, score: f64) -> Self {
        Self { member, score }
    }
}

/// Options for ZADD command
#[derive(Debug, Clone, Default)]
pub struct ZAddOptions {
    /// NX: Only add new elements (don't update existing)
    pub nx: bool,
    /// XX: Only update existing elements (don't add new)
    pub xx: bool,
    /// GT: Only update when new score > current score
    pub gt: bool,
    /// LT: Only update when new score < current score
    pub lt: bool,
    /// CH: Return number of changed elements (added + updated) instead of just added
    pub ch: bool,
}

/// Result from ZADD operation
#[derive(Debug, Clone)]
pub struct ZAddResult {
    /// Number of elements added (or changed if CH option was used)
    pub count: i64,
    /// If INCR option was used, the new score of the member
    pub new_score: Option<f64>,
}

/// Range specification for sorted set queries
#[derive(Debug, Clone)]
pub enum ZRangeType {
    /// Range by index (ZRANGE with BYSCORE=false)
    ByIndex,
    /// Range by score (ZRANGEBYSCORE equivalent)
    ByScore,
    /// Range by lexicographical order (ZRANGEBYLEX equivalent)
    ByLex,
}

/// Range bounds for score-based queries
#[derive(Debug, Clone)]
pub struct ScoreRange {
    /// Minimum score (use f64::NEG_INFINITY for -inf)
    pub min: f64,
    /// Maximum score (use f64::INFINITY for +inf)
    pub max: f64,
    /// Whether min is exclusive
    pub min_exclusive: bool,
    /// Whether max is exclusive
    pub max_exclusive: bool,
}

impl ScoreRange {
    /// Create an inclusive range
    pub fn inclusive(min: f64, max: f64) -> Self {
        Self {
            min,
            max,
            min_exclusive: false,
            max_exclusive: false,
        }
    }

    /// Create a range from -inf to +inf
    pub fn all() -> Self {
        Self::inclusive(f64::NEG_INFINITY, f64::INFINITY)
    }
}

/// Range bounds for lexicographical queries
#[derive(Debug, Clone)]
pub struct LexRange {
    /// Minimum value (use "-" for minimum, "[value" for inclusive, "(value" for exclusive)
    pub min: String,
    /// Maximum value (use "+" for maximum, "[value" for inclusive, "(value" for exclusive)
    pub max: String,
}

impl LexRange {
    /// Create a range from - to +
    pub fn all() -> Self {
        Self {
            min: "-".to_string(),
            max: "+".to_string(),
        }
    }

    /// Create an inclusive range
    pub fn inclusive(min: &str, max: &str) -> Self {
        Self {
            min: format!("[{}", min),
            max: format!("[{}", max),
        }
    }
}

/// Options for range queries
#[derive(Debug, Clone, Default)]
pub struct ZRangeOptions {
    /// Include scores in the result
    pub with_scores: bool,
    /// Reverse order
    pub rev: bool,
    /// Offset for LIMIT
    pub offset: Option<i64>,
    /// Count for LIMIT
    pub count: Option<i64>,
}

/// Result from ZSCAN operation
#[derive(Debug, Clone)]
pub struct ZScanResult {
    /// Cursor for next iteration (0 = iteration complete)
    pub cursor: u64,
    /// Members with scores returned in this batch
    pub members: Vec<ScoredMember>,
}

/// Result from blocking pop operations
#[derive(Debug, Clone)]
pub struct ZPopResult {
    /// The key from which the element was popped
    pub key: String,
    /// The popped members with their scores
    pub members: Vec<ScoredMember>,
}

/// Aggregate function for ZUNION/ZINTER operations
#[derive(Debug, Clone, Copy, Default)]
pub enum ZAggregate {
    #[default]
    Sum,
    Min,
    Max,
}

impl ZAggregate {
    pub fn as_str(&self) -> &'static str {
        match self {
            ZAggregate::Sum => "SUM",
            ZAggregate::Min => "MIN",
            ZAggregate::Max => "MAX",
        }
    }
}

/// Options for set algebra operations (ZUNION, ZINTER, ZDIFF)
#[derive(Debug, Clone, Default)]
pub struct ZSetAlgebraOptions {
    /// Weights for each key
    pub weights: Option<Vec<f64>>,
    /// Aggregate function
    pub aggregate: ZAggregate,
    /// Include scores in result
    pub with_scores: bool,
}

/// Direction for ZMPOP
#[derive(Debug, Clone, Copy)]
pub enum ZPopDirection {
    Min,
    Max,
}

impl ZPopDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            ZPopDirection::Min => "MIN",
            ZPopDirection::Max => "MAX",
        }
    }
}

/// Repository trait for Redis sorted set operations
#[async_trait]
pub trait SortedSetRepository: Send + Sync {
    // ========== Basic operations ==========

    /// ZADD - Add members with scores to a sorted set
    async fn zadd(
        &self,
        key: &str,
        members: &[ScoredMember],
        options: Option<ZAddOptions>,
    ) -> Result<ZAddResult, CacheError>;

    /// ZADD with INCR option - Increment the score of a member
    async fn zadd_incr(
        &self,
        key: &str,
        member: &str,
        score: f64,
        options: Option<ZAddOptions>,
    ) -> Result<Option<f64>, CacheError>;

    /// ZREM - Remove members from a sorted set
    async fn zrem(&self, key: &str, members: &[String]) -> Result<i64, CacheError>;

    /// ZSCORE - Get the score of a member
    async fn zscore(&self, key: &str, member: &str) -> Result<Option<f64>, CacheError>;

    /// ZMSCORE - Get scores of multiple members
    async fn zmscore(&self, key: &str, members: &[String]) -> Result<Vec<Option<f64>>, CacheError>;

    /// ZINCRBY - Increment the score of a member
    async fn zincrby(&self, key: &str, member: &str, increment: f64) -> Result<f64, CacheError>;

    /// ZCARD - Get the number of members in a sorted set
    async fn zcard(&self, key: &str) -> Result<i64, CacheError>;

    /// ZCOUNT - Count members with scores in a range
    async fn zcount(&self, key: &str, range: &ScoreRange) -> Result<i64, CacheError>;

    /// ZLEXCOUNT - Count members in a lexicographical range (all members must have same score)
    async fn zlexcount(&self, key: &str, range: &LexRange) -> Result<i64, CacheError>;

    // ========== Rank operations ==========

    /// ZRANK - Get the rank of a member (0-based, lowest score first)
    async fn zrank(&self, key: &str, member: &str) -> Result<Option<i64>, CacheError>;

    /// ZREVRANK - Get the reverse rank of a member (0-based, highest score first)
    async fn zrevrank(&self, key: &str, member: &str) -> Result<Option<i64>, CacheError>;

    // ========== Range operations ==========

    /// ZRANGE - Get members in a range by index
    async fn zrange(
        &self,
        key: &str,
        start: i64,
        stop: i64,
        options: Option<ZRangeOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError>;

    /// ZRANGEBYSCORE - Get members with scores in a range
    async fn zrangebyscore(
        &self,
        key: &str,
        range: &ScoreRange,
        options: Option<ZRangeOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError>;

    /// ZRANGEBYLEX - Get members in a lexicographical range
    async fn zrangebylex(
        &self,
        key: &str,
        range: &LexRange,
        options: Option<ZRangeOptions>,
    ) -> Result<Vec<String>, CacheError>;

    /// ZRANGESTORE - Store a range in a new key
    async fn zrangestore(
        &self,
        destination: &str,
        source: &str,
        start: i64,
        stop: i64,
        options: Option<ZRangeOptions>,
    ) -> Result<i64, CacheError>;

    // ========== Remove range operations ==========

    /// ZREMRANGEBYRANK - Remove members by rank range
    async fn zremrangebyrank(&self, key: &str, start: i64, stop: i64) -> Result<i64, CacheError>;

    /// ZREMRANGEBYSCORE - Remove members by score range
    async fn zremrangebyscore(&self, key: &str, range: &ScoreRange) -> Result<i64, CacheError>;

    /// ZREMRANGEBYLEX - Remove members by lexicographical range
    async fn zremrangebylex(&self, key: &str, range: &LexRange) -> Result<i64, CacheError>;

    // ========== Pop operations ==========

    /// ZPOPMIN - Remove and return members with lowest scores
    async fn zpopmin(&self, key: &str, count: Option<i64>) -> Result<Vec<ScoredMember>, CacheError>;

    /// ZPOPMAX - Remove and return members with highest scores
    async fn zpopmax(&self, key: &str, count: Option<i64>) -> Result<Vec<ScoredMember>, CacheError>;

    /// BZPOPMIN - Blocking pop of member with lowest score
    async fn bzpopmin(
        &self,
        keys: &[String],
        timeout: f64,
    ) -> Result<Option<ZPopResult>, CacheError>;

    /// BZPOPMAX - Blocking pop of member with highest score
    async fn bzpopmax(
        &self,
        keys: &[String],
        timeout: f64,
    ) -> Result<Option<ZPopResult>, CacheError>;

    /// ZMPOP - Pop members from multiple keys
    async fn zmpop(
        &self,
        keys: &[String],
        direction: ZPopDirection,
        count: Option<i64>,
    ) -> Result<Option<ZPopResult>, CacheError>;

    /// BZMPOP - Blocking pop from multiple keys
    async fn bzmpop(
        &self,
        keys: &[String],
        direction: ZPopDirection,
        timeout: f64,
        count: Option<i64>,
    ) -> Result<Option<ZPopResult>, CacheError>;

    // ========== Random access ==========

    /// ZRANDMEMBER - Get random members
    async fn zrandmember(
        &self,
        key: &str,
        count: Option<i64>,
        with_scores: bool,
    ) -> Result<Vec<ScoredMember>, CacheError>;

    // ========== Set algebra operations ==========

    /// ZUNION - Get the union of multiple sorted sets
    async fn zunion(
        &self,
        keys: &[String],
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError>;

    /// ZUNIONSTORE - Store the union of multiple sorted sets
    async fn zunionstore(
        &self,
        destination: &str,
        keys: &[String],
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<i64, CacheError>;

    /// ZINTER - Get the intersection of multiple sorted sets
    async fn zinter(
        &self,
        keys: &[String],
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError>;

    /// ZINTERSTORE - Store the intersection of multiple sorted sets
    async fn zinterstore(
        &self,
        destination: &str,
        keys: &[String],
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<i64, CacheError>;

    /// ZINTERCARD - Get the cardinality of the intersection
    async fn zintercard(&self, keys: &[String], limit: Option<u64>) -> Result<i64, CacheError>;

    /// ZDIFF - Get the difference of sorted sets
    async fn zdiff(
        &self,
        keys: &[String],
        with_scores: bool,
    ) -> Result<Vec<ScoredMember>, CacheError>;

    /// ZDIFFSTORE - Store the difference of sorted sets
    async fn zdiffstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError>;

    // ========== Scan operation ==========

    /// ZSCAN - Incrementally iterate sorted set members
    async fn zscan(
        &self,
        key: &str,
        cursor: u64,
        pattern: Option<&str>,
        count: Option<u64>,
    ) -> Result<ZScanResult, CacheError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scored_member() {
        let member = ScoredMember::new("test".to_string(), 1.5);
        assert_eq!(member.member, "test");
        assert_eq!(member.score, 1.5);
    }

    #[test]
    fn test_zadd_options_default() {
        let options = ZAddOptions::default();
        assert!(!options.nx);
        assert!(!options.xx);
        assert!(!options.gt);
        assert!(!options.lt);
        assert!(!options.ch);
    }

    #[test]
    fn test_score_range() {
        let range = ScoreRange::inclusive(0.0, 100.0);
        assert_eq!(range.min, 0.0);
        assert_eq!(range.max, 100.0);
        assert!(!range.min_exclusive);
        assert!(!range.max_exclusive);

        let all = ScoreRange::all();
        assert_eq!(all.min, f64::NEG_INFINITY);
        assert_eq!(all.max, f64::INFINITY);
    }

    #[test]
    fn test_lex_range() {
        let all = LexRange::all();
        assert_eq!(all.min, "-");
        assert_eq!(all.max, "+");

        let inclusive = LexRange::inclusive("a", "z");
        assert_eq!(inclusive.min, "[a");
        assert_eq!(inclusive.max, "[z");
    }

    #[test]
    fn test_zscan_result() {
        let result = ZScanResult {
            cursor: 42,
            members: vec![
                ScoredMember::new("a".to_string(), 1.0),
                ScoredMember::new("b".to_string(), 2.0),
            ],
        };
        assert_eq!(result.cursor, 42);
        assert_eq!(result.members.len(), 2);
    }

    #[test]
    fn test_aggregate_as_str() {
        assert_eq!(ZAggregate::Sum.as_str(), "SUM");
        assert_eq!(ZAggregate::Min.as_str(), "MIN");
        assert_eq!(ZAggregate::Max.as_str(), "MAX");
    }

    #[test]
    fn test_pop_direction_as_str() {
        assert_eq!(ZPopDirection::Min.as_str(), "MIN");
        assert_eq!(ZPopDirection::Max.as_str(), "MAX");
    }
}
