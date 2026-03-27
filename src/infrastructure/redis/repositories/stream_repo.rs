//! Redis Stream Repository Implementation
//!
//! Concrete implementation of StreamRepository using Redis.
//! Follows Architecture Decision 3: Blocking commands enforce max 30s timeout.

use async_trait::async_trait;
use redis::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::domain::entities::{
    AutoClaimResult, ClaimResult, ConsumerGroupInfo, ConsumerInfo, PendingEntry, PendingSummary,
    StreamEntry, StreamInfo, StreamReadResult, XAddOptions, XAutoClaimOptions, XClaimOptions,
    XGroupCreateOptions, XPendingOptions, XReadGroupOptions, XReadOptions, XTrimStrategy,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::StreamRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::shared::blocking::MAX_BLOCKING_TIMEOUT_SECS;

/// Type alias for Redis stream read results: Vec of (stream_key, Vec of (entry_id, Vec of (field, value)))
type RedisStreamReadResult = Vec<(String, Vec<(String, Vec<(String, String)>)>)>;

/// Redis implementation of StreamRepository
#[derive(Clone)]
pub struct RedisStreamRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisStreamRepository {
    /// Create a new RedisStreamRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }

    /// Parse a stream entry from Redis response
    fn parse_entry(id: String, fields_raw: Vec<(String, String)>) -> StreamEntry {
        let mut fields = HashMap::new();
        for (k, v) in fields_raw {
            fields.insert(k, v);
        }
        StreamEntry { id, fields }
    }

    /// Parse stream read results from Redis response
    fn parse_read_results(results: RedisStreamReadResult) -> Vec<StreamReadResult> {
        results
            .into_iter()
            .map(|(key, entries)| {
                let parsed_entries = entries
                    .into_iter()
                    .map(|(id, fields)| Self::parse_entry(id, fields))
                    .collect();
                StreamReadResult {
                    key,
                    entries: parsed_entries,
                }
            })
            .collect()
    }

    /// Enforce blocking timeout bounds (min 1s, max 30s)
    fn enforce_max_timeout(timeout: Duration) -> Duration {
        timeout.clamp(
            Duration::from_secs(1),
            Duration::from_secs(MAX_BLOCKING_TIMEOUT_SECS),
        )
    }
}

#[async_trait]
impl StreamRepository for RedisStreamRepository {
    // ========== Basic Stream Operations ==========

    async fn xadd(
        &self,
        key: &str,
        fields: &HashMap<String, String>,
        options: XAddOptions,
    ) -> Result<String, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XADD");
        cmd.arg(key);

        // NOMKSTREAM option
        if options.no_mkstream {
            cmd.arg("NOMKSTREAM");
        }

        // Trimming options
        if let Some(maxlen) = options.maxlen {
            cmd.arg("MAXLEN");
            if options.approximate {
                cmd.arg("~");
            }
            cmd.arg(maxlen);
            if let Some(limit) = options.limit {
                cmd.arg("LIMIT").arg(limit);
            }
        } else if let Some(minid) = &options.minid {
            cmd.arg("MINID");
            if options.approximate {
                cmd.arg("~");
            }
            cmd.arg(minid);
            if let Some(limit) = options.limit {
                cmd.arg("LIMIT").arg(limit);
            }
        }

        // Entry ID (or * for auto-generate)
        if let Some(id) = &options.id {
            cmd.arg(id);
        } else {
            cmd.arg("*");
        }

        // Fields
        for (field, value) in fields {
            cmd.arg(field).arg(value);
        }

        let result: String = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn xlen(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("XLEN").arg(key).query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn xrange(
        &self,
        key: &str,
        start: &str,
        end: &str,
        count: Option<i64>,
    ) -> Result<Vec<StreamEntry>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XRANGE");
        cmd.arg(key).arg(start).arg(end);

        if let Some(c) = count {
            cmd.arg("COUNT").arg(c);
        }

        let result: Vec<(String, Vec<(String, String)>)> = cmd.query_async(&mut *conn).await?;
        Ok(result
            .into_iter()
            .map(|(id, fields)| Self::parse_entry(id, fields))
            .collect())
    }

    async fn xrevrange(
        &self,
        key: &str,
        end: &str,
        start: &str,
        count: Option<i64>,
    ) -> Result<Vec<StreamEntry>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XREVRANGE");
        cmd.arg(key).arg(end).arg(start);

        if let Some(c) = count {
            cmd.arg("COUNT").arg(c);
        }

        let result: Vec<(String, Vec<(String, String)>)> = cmd.query_async(&mut *conn).await?;
        Ok(result
            .into_iter()
            .map(|(id, fields)| Self::parse_entry(id, fields))
            .collect())
    }

    async fn xdel(&self, key: &str, ids: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XDEL");
        cmd.arg(key);
        for id in ids {
            cmd.arg(id);
        }
        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn xtrim(&self, key: &str, strategy: XTrimStrategy) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XTRIM");
        cmd.arg(key);

        match strategy {
            XTrimStrategy::MaxLen { count, approximate } => {
                cmd.arg("MAXLEN");
                if approximate {
                    cmd.arg("~");
                }
                cmd.arg(count);
            }
            XTrimStrategy::MinId { id, approximate } => {
                cmd.arg("MINID");
                if approximate {
                    cmd.arg("~");
                }
                cmd.arg(&id);
            }
        }

        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn xinfo_stream(&self, key: &str, full: bool) -> Result<StreamInfo, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XINFO");
        cmd.arg("STREAM").arg(key);

        if full {
            cmd.arg("FULL");
        }

        let result: Value = cmd.query_async(&mut *conn).await?;

        // Parse the XINFO STREAM response
        let info = Self::parse_xinfo_stream(result)?;
        Ok(info)
    }

    // ========== Read Operations ==========

    async fn xread(
        &self,
        streams: &[(String, String)],
        options: XReadOptions,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XREAD");

        if let Some(count) = options.count {
            cmd.arg("COUNT").arg(count);
        }

        if let Some(block_ms) = options.block_ms {
            // Enforce max timeout (Architecture Decision 3)
            let max_ms = (MAX_BLOCKING_TIMEOUT_SECS * 1000) as i64;
            let actual_ms = block_ms.min(max_ms);
            cmd.arg("BLOCK").arg(actual_ms);
        }

        cmd.arg("STREAMS");
        for (key, _) in streams {
            cmd.arg(key);
        }
        for (_, id) in streams {
            cmd.arg(id);
        }

        let result: Option<Vec<(String, Vec<(String, Vec<(String, String)>)>)>> =
            cmd.query_async(&mut *conn).await?;

        Ok(result.map(Self::parse_read_results))
    }

    async fn xread_blocking(
        &self,
        streams: &[(String, String)],
        count: Option<i64>,
        timeout: Duration,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
        // Enforce maximum timeout (Architecture Decision 3)
        let timeout = Self::enforce_max_timeout(timeout);

        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XREAD");

        if let Some(c) = count {
            cmd.arg("COUNT").arg(c);
        }

        cmd.arg("BLOCK").arg(timeout.as_millis() as i64);
        cmd.arg("STREAMS");

        for (key, _) in streams {
            cmd.arg(key);
        }
        for (_, id) in streams {
            cmd.arg(id);
        }

        let result: Option<Vec<(String, Vec<(String, Vec<(String, String)>)>)>> =
            cmd.query_async(&mut *conn).await?;

        Ok(result.map(Self::parse_read_results))
    }

    // ========== Consumer Group Operations ==========

    async fn xgroup_create(
        &self,
        key: &str,
        group: &str,
        id: &str,
        options: XGroupCreateOptions,
    ) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XGROUP");
        cmd.arg("CREATE").arg(key).arg(group).arg(id);

        if options.mkstream {
            cmd.arg("MKSTREAM");
        }

        if let Some(entries_read) = options.entries_read {
            cmd.arg("ENTRIESREAD").arg(entries_read);
        }

        let _: () = cmd.query_async(&mut *conn).await?;
        Ok(())
    }

    async fn xgroup_destroy(&self, key: &str, group: &str) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("XGROUP")
            .arg("DESTROY")
            .arg(key)
            .arg(group)
            .query_async(&mut *conn)
            .await?;
        Ok(result == 1)
    }

    async fn xgroup_setid(
        &self,
        key: &str,
        group: &str,
        id: &str,
        entries_read: Option<i64>,
    ) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XGROUP");
        cmd.arg("SETID").arg(key).arg(group).arg(id);

        if let Some(er) = entries_read {
            cmd.arg("ENTRIESREAD").arg(er);
        }

        let _: () = cmd.query_async(&mut *conn).await?;
        Ok(())
    }

    async fn xgroup_createconsumer(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
    ) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("XGROUP")
            .arg("CREATECONSUMER")
            .arg(key)
            .arg(group)
            .arg(consumer)
            .query_async(&mut *conn)
            .await?;
        Ok(result == 1)
    }

    async fn xgroup_delconsumer(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
    ) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("XGROUP")
            .arg("DELCONSUMER")
            .arg(key)
            .arg(group)
            .arg(consumer)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn xinfo_groups(&self, key: &str) -> Result<Vec<ConsumerGroupInfo>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Value = redis::cmd("XINFO")
            .arg("GROUPS")
            .arg(key)
            .query_async(&mut *conn)
            .await?;

        Self::parse_xinfo_groups(result)
    }

    async fn xinfo_consumers(
        &self,
        key: &str,
        group: &str,
    ) -> Result<Vec<ConsumerInfo>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Value = redis::cmd("XINFO")
            .arg("CONSUMERS")
            .arg(key)
            .arg(group)
            .query_async(&mut *conn)
            .await?;

        Self::parse_xinfo_consumers(result)
    }

    // ========== Consumer Group Read Operations ==========

    async fn xreadgroup(
        &self,
        group: &str,
        consumer: &str,
        streams: &[(String, String)],
        options: XReadGroupOptions,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XREADGROUP");
        cmd.arg("GROUP").arg(group).arg(consumer);

        if let Some(count) = options.count {
            cmd.arg("COUNT").arg(count);
        }

        if let Some(block_ms) = options.block_ms {
            // Enforce max timeout (Architecture Decision 3)
            let max_ms = (MAX_BLOCKING_TIMEOUT_SECS * 1000) as i64;
            let actual_ms = block_ms.min(max_ms);
            cmd.arg("BLOCK").arg(actual_ms);
        }

        if options.no_ack {
            cmd.arg("NOACK");
        }

        cmd.arg("STREAMS");
        for (key, _) in streams {
            cmd.arg(key);
        }
        for (_, id) in streams {
            cmd.arg(id);
        }

        let result: Option<Vec<(String, Vec<(String, Vec<(String, String)>)>)>> =
            cmd.query_async(&mut *conn).await?;

        Ok(result.map(Self::parse_read_results))
    }

    async fn xreadgroup_blocking(
        &self,
        group: &str,
        consumer: &str,
        streams: &[(String, String)],
        count: Option<i64>,
        no_ack: bool,
        timeout: Duration,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
        // Enforce maximum timeout (Architecture Decision 3)
        let timeout = Self::enforce_max_timeout(timeout);

        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XREADGROUP");
        cmd.arg("GROUP").arg(group).arg(consumer);

        if let Some(c) = count {
            cmd.arg("COUNT").arg(c);
        }

        cmd.arg("BLOCK").arg(timeout.as_millis() as i64);

        if no_ack {
            cmd.arg("NOACK");
        }

        cmd.arg("STREAMS");
        for (key, _) in streams {
            cmd.arg(key);
        }
        for (_, id) in streams {
            cmd.arg(id);
        }

        let result: Option<Vec<(String, Vec<(String, Vec<(String, String)>)>)>> =
            cmd.query_async(&mut *conn).await?;

        Ok(result.map(Self::parse_read_results))
    }

    async fn xack(&self, key: &str, group: &str, ids: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XACK");
        cmd.arg(key).arg(group);
        for id in ids {
            cmd.arg(id);
        }
        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    // ========== Pending Entry Operations ==========

    async fn xpending_summary(&self, key: &str, group: &str) -> Result<PendingSummary, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Value = redis::cmd("XPENDING")
            .arg(key)
            .arg(group)
            .query_async(&mut *conn)
            .await?;

        Self::parse_pending_summary(result)
    }

    async fn xpending(
        &self,
        key: &str,
        group: &str,
        options: XPendingOptions,
    ) -> Result<Vec<PendingEntry>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XPENDING");
        cmd.arg(key).arg(group);

        if let Some(idle_ms) = options.idle_ms {
            cmd.arg("IDLE").arg(idle_ms);
        }

        // Start and end are required for detailed output
        let start = options.start.as_deref().unwrap_or("-");
        let end = options.end.as_deref().unwrap_or("+");
        let count = options.count.unwrap_or(100);

        cmd.arg(start).arg(end).arg(count);

        if let Some(consumer) = &options.consumer {
            cmd.arg(consumer);
        }

        let result: Vec<(String, String, i64, i64)> = cmd.query_async(&mut *conn).await?;

        Ok(result
            .into_iter()
            .map(
                |(id, consumer, idle_time_ms, delivery_count)| PendingEntry {
                    id,
                    consumer,
                    idle_time_ms,
                    delivery_count,
                },
            )
            .collect())
    }

    async fn xclaim(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
        ids: &[String],
        options: XClaimOptions,
    ) -> Result<ClaimResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XCLAIM");
        cmd.arg(key)
            .arg(group)
            .arg(consumer)
            .arg(options.min_idle_time_ms);

        for id in ids {
            cmd.arg(id);
        }

        if let Some(idle_ms) = options.idle_ms {
            cmd.arg("IDLE").arg(idle_ms);
        }
        if let Some(time_ms) = options.time_ms {
            cmd.arg("TIME").arg(time_ms);
        }
        if let Some(retry_count) = options.retry_count {
            cmd.arg("RETRYCOUNT").arg(retry_count);
        }
        if options.force {
            cmd.arg("FORCE");
        }
        if options.just_id {
            cmd.arg("JUSTID");
        }
        if let Some(last_id) = &options.last_id {
            cmd.arg("LASTID").arg(last_id);
        }

        if options.just_id {
            let ids: Vec<String> = cmd.query_async(&mut *conn).await?;
            let entries = ids
                .into_iter()
                .map(|id| StreamEntry {
                    id,
                    fields: HashMap::new(),
                })
                .collect();
            Ok(ClaimResult { entries })
        } else {
            let result: Vec<(String, Vec<(String, String)>)> = cmd.query_async(&mut *conn).await?;
            let entries = result
                .into_iter()
                .map(|(id, fields)| Self::parse_entry(id, fields))
                .collect();
            Ok(ClaimResult { entries })
        }
    }

    async fn xautoclaim(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
        min_idle_time_ms: i64,
        start: &str,
        options: XAutoClaimOptions,
    ) -> Result<AutoClaimResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XAUTOCLAIM");
        cmd.arg(key)
            .arg(group)
            .arg(consumer)
            .arg(min_idle_time_ms)
            .arg(start);

        if let Some(count) = options.count {
            cmd.arg("COUNT").arg(count);
        }
        if options.just_id {
            cmd.arg("JUSTID");
        }

        let result: Value = cmd.query_async(&mut *conn).await?;
        Self::parse_autoclaim_result(result, options.just_id)
    }

    // ========== Stream Management ==========

    async fn xsetid(
        &self,
        key: &str,
        last_id: &str,
        entries_added: Option<i64>,
        max_deleted_id: Option<&str>,
    ) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("XSETID");
        cmd.arg(key).arg(last_id);

        if let Some(ea) = entries_added {
            cmd.arg("ENTRIESADDED").arg(ea);
        }
        if let Some(max_del) = max_deleted_id {
            cmd.arg("MAXDELETEDID").arg(max_del);
        }

        let _: () = cmd.query_async(&mut *conn).await?;
        Ok(())
    }
}

