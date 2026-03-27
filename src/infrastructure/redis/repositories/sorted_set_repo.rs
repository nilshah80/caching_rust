//! Redis Sorted Set Repository Implementation
//!
//! Concrete implementation of SortedSetRepository using Redis.

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    LexRange, ScoreRange, ScoredMember, SortedSetRepository, ZAddOptions, ZAddResult,
    ZPopDirection, ZPopResult, ZRangeOptions, ZScanResult, ZSetAlgebraOptions,
};
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Redis implementation of SortedSetRepository
#[derive(Clone)]
pub struct RedisSortedSetRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisSortedSetRepository {
    /// Create a new RedisSortedSetRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }

    /// Parse a flat vector of strings into ScoredMembers (alternating member, score)
    fn parse_members_with_scores(data: Vec<String>) -> Vec<ScoredMember> {
        data.chunks(2)
            .filter_map(|chunk| {
                if chunk.len() == 2 {
                    let member = chunk[0].clone();
                    let score = chunk[1].parse::<f64>().unwrap_or(0.0);
                    Some(ScoredMember::new(member, score))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Format score for Redis command (handles infinity)
    fn format_score(score: f64, exclusive: bool) -> String {
        if score == f64::NEG_INFINITY {
            "-inf".to_string()
        } else if score == f64::INFINITY {
            "+inf".to_string()
        } else if exclusive {
            format!("({}", score)
        } else {
            score.to_string()
        }
    }
}

#[async_trait]
impl SortedSetRepository for RedisSortedSetRepository {
    async fn zadd(
        &self,
        key: &str,
        members: &[ScoredMember],
        options: Option<ZAddOptions>,
    ) -> Result<ZAddResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZADD");
        cmd.arg(key);

        if let Some(opts) = &options {
            if opts.nx {
                cmd.arg("NX");
            }
            if opts.xx {
                cmd.arg("XX");
            }
            if opts.gt {
                cmd.arg("GT");
            }
            if opts.lt {
                cmd.arg("LT");
            }
            if opts.ch {
                cmd.arg("CH");
            }
        }

        for member in members {
            cmd.arg(member.score).arg(&member.member);
        }

        let count: i64 = cmd.query_async(&mut *conn).await?;
        Ok(ZAddResult {
            count,
            new_score: None,
        })
    }

    async fn zadd_incr(
        &self,
        key: &str,
        member: &str,
        score: f64,
        options: Option<ZAddOptions>,
    ) -> Result<Option<f64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZADD");
        cmd.arg(key);

        if let Some(opts) = &options {
            if opts.nx {
                cmd.arg("NX");
            }
            if opts.xx {
                cmd.arg("XX");
            }
            if opts.gt {
                cmd.arg("GT");
            }
            if opts.lt {
                cmd.arg("LT");
            }
        }

        cmd.arg("INCR").arg(score).arg(member);

        let result: Option<f64> = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn zrem(&self, key: &str, members: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZREM");
        cmd.arg(key);
        for member in members {
            cmd.arg(member);
        }
        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn zscore(&self, key: &str, member: &str) -> Result<Option<f64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Option<f64> = redis::cmd("ZSCORE")
            .arg(key)
            .arg(member)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn zmscore(&self, key: &str, members: &[String]) -> Result<Vec<Option<f64>>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZMSCORE");
        cmd.arg(key);
        for member in members {
            cmd.arg(member);
        }
        let result: Vec<Option<f64>> = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn zincrby(&self, key: &str, member: &str, increment: f64) -> Result<f64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: f64 = redis::cmd("ZINCRBY")
            .arg(key)
            .arg(increment)
            .arg(member)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn zcard(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("ZCARD").arg(key).query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn zcount(&self, key: &str, range: &ScoreRange) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let min = Self::format_score(range.min, range.min_exclusive);
        let max = Self::format_score(range.max, range.max_exclusive);

        let result: i64 = redis::cmd("ZCOUNT")
            .arg(key)
            .arg(&min)
            .arg(&max)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn zlexcount(&self, key: &str, range: &LexRange) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("ZLEXCOUNT")
            .arg(key)
            .arg(&range.min)
            .arg(&range.max)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn zrank(&self, key: &str, member: &str) -> Result<Option<i64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Option<i64> = redis::cmd("ZRANK")
            .arg(key)
            .arg(member)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn zrevrank(&self, key: &str, member: &str) -> Result<Option<i64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Option<i64> = redis::cmd("ZREVRANK")
            .arg(key)
            .arg(member)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn zrange(
        &self,
        key: &str,
        start: i64,
        stop: i64,
        options: Option<ZRangeOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let mut conn = self.pool.get().await?;
        let opts = options.unwrap_or_default();

        let mut cmd = redis::cmd("ZRANGE");
        cmd.arg(key).arg(start).arg(stop);

        if opts.rev {
            cmd.arg("REV");
        }

        if opts.with_scores {
            cmd.arg("WITHSCORES");
            let result: Vec<String> = cmd.query_async(&mut *conn).await?;
            Ok(Self::parse_members_with_scores(result))
        } else {
            let members: Vec<String> = cmd.query_async(&mut *conn).await?;
            Ok(members
                .into_iter()
                .map(|m| ScoredMember::new(m, 0.0))
                .collect())
        }
    }

    async fn zrangebyscore(
        &self,
        key: &str,
        range: &ScoreRange,
        options: Option<ZRangeOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let mut conn = self.pool.get().await?;
        let opts = options.unwrap_or_default();
        let min = Self::format_score(range.min, range.min_exclusive);
        let max = Self::format_score(range.max, range.max_exclusive);

        // Use unified ZRANGE command with BYSCORE
        let mut cmd = redis::cmd("ZRANGE");
        cmd.arg(key);

        if opts.rev {
            // For reverse, swap min/max
            cmd.arg(&max).arg(&min);
        } else {
            cmd.arg(&min).arg(&max);
        }

        cmd.arg("BYSCORE");

        if opts.rev {
            cmd.arg("REV");
        }

        if let (Some(offset), Some(count)) = (opts.offset, opts.count) {
            cmd.arg("LIMIT").arg(offset).arg(count);
        }

        if opts.with_scores {
            cmd.arg("WITHSCORES");
            let result: Vec<String> = cmd.query_async(&mut *conn).await?;
            Ok(Self::parse_members_with_scores(result))
        } else {
            let members: Vec<String> = cmd.query_async(&mut *conn).await?;
            Ok(members
                .into_iter()
                .map(|m| ScoredMember::new(m, 0.0))
                .collect())
        }
    }

    async fn zrangebylex(
        &self,
        key: &str,
        range: &LexRange,
        options: Option<ZRangeOptions>,
    ) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let opts = options.unwrap_or_default();

        // Use unified ZRANGE command with BYLEX
        let mut cmd = redis::cmd("ZRANGE");
        cmd.arg(key);

        if opts.rev {
            cmd.arg(&range.max).arg(&range.min);
        } else {
            cmd.arg(&range.min).arg(&range.max);
        }

        cmd.arg("BYLEX");

        if opts.rev {
            cmd.arg("REV");
        }

        if let (Some(offset), Some(count)) = (opts.offset, opts.count) {
            cmd.arg("LIMIT").arg(offset).arg(count);
        }

        let result: Vec<String> = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn zrangestore(
        &self,
        destination: &str,
        source: &str,
        start: i64,
        stop: i64,
        options: Option<ZRangeOptions>,
    ) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let opts = options.unwrap_or_default();

        let mut cmd = redis::cmd("ZRANGESTORE");
        cmd.arg(destination).arg(source).arg(start).arg(stop);

        if opts.rev {
            cmd.arg("REV");
        }

        if let (Some(offset), Some(count)) = (opts.offset, opts.count) {
            cmd.arg("LIMIT").arg(offset).arg(count);
        }

        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn zremrangebyrank(&self, key: &str, start: i64, stop: i64) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("ZREMRANGEBYRANK")
            .arg(key)
            .arg(start)
            .arg(stop)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn zremrangebyscore(&self, key: &str, range: &ScoreRange) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let min = Self::format_score(range.min, range.min_exclusive);
        let max = Self::format_score(range.max, range.max_exclusive);

        let result: i64 = redis::cmd("ZREMRANGEBYSCORE")
            .arg(key)
            .arg(&min)
            .arg(&max)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn zremrangebylex(&self, key: &str, range: &LexRange) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("ZREMRANGEBYLEX")
            .arg(key)
            .arg(&range.min)
            .arg(&range.max)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn zpopmin(
        &self,
        key: &str,
        count: Option<i64>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZPOPMIN");
        cmd.arg(key);

        if let Some(c) = count {
            cmd.arg(c);
        }

        let result: Vec<String> = cmd.query_async(&mut *conn).await?;
        Ok(Self::parse_members_with_scores(result))
    }

    async fn zpopmax(
        &self,
        key: &str,
        count: Option<i64>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZPOPMAX");
        cmd.arg(key);

        if let Some(c) = count {
            cmd.arg(c);
        }

        let result: Vec<String> = cmd.query_async(&mut *conn).await?;
        Ok(Self::parse_members_with_scores(result))
    }

    async fn bzpopmin(
        &self,
        keys: &[String],
        timeout: f64,
    ) -> Result<Option<ZPopResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("BZPOPMIN");
        for key in keys {
            cmd.arg(key);
        }
        cmd.arg(timeout);

        let result: Option<(String, String, f64)> = cmd.query_async(&mut *conn).await?;

        Ok(result.map(|(key, member, score)| ZPopResult {
            key,
            members: vec![ScoredMember::new(member, score)],
        }))
    }

    async fn bzpopmax(
        &self,
        keys: &[String],
        timeout: f64,
    ) -> Result<Option<ZPopResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("BZPOPMAX");
        for key in keys {
            cmd.arg(key);
        }
        cmd.arg(timeout);

        let result: Option<(String, String, f64)> = cmd.query_async(&mut *conn).await?;

        Ok(result.map(|(key, member, score)| ZPopResult {
            key,
            members: vec![ScoredMember::new(member, score)],
        }))
    }

    async fn zmpop(
        &self,
        keys: &[String],
        direction: ZPopDirection,
        count: Option<i64>,
    ) -> Result<Option<ZPopResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZMPOP");
        cmd.arg(keys.len());
        for key in keys {
            cmd.arg(key);
        }
        cmd.arg(direction.as_str());

        if let Some(c) = count {
            cmd.arg("COUNT").arg(c);
        }

        // ZMPOP returns: nil or [key, [[member, score], ...]]
        let result: Option<(String, Vec<(String, f64)>)> = cmd.query_async(&mut *conn).await?;

        Ok(result.map(|(key, items)| ZPopResult {
            key,
            members: items
                .into_iter()
                .map(|(m, s)| ScoredMember::new(m, s))
                .collect(),
        }))
    }

    async fn bzmpop(
        &self,
        keys: &[String],
        direction: ZPopDirection,
        timeout: f64,
        count: Option<i64>,
    ) -> Result<Option<ZPopResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("BZMPOP");
        cmd.arg(timeout).arg(keys.len());
        for key in keys {
            cmd.arg(key);
        }
        cmd.arg(direction.as_str());

        if let Some(c) = count {
            cmd.arg("COUNT").arg(c);
        }

        let result: Option<(String, Vec<(String, f64)>)> = cmd.query_async(&mut *conn).await?;

        Ok(result.map(|(key, items)| ZPopResult {
            key,
            members: items
                .into_iter()
                .map(|(m, s)| ScoredMember::new(m, s))
                .collect(),
        }))
    }

    async fn zrandmember(
        &self,
        key: &str,
        count: Option<i64>,
        with_scores: bool,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZRANDMEMBER");
        cmd.arg(key);

        if let Some(c) = count {
            cmd.arg(c);
            if with_scores {
                cmd.arg("WITHSCORES");
                let result: Vec<String> = cmd.query_async(&mut *conn).await?;
                Ok(Self::parse_members_with_scores(result))
            } else {
                let members: Vec<String> = cmd.query_async(&mut *conn).await?;
                Ok(members
                    .into_iter()
                    .map(|m| ScoredMember::new(m, 0.0))
                    .collect())
            }
        } else {
            // Without count, returns single member
            let member: Option<String> = cmd.query_async(&mut *conn).await?;
            Ok(member
                .into_iter()
                .map(|m| ScoredMember::new(m, 0.0))
                .collect())
        }
    }

    async fn zunion(
        &self,
        keys: &[String],
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let mut conn = self.pool.get().await?;
        let opts = options.unwrap_or_default();

        let mut cmd = redis::cmd("ZUNION");
        cmd.arg(keys.len());
        for key in keys {
            cmd.arg(key);
        }

        if let Some(weights) = &opts.weights {
            cmd.arg("WEIGHTS");
            for w in weights {
                cmd.arg(*w);
            }
        }

        cmd.arg("AGGREGATE").arg(opts.aggregate.as_str());

        if opts.with_scores {
            cmd.arg("WITHSCORES");
            let result: Vec<String> = cmd.query_async(&mut *conn).await?;
            Ok(Self::parse_members_with_scores(result))
        } else {
            let members: Vec<String> = cmd.query_async(&mut *conn).await?;
            Ok(members
                .into_iter()
                .map(|m| ScoredMember::new(m, 0.0))
                .collect())
        }
    }

    async fn zunionstore(
        &self,
        destination: &str,
        keys: &[String],
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let opts = options.unwrap_or_default();

        let mut cmd = redis::cmd("ZUNIONSTORE");
        cmd.arg(destination).arg(keys.len());
        for key in keys {
            cmd.arg(key);
        }

        if let Some(weights) = &opts.weights {
            cmd.arg("WEIGHTS");
            for w in weights {
                cmd.arg(*w);
            }
        }

        cmd.arg("AGGREGATE").arg(opts.aggregate.as_str());

        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn zinter(
        &self,
        keys: &[String],
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let mut conn = self.pool.get().await?;
        let opts = options.unwrap_or_default();

        let mut cmd = redis::cmd("ZINTER");
        cmd.arg(keys.len());
        for key in keys {
            cmd.arg(key);
        }

        if let Some(weights) = &opts.weights {
            cmd.arg("WEIGHTS");
            for w in weights {
                cmd.arg(*w);
            }
        }

        cmd.arg("AGGREGATE").arg(opts.aggregate.as_str());

        if opts.with_scores {
            cmd.arg("WITHSCORES");
            let result: Vec<String> = cmd.query_async(&mut *conn).await?;
            Ok(Self::parse_members_with_scores(result))
        } else {
            let members: Vec<String> = cmd.query_async(&mut *conn).await?;
            Ok(members
                .into_iter()
                .map(|m| ScoredMember::new(m, 0.0))
                .collect())
        }
    }

    async fn zinterstore(
        &self,
        destination: &str,
        keys: &[String],
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let opts = options.unwrap_or_default();

        let mut cmd = redis::cmd("ZINTERSTORE");
        cmd.arg(destination).arg(keys.len());
        for key in keys {
            cmd.arg(key);
        }

        if let Some(weights) = &opts.weights {
            cmd.arg("WEIGHTS");
            for w in weights {
                cmd.arg(*w);
            }
        }

        cmd.arg("AGGREGATE").arg(opts.aggregate.as_str());

        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn zintercard(&self, keys: &[String], limit: Option<u64>) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZINTERCARD");
        cmd.arg(keys.len());
        for key in keys {
            cmd.arg(key);
        }

        if let Some(l) = limit {
            cmd.arg("LIMIT").arg(l);
        }

        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn zdiff(
        &self,
        keys: &[String],
        with_scores: bool,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZDIFF");
        cmd.arg(keys.len());
        for key in keys {
            cmd.arg(key);
        }

        if with_scores {
            cmd.arg("WITHSCORES");
            let result: Vec<String> = cmd.query_async(&mut *conn).await?;
            Ok(Self::parse_members_with_scores(result))
        } else {
            let members: Vec<String> = cmd.query_async(&mut *conn).await?;
            Ok(members
                .into_iter()
                .map(|m| ScoredMember::new(m, 0.0))
                .collect())
        }
    }

    async fn zdiffstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZDIFFSTORE");
        cmd.arg(destination).arg(keys.len());
        for key in keys {
            cmd.arg(key);
        }

        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn zscan(
        &self,
        key: &str,
        cursor: u64,
        pattern: Option<&str>,
        count: Option<u64>,
    ) -> Result<ZScanResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("ZSCAN");
        cmd.arg(key).arg(cursor);

        if let Some(p) = pattern {
            cmd.arg("MATCH").arg(p);
        }
        if let Some(c) = count {
            cmd.arg("COUNT").arg(c);
        }

        let (next_cursor, data): (u64, Vec<String>) = cmd.query_async(&mut *conn).await?;
        let members = Self::parse_members_with_scores(data);

        Ok(ZScanResult {
            cursor: next_cursor,
            members,
        })
    }
}