// Helper methods for parsing Redis responses
impl RedisStreamRepository {
    fn parse_xinfo_stream(value: Value) -> Result<StreamInfo, CacheError> {
        // XINFO STREAM returns a map-like array
        let pairs = Self::value_to_pairs(value)?;
        let mut info = StreamInfo {
            length: 0,
            groups: 0,
            first_entry_id: None,
            last_entry_id: None,
            first_entry: None,
            last_entry: None,
            max_deleted_entry_id: None,
            entries_added: None,
            radix_tree_keys: None,
            radix_tree_nodes: None,
        };

        for (key, value) in pairs {
            match key.as_str() {
                "length" => info.length = Self::value_to_i64(&value).unwrap_or(0),
                "groups" => info.groups = Self::value_to_i64(&value).unwrap_or(0),
                "first-entry" => {
                    if let Some(entry) = Self::parse_optional_entry(value) {
                        info.first_entry_id = Some(entry.id.clone());
                        info.first_entry = Some(entry);
                    }
                }
                "last-entry" => {
                    if let Some(entry) = Self::parse_optional_entry(value) {
                        info.last_entry_id = Some(entry.id.clone());
                        info.last_entry = Some(entry);
                    }
                }
                "max-deleted-entry-id" => {
                    info.max_deleted_entry_id = Self::value_to_string(&value);
                }
                "entries-added" => {
                    info.entries_added = Self::value_to_i64(&value);
                }
                "radix-tree-keys" => {
                    info.radix_tree_keys = Self::value_to_i64(&value);
                }
                "radix-tree-nodes" => {
                    info.radix_tree_nodes = Self::value_to_i64(&value);
                }
                _ => {}
            }
        }

        Ok(info)
    }

    fn parse_optional_entry(value: Value) -> Option<StreamEntry> {
        match value {
            Value::Array(arr) if arr.len() >= 2 => {
                let id = Self::value_to_string(&arr[0])?;
                if let Value::Array(fields_arr) = &arr[1] {
                    let fields = Self::parse_fields_array(fields_arr);
                    Some(StreamEntry { id, fields })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn parse_fields_array(arr: &[Value]) -> HashMap<String, String> {
        let mut fields = HashMap::new();
        let mut iter = arr.iter();
        while let (Some(k), Some(v)) = (iter.next(), iter.next()) {
            if let (Some(key), Some(value)) = (Self::value_to_string(k), Self::value_to_string(v)) {
                fields.insert(key, value);
            }
        }
        fields
    }

    fn parse_xinfo_groups(value: Value) -> Result<Vec<ConsumerGroupInfo>, CacheError> {
        match value {
            Value::Array(groups) => {
                let mut result = Vec::new();
                for group in groups {
                    let pairs = Self::value_to_pairs(group)?;
                    let mut info = ConsumerGroupInfo {
                        name: String::new(),
                        consumers: 0,
                        pending: 0,
                        last_delivered_id: String::new(),
                        entries_read: None,
                        lag: None,
                    };

                    for (key, val) in pairs {
                        match key.as_str() {
                            "name" => info.name = Self::value_to_string(&val).unwrap_or_default(),
                            "consumers" => info.consumers = Self::value_to_i64(&val).unwrap_or(0),
                            "pending" => info.pending = Self::value_to_i64(&val).unwrap_or(0),
                            "last-delivered-id" => {
                                info.last_delivered_id =
                                    Self::value_to_string(&val).unwrap_or_default()
                            }
                            "entries-read" => info.entries_read = Self::value_to_i64(&val),
                            "lag" => info.lag = Self::value_to_i64(&val),
                            _ => {}
                        }
                    }
                    result.push(info);
                }
                Ok(result)
            }
            _ => Ok(vec![]),
        }
    }

    fn parse_xinfo_consumers(value: Value) -> Result<Vec<ConsumerInfo>, CacheError> {
        match value {
            Value::Array(consumers) => {
                let mut result = Vec::new();
                for consumer in consumers {
                    let pairs = Self::value_to_pairs(consumer)?;
                    let mut info = ConsumerInfo {
                        name: String::new(),
                        pending: 0,
                        idle_ms: 0,
                        inactive_ms: None,
                    };

                    for (key, val) in pairs {
                        match key.as_str() {
                            "name" => info.name = Self::value_to_string(&val).unwrap_or_default(),
                            "pending" => info.pending = Self::value_to_i64(&val).unwrap_or(0),
                            "idle" => info.idle_ms = Self::value_to_i64(&val).unwrap_or(0),
                            "inactive" => info.inactive_ms = Self::value_to_i64(&val),
                            _ => {}
                        }
                    }
                    result.push(info);
                }
                Ok(result)
            }
            _ => Ok(vec![]),
        }
    }

    fn parse_pending_summary(value: Value) -> Result<PendingSummary, CacheError> {
        match value {
            Value::Array(arr) if arr.len() >= 4 => {
                let count = Self::value_to_i64(&arr[0]).unwrap_or(0);
                let min_id = Self::value_to_string(&arr[1]);
                let max_id = Self::value_to_string(&arr[2]);

                let mut consumers = HashMap::new();
                if let Value::Array(consumer_arr) = &arr[3] {
                    for item in consumer_arr {
                        if let Value::Array(pair) = item
                            && pair.len() >= 2
                            && let (Some(name), Some(pending)) = (
                                Self::value_to_string(&pair[0]),
                                Self::value_to_string(&pair[1]),
                            )
                            && let Ok(p) = pending.parse::<i64>()
                        {
                            consumers.insert(name, p);
                        }
                    }
                }

                Ok(PendingSummary {
                    count,
                    min_id,
                    max_id,
                    consumers,
                })
            }
            _ => Ok(PendingSummary {
                count: 0,
                min_id: None,
                max_id: None,
                consumers: HashMap::new(),
            }),
        }
    }

    fn parse_autoclaim_result(value: Value, just_id: bool) -> Result<AutoClaimResult, CacheError> {
        match value {
            Value::Array(arr) if arr.len() >= 2 => {
                let next_id = Self::value_to_string(&arr[0]).unwrap_or_else(|| "0-0".to_string());

                let entries = if just_id {
                    // When JUSTID is used, entries are just IDs
                    match &arr[1] {
                        Value::Array(ids) => ids
                            .iter()
                            .filter_map(Self::value_to_string)
                            .map(|id| StreamEntry {
                                id,
                                fields: HashMap::new(),
                            })
                            .collect(),
                        _ => vec![],
                    }
                } else {
                    // Full entries
                    match &arr[1] {
                        Value::Array(entry_arr) => entry_arr
                            .iter()
                            .filter_map(|e| {
                                if let Value::Array(pair) = e
                                    && pair.len() >= 2
                                {
                                    let id = Self::value_to_string(&pair[0])?;
                                    if let Value::Array(fields_arr) = &pair[1] {
                                        let fields = Self::parse_fields_array(fields_arr);
                                        return Some(StreamEntry { id, fields });
                                    }
                                }
                                None
                            })
                            .collect(),
                        _ => vec![],
                    }
                };

                // Deleted IDs (Redis 7.0+)
                let deleted_ids = if arr.len() >= 3 {
                    match &arr[2] {
                        Value::Array(ids) => ids.iter().filter_map(Self::value_to_string).collect(),
                        _ => vec![],
                    }
                } else {
                    vec![]
                };

                Ok(AutoClaimResult {
                    next_id,
                    entries,
                    deleted_ids,
                })
            }
            _ => Ok(AutoClaimResult {
                next_id: "0-0".to_string(),
                entries: vec![],
                deleted_ids: vec![],
            }),
        }
    }

    fn value_to_pairs(value: Value) -> Result<Vec<(String, Value)>, CacheError> {
        match value {
            Value::Array(arr) => {
                let mut pairs = Vec::new();
                let mut iter = arr.into_iter();
                while let (Some(k), Some(v)) = (iter.next(), iter.next()) {
                    if let Some(key) = Self::value_to_string(&k) {
                        pairs.push((key, v));
                    }
                }
                Ok(pairs)
            }
            Value::Map(map) => {
                let mut pairs = Vec::new();
                for (k, v) in map {
                    if let Some(key) = Self::value_to_string(&k) {
                        pairs.push((key, v));
                    }
                }
                Ok(pairs)
            }
            _ => Ok(vec![]),
        }
    }

    fn value_to_string(value: &Value) -> Option<String> {
        match value {
            Value::BulkString(bytes) => String::from_utf8(bytes.clone()).ok(),
            Value::SimpleString(s) => Some(s.clone()),
            Value::Int(i) => Some(i.to_string()),
            _ => None,
        }
    }

    fn value_to_i64(value: &Value) -> Option<i64> {
        match value {
            Value::Int(i) => Some(*i),
            Value::BulkString(bytes) => String::from_utf8(bytes.clone()).ok()?.parse().ok(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enforce_max_timeout() {
        assert_eq!(
            RedisStreamRepository::enforce_max_timeout(Duration::from_secs(10)),
            Duration::from_secs(10)
        );
        assert_eq!(
            RedisStreamRepository::enforce_max_timeout(Duration::from_secs(60)),
            Duration::from_secs(30)
        );
        assert_eq!(
            RedisStreamRepository::enforce_max_timeout(Duration::from_secs(30)),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn test_parse_entry() {
        let fields = vec![
            ("user".to_string(), "alice".to_string()),
            ("action".to_string(), "login".to_string()),
        ];
        let entry = RedisStreamRepository::parse_entry("1704000001234-0".to_string(), fields);

        assert_eq!(entry.id, "1704000001234-0");
        assert_eq!(entry.fields.get("user"), Some(&"alice".to_string()));
        assert_eq!(entry.fields.get("action"), Some(&"login".to_string()));
    }
}
