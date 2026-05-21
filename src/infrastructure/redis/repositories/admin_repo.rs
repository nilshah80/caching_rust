//! Redis Admin Repository Implementation
//!
//! Concrete implementation of AdminRepository using Redis.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::domain::entities::{
    AclDryrunResult, AclLogEntry, BgRewriteAofResult, BgSaveResult, ClientInfo, ClientKillOptions,
    ClientPauseOptions, CopyKeyOptions, FlushOptions, FlushResult, HotkeysReport,
    HotkeysStartOptions, KeyAndFlags, LatencyEvent, MemoryStats, MemoryUsage, ModuleInfo,
    MoveKeyOptions, ServerInfo, ServerTime, SlowlogEntry, WaitAofResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::AdminRepository;
use crate::infrastructure::redis::blocking::query_with_blocking_timeout;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::pool_connection::PoolConnection;

/// Redis implementation of AdminRepository
pub struct RedisAdminRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisAdminRepository {
    /// Create a new RedisAdminRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AdminRepository for RedisAdminRepository {
    // ========================================================================
    // Server Operations
    // ========================================================================

    async fn get_server_info(&self) -> Result<ServerInfo, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let info: String = redis::cmd("INFO").query_async(&mut conn).await?;

        Ok(parse_server_info(&info))
    }

    async fn get_server_time(&self) -> Result<ServerTime, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let time: (i64, i64) = redis::cmd("TIME").query_async(&mut conn).await?;

        Ok(ServerTime {
            timestamp: time.0,
            microseconds: time.1,
        })
    }

    async fn get_db_size(&self) -> Result<i64, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let keys: i64 = redis::cmd("DBSIZE").query_async(&mut conn).await?;

        Ok(keys)
    }

    async fn get_last_save(&self) -> Result<i64, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let timestamp: i64 = redis::cmd("LASTSAVE").query_async(&mut conn).await?;

        Ok(timestamp)
    }

    async fn debug_object(&self, key: &str) -> Result<String, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let output: String = redis::cmd("DEBUG")
            .arg("OBJECT")
            .arg(key)
            .query_async(&mut conn)
            .await?;

        Ok(output)
    }

    async fn shutdown(&self, save: bool, now: bool) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mut cmd = redis::cmd("SHUTDOWN");
        cmd.arg(if save { "SAVE" } else { "NOSAVE" });
        if now {
            cmd.arg("NOW");
        }

        match cmd.query_async::<()>(&mut conn).await {
            Ok(()) => Ok(()),
            // Redis usually closes the connection before replying; treat that as success.
            Err(redis_err) if redis_err.is_connection_dropped() => Ok(()),
            Err(redis_err) => Err(CacheError::RedisError(redis_err)),
        }
    }

    async fn module_list(&self) -> Result<Vec<ModuleInfo>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let result: redis::Value = redis::cmd("MODULE")
            .arg("LIST")
            .query_async(&mut conn)
            .await?;

        parse_module_list(result)
    }

    // ========================================================================
    // Memory Operations
    // ========================================================================

    async fn get_memory_stats(&self) -> Result<MemoryStats, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let stats: Vec<redis::Value> = redis::cmd("MEMORY")
            .arg("STATS")
            .query_async(&mut conn)
            .await?;

        Ok(parse_memory_stats(&stats))
    }

    async fn get_memory_usage(&self, key: &str, samples: u32) -> Result<MemoryUsage, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let bytes: Option<i64> = redis::cmd("MEMORY")
            .arg("USAGE")
            .arg(key)
            .arg("SAMPLES")
            .arg(samples)
            .query_async(&mut conn)
            .await?;

        Ok(MemoryUsage {
            key: key.to_string(),
            bytes,
        })
    }

    async fn memory_doctor(&self) -> Result<String, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let report: String = redis::cmd("MEMORY")
            .arg("DOCTOR")
            .query_async(&mut conn)
            .await?;

        Ok(report)
    }

    async fn memory_purge(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("MEMORY")
            .arg("PURGE")
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    // ========================================================================
    // Database Operations
    // ========================================================================

    async fn flush_db(&self, options: FlushOptions) -> Result<FlushResult, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mode = if options.async_mode { "ASYNC" } else { "SYNC" };

        let mut cmd = redis::cmd("FLUSHDB");
        if options.async_mode {
            cmd.arg("ASYNC");
        }

        let _: () = cmd.query_async(&mut conn).await?;

        Ok(FlushResult {
            success: true,
            mode: mode.to_string(),
        })
    }

    async fn flush_all(&self, options: FlushOptions) -> Result<FlushResult, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mode = if options.async_mode { "ASYNC" } else { "SYNC" };

        let mut cmd = redis::cmd("FLUSHALL");
        if options.async_mode {
            cmd.arg("ASYNC");
        }

        let _: () = cmd.query_async(&mut conn).await?;

        Ok(FlushResult {
            success: true,
            mode: mode.to_string(),
        })
    }

    async fn copy_key(&self, options: CopyKeyOptions) -> Result<bool, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mut cmd = redis::cmd("COPY");
        cmd.arg(&options.source).arg(&options.destination);
        if let Some(db) = options.db {
            cmd.arg("DB").arg(db);
        }
        if options.replace {
            cmd.arg("REPLACE");
        }

        let result: i64 = cmd.query_async(&mut conn).await?;

        Ok(result == 1)
    }

    async fn move_key(&self, options: MoveKeyOptions) -> Result<bool, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let result: i64 = redis::cmd("MOVE")
            .arg(&options.key)
            .arg(options.db)
            .query_async(&mut conn)
            .await?;

        Ok(result == 1)
    }

    async fn swap_db(&self, db1: u8, db2: u8) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("SWAPDB")
            .arg(db1)
            .arg(db2)
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    // ========================================================================
    // Configuration Operations
    // ========================================================================

    async fn config_get(&self, pattern: &str) -> Result<HashMap<String, String>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let result: Vec<String> = redis::cmd("CONFIG")
            .arg("GET")
            .arg(pattern)
            .query_async(&mut conn)
            .await?;

        let mut config = HashMap::new();
        let mut iter = result.iter();
        while let Some(key) = iter.next() {
            if let Some(value) = iter.next() {
                config.insert(key.clone(), value.clone());
            }
        }

        Ok(config)
    }

    async fn config_set(&self, parameter: &str, value: &str) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("CONFIG")
            .arg("SET")
            .arg(parameter)
            .arg(value)
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    async fn config_rewrite(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("CONFIG")
            .arg("REWRITE")
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    async fn config_resetstat(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("CONFIG")
            .arg("RESETSTAT")
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    // ========================================================================
    // Persistence Operations
    // ========================================================================

    async fn save(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("SAVE").query_async(&mut conn).await?;

        Ok(())
    }

    async fn bgsave(&self, schedule: bool) -> Result<BgSaveResult, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mut cmd = redis::cmd("BGSAVE");
        if schedule {
            cmd.arg("SCHEDULE");
        }
        let result: String = cmd.query_async(&mut conn).await?;

        Ok(BgSaveResult {
            started: true,
            message: result,
        })
    }

    async fn bgrewriteaof(&self) -> Result<BgRewriteAofResult, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let result: String = redis::cmd("BGREWRITEAOF").query_async(&mut conn).await?;

        Ok(BgRewriteAofResult {
            started: true,
            message: result,
        })
    }

    // ========================================================================
    // Client Operations
    // ========================================================================

    async fn client_list(&self) -> Result<Vec<ClientInfo>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let result: String = redis::cmd("CLIENT")
            .arg("LIST")
            .query_async(&mut conn)
            .await?;

        Ok(parse_client_list(&result))
    }

    async fn client_kill(&self, options: ClientKillOptions) -> Result<i64, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mut cmd = redis::cmd("CLIENT");
        cmd.arg("KILL");

        if let Some(id) = options.id {
            cmd.arg("ID").arg(id);
        }
        if let Some(addr) = &options.addr {
            cmd.arg("ADDR").arg(addr);
        }
        if let Some(client_type) = &options.client_type {
            cmd.arg("TYPE").arg(client_type);
        }

        let killed: i64 = cmd.query_async(&mut conn).await?;

        Ok(killed)
    }

    async fn client_pause(&self, options: ClientPauseOptions) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("CLIENT")
            .arg("PAUSE")
            .arg(options.timeout_ms)
            .arg(options.mode.to_uppercase())
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    async fn client_unpause(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("CLIENT")
            .arg("UNPAUSE")
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    async fn client_setname(&self, name: &str) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("CLIENT")
            .arg("SETNAME")
            .arg(name)
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    async fn client_getname(&self) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let name: Option<String> = redis::cmd("CLIENT")
            .arg("GETNAME")
            .query_async(&mut conn)
            .await?;

        Ok(name)
    }

    async fn client_id(&self) -> Result<i64, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let id: i64 = redis::cmd("CLIENT")
            .arg("ID")
            .query_async(&mut conn)
            .await?;

        Ok(id)
    }

    async fn client_info(&self) -> Result<ClientInfo, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let result: String = redis::cmd("CLIENT")
            .arg("INFO")
            .query_async(&mut conn)
            .await?;

        Ok(parse_client_list(&result)
            .into_iter()
            .next()
            .unwrap_or_default())
    }

    // ========================================================================
    // Slowlog Operations
    // ========================================================================

    async fn slowlog_get(&self, count: i64) -> Result<Vec<SlowlogEntry>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let result: Vec<Vec<redis::Value>> = redis::cmd("SLOWLOG")
            .arg("GET")
            .arg(count)
            .query_async(&mut conn)
            .await?;

        Ok(parse_slowlog_entries(&result))
    }

    async fn slowlog_len(&self) -> Result<i64, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let length: i64 = redis::cmd("SLOWLOG")
            .arg("LEN")
            .query_async(&mut conn)
            .await?;

        Ok(length)
    }

    async fn slowlog_reset(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("SLOWLOG")
            .arg("RESET")
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    // ========================================================================
    // Latency Operations
    // ========================================================================

    async fn latency_latest(&self) -> Result<Vec<LatencyEvent>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let result: Vec<Vec<redis::Value>> = redis::cmd("LATENCY")
            .arg("LATEST")
            .query_async(&mut conn)
            .await?;

        Ok(parse_latency_events(&result))
    }

    async fn latency_history(&self, event: &str) -> Result<Vec<LatencyEvent>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let result: Vec<Vec<redis::Value>> = redis::cmd("LATENCY")
            .arg("HISTORY")
            .arg(event)
            .query_async(&mut conn)
            .await?;

        let samples = result
            .iter()
            .filter_map(|entry| {
                if entry.len() >= 2 {
                    let timestamp = match &entry[0] {
                        redis::Value::Int(v) => *v,
                        _ => return None,
                    };
                    let latency_ms = match &entry[1] {
                        redis::Value::Int(v) => *v,
                        _ => return None,
                    };
                    Some(LatencyEvent {
                        event: event.to_string(),
                        timestamp,
                        latency_ms,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(samples)
    }

    async fn latency_doctor(&self) -> Result<String, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let report: String = redis::cmd("LATENCY")
            .arg("DOCTOR")
            .query_async(&mut conn)
            .await?;

        Ok(report)
    }

    async fn latency_reset(&self, events: &[String]) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mut cmd = redis::cmd("LATENCY");
        cmd.arg("RESET");
        for event in events {
            cmd.arg(event);
        }

        let _: i64 = cmd.query_async(&mut conn).await?;

        Ok(())
    }

    async fn latency_graph(&self, event: &str) -> Result<String, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let graph: String = redis::cmd("LATENCY")
            .arg("GRAPH")
            .arg(event)
            .query_async(&mut conn)
            .await?;

        Ok(graph)
    }

    // ========================================================================
    // ACL Operations
    // ========================================================================

    async fn acl_list(&self) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let rules: Vec<String> = redis::cmd("ACL").arg("LIST").query_async(&mut conn).await?;

        Ok(rules)
    }

    async fn acl_users(&self) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let users: Vec<String> = redis::cmd("ACL")
            .arg("USERS")
            .query_async(&mut conn)
            .await?;

        Ok(users)
    }

    async fn acl_whoami(&self) -> Result<String, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let username: String = redis::cmd("ACL")
            .arg("WHOAMI")
            .query_async(&mut conn)
            .await?;

        Ok(username)
    }

    async fn acl_cat(&self, category: Option<&str>) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mut cmd = redis::cmd("ACL");
        cmd.arg("CAT");
        if let Some(cat) = category {
            cmd.arg(cat);
        }

        let items: Vec<String> = cmd.query_async(&mut conn).await?;

        Ok(items)
    }

    async fn acl_genpass(&self, bits: u32) -> Result<String, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let password: String = redis::cmd("ACL")
            .arg("GENPASS")
            .arg(bits)
            .query_async(&mut conn)
            .await?;

        Ok(password)
    }

    async fn acl_log(
        &self,
        count: Option<i64>,
        reset: bool,
    ) -> Result<Vec<AclLogEntry>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mut cmd = redis::cmd("ACL");
        cmd.arg("LOG");
        if let Some(c) = count {
            cmd.arg(c);
        }
        if reset {
            cmd.arg("RESET");
        }

        let result: Vec<Vec<redis::Value>> = cmd.query_async(&mut conn).await?;

        Ok(parse_acl_log_entries(&result))
    }

    async fn acl_dryrun(
        &self,
        username: &str,
        command: &[String],
    ) -> Result<AclDryrunResult, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mut cmd = redis::cmd("ACL");
        cmd.arg("DRYRUN").arg(username);
        for arg in command {
            cmd.arg(arg.as_str());
        }

        // ACL DRYRUN returns "OK" on success, or an error string on denial
        let result: String = cmd.query_async(&mut conn).await?;
        if result == "OK" {
            Ok(AclDryrunResult {
                allowed: true,
                reason: None,
            })
        } else {
            Ok(AclDryrunResult {
                allowed: false,
                reason: Some(result),
            })
        }
    }

    async fn acl_setuser(&self, username: &str, rules: &[String]) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mut cmd = redis::cmd("ACL");
        cmd.arg("SETUSER").arg(username);
        for rule in rules {
            cmd.arg(rule.as_str());
        }
        let _: () = cmd.query_async(&mut conn).await?;

        Ok(())
    }

    async fn acl_deluser(&self, usernames: &[String]) -> Result<i64, CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let mut cmd = redis::cmd("ACL");
        cmd.arg("DELUSER");
        for username in usernames {
            cmd.arg(username.as_str());
        }
        let deleted: i64 = cmd.query_async(&mut conn).await?;

        Ok(deleted)
    }

    async fn acl_load(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("ACL").arg("LOAD").query_async(&mut conn).await?;

        Ok(())
    }

    async fn acl_save(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;

        let _: () = redis::cmd("ACL").arg("SAVE").query_async(&mut conn).await?;

        Ok(())
    }

    // ========================================================================
    // Command Introspection Operations
    // ========================================================================

    async fn command_list(&self, filter: Option<&str>) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let mut cmd = redis::cmd("COMMAND");
        cmd.arg("LIST");
        if let Some(f) = filter {
            cmd.arg("FILTERBY").arg("PATTERN").arg(f);
        }
        let result: Vec<String> = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn command_count(&self) -> Result<i64, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let result: i64 = redis::cmd("COMMAND")
            .arg("COUNT")
            .query_async(&mut conn)
            .await?;
        Ok(result)
    }

    async fn command_docs(&self, commands: &[String]) -> Result<serde_json::Value, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let mut cmd = redis::cmd("COMMAND");
        cmd.arg("DOCS");
        for c in commands {
            cmd.arg(c.as_str());
        }
        let result: redis::Value = cmd.query_async(&mut conn).await?;
        Ok(redis_value_to_json(result))
    }

    async fn command_info(&self, commands: &[String]) -> Result<serde_json::Value, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let mut cmd = redis::cmd("COMMAND");
        cmd.arg("INFO");
        for c in commands {
            cmd.arg(c.as_str());
        }
        let result: redis::Value = cmd.query_async(&mut conn).await?;
        Ok(redis_value_to_json(result))
    }

    async fn command_getkeys(&self, command: &[String]) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let mut cmd = redis::cmd("COMMAND");
        cmd.arg("GETKEYS");
        for arg in command {
            cmd.arg(arg.as_str());
        }
        let result: Vec<String> = cmd.query_async(&mut conn).await?;
        Ok(result)
    }

    async fn command_getkeysandflags(
        &self,
        command: &[String],
    ) -> Result<Vec<KeyAndFlags>, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let mut cmd = redis::cmd("COMMAND");
        cmd.arg("GETKEYSANDFLAGS");
        for arg in command {
            cmd.arg(arg.as_str());
        }
        let result: redis::Value = cmd.query_async(&mut conn).await?;
        parse_keys_and_flags(result)
    }

    async fn latency_histogram(
        &self,
        commands: &[String],
    ) -> Result<serde_json::Value, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let mut cmd = redis::cmd("LATENCY");
        cmd.arg("HISTOGRAM");
        for c in commands {
            cmd.arg(c.as_str());
        }
        let result: redis::Value = cmd.query_async(&mut conn).await?;
        Ok(redis_value_to_json(result))
    }

    async fn memory_malloc_stats(&self) -> Result<String, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let stats: String = redis::cmd("MEMORY")
            .arg("MALLOC-STATS")
            .query_async(&mut conn)
            .await?;
        Ok(stats)
    }

    async fn hotkeys_start(&self, options: HotkeysStartOptions) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let mut cmd = redis::cmd("HOTKEYS");
        cmd.arg("START");

        // METRICS <count> [CPU] [NET] — count is the number of metric flags.
        let metric_count: u32 = u32::from(options.cpu) + u32::from(options.net);
        cmd.arg("METRICS").arg(metric_count);
        if options.cpu {
            cmd.arg("CPU");
        }
        if options.net {
            cmd.arg("NET");
        }

        if let Some(k) = options.top_k {
            cmd.arg("COUNT").arg(k);
        }
        if let Some(d) = options.duration_seconds {
            cmd.arg("DURATION").arg(d);
        }
        if let Some(r) = options.sample_ratio {
            cmd.arg("SAMPLE").arg(r);
        }
        let expanded = expand_hotkeys_slot_ranges(&options.slots);
        if !expanded.is_empty() {
            // Redis 8.6 wire format: `SLOTS <count> <slot> [<slot> ...]` — each
            // entry is an individual slot number, not a range pair. The repo
            // therefore expands the `[start,end]` ranges supplied at the API
            // boundary before forwarding them to Redis.
            cmd.arg("SLOTS").arg(expanded.len());
            for slot in &expanded {
                cmd.arg(*slot);
            }
        }

        cmd.query_async::<()>(&mut conn).await?;
        Ok(())
    }

    async fn hotkeys_stop(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        redis::cmd("HOTKEYS")
            .arg("STOP")
            .query_async::<()>(&mut conn)
            .await?;
        Ok(())
    }

    async fn hotkeys_get(&self) -> Result<HotkeysReport, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let result: redis::Value = redis::cmd("HOTKEYS")
            .arg("GET")
            .query_async(&mut conn)
            .await?;
        Ok(HotkeysReport {
            data: redis_value_to_json(result),
        })
    }

    async fn hotkeys_reset(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        redis::cmd("HOTKEYS")
            .arg("RESET")
            .query_async::<()>(&mut conn)
            .await?;
        Ok(())
    }

    async fn wait_aof(
        &self,
        numlocal: u64,
        numreplicas: u64,
        timeout_ms: u64,
    ) -> Result<WaitAofResult, CacheError> {
        let mut conn = PoolConnection::Standalone(self.pool.get_standalone().await?);
        let mut cmd = redis::cmd("WAITAOF");
        cmd.arg(numlocal).arg(numreplicas).arg(timeout_ms);
        let reply: (i64, i64) = query_with_blocking_timeout(
            &mut conn,
            &mut cmd,
            Duration::from_millis(timeout_ms),
            self.pool.response_timeout(),
        )
        .await?;
        Ok(WaitAofResult {
            local: reply.0,
            replicas: reply.1,
        })
    }

    async fn client_unblock(&self, client_id: i64, error: bool) -> Result<i64, CacheError> {
        let mut conn = self.pool.get_standalone().await?;
        let mut cmd = redis::cmd("CLIENT");
        cmd.arg("UNBLOCK").arg(client_id);
        if error {
            cmd.arg("ERROR");
        } else {
            cmd.arg("TIMEOUT");
        }
        let reply: i64 = cmd.query_async(&mut conn).await?;
        Ok(reply)
    }
}

/// Flatten `HOTKEYS START` slot ranges into the individual slot list Redis
/// expects on the wire (`SLOTS count slot [slot ...]`).
///
/// Each `[start, end]` range is expanded to every slot it covers. Ordering and
/// duplicates from the caller are preserved — Redis tolerates either.
/// Service-layer validation (see [`crate::application::services::AdminService::hotkeys_start`])
/// guarantees `start ≤ end ≤ 16_383` so the inclusive range iteration cannot
/// overflow or run unbounded.
pub(crate) fn expand_hotkeys_slot_ranges(
    ranges: &[crate::domain::entities::HotkeysSlotRange],
) -> Vec<u16> {
    let mut out = Vec::new();
    for range in ranges {
        if range.start > range.end {
            // Defensive: validation should have rejected this already.
            continue;
        }
        for slot in range.start..=range.end {
            out.push(slot);
        }
    }
    out
}

/// Parse the array reply from `COMMAND GETKEYSANDFLAGS` into typed entries.
///
/// Redis returns an outer array where each item is `[key_bulk, [flag_bulk...]]`.
/// Anything that doesn't match that shape is treated as an unexpected reply.
fn parse_keys_and_flags(value: redis::Value) -> Result<Vec<KeyAndFlags>, CacheError> {
    let outer = match value {
        redis::Value::Array(items) => items,
        _ => {
            return Err(CacheError::Internal(
                "Unexpected reply shape from COMMAND GETKEYSANDFLAGS".to_string(),
            ));
        }
    };

    let mut out = Vec::with_capacity(outer.len());
    for entry in outer {
        let parts = match entry {
            redis::Value::Array(p) if p.len() >= 2 => p,
            _ => {
                return Err(CacheError::Internal(
                    "Malformed COMMAND GETKEYSANDFLAGS entry".to_string(),
                ));
            }
        };

        let key = match &parts[0] {
            redis::Value::BulkString(bytes) => String::from_utf8_lossy(bytes).into_owned(),
            redis::Value::SimpleString(s) => s.clone(),
            _ => {
                return Err(CacheError::Internal(
                    "Expected bulk string for key in COMMAND GETKEYSANDFLAGS".to_string(),
                ));
            }
        };

        let flags = match &parts[1] {
            redis::Value::Array(arr) => arr
                .iter()
                .filter_map(|v| match v {
                    redis::Value::BulkString(bytes) => {
                        Some(String::from_utf8_lossy(bytes).into_owned())
                    }
                    redis::Value::SimpleString(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };

        out.push(KeyAndFlags { key, flags });
    }
    Ok(out)
}

// ============================================================================
// Helper: Convert redis::Value to serde_json::Value
// ============================================================================

fn redis_value_to_json(value: redis::Value) -> serde_json::Value {
    match value {
        redis::Value::Nil => serde_json::Value::Null,
        redis::Value::Int(i) => serde_json::json!(i),
        redis::Value::BulkString(bytes) => String::from_utf8(bytes)
            .map(|s| serde_json::json!(s))
            .unwrap_or(serde_json::Value::Null),
        redis::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(redis_value_to_json).collect())
        }
        redis::Value::SimpleString(s) => serde_json::json!(s),
        redis::Value::Okay => serde_json::json!("OK"),
        redis::Value::Map(pairs) => {
            let obj: serde_json::Map<String, serde_json::Value> = pairs
                .into_iter()
                .filter_map(|(k, v)| {
                    let key = match k {
                        redis::Value::BulkString(key_bytes) => String::from_utf8(key_bytes).ok(),
                        redis::Value::SimpleString(s) => Some(s),
                        _ => None,
                    };
                    key.map(|k| (k, redis_value_to_json(v)))
                })
                .collect();
            serde_json::Value::Object(obj)
        }
        _ => serde_json::Value::Null,
    }
}

// ============================================================================
// Parsing Helper Functions
// ============================================================================

fn value_to_string(value: &redis::Value) -> Option<String> {
    match value {
        redis::Value::BulkString(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        redis::Value::SimpleString(s) => Some(s.clone()),
        redis::Value::Int(n) => Some(n.to_string()),
        _ => None,
    }
}

fn value_to_i64(value: &redis::Value) -> Option<i64> {
    match value {
        redis::Value::Int(n) => Some(*n),
        redis::Value::BulkString(bytes) => std::str::from_utf8(bytes).ok()?.parse().ok(),
        redis::Value::SimpleString(s) => s.parse().ok(),
        _ => None,
    }
}

fn array_fields_to_pairs(
    fields: Vec<redis::Value>,
) -> Result<Vec<(redis::Value, redis::Value)>, CacheError> {
    let mut iter = fields.into_iter();
    let mut pairs = Vec::new();
    while let Some(key) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(CacheError::Internal(
                "Malformed MODULE LIST entry".to_string(),
            ));
        };
        pairs.push((key, value));
    }
    Ok(pairs)
}

fn parse_module_info(value: redis::Value) -> Result<ModuleInfo, CacheError> {
    let pairs = match value {
        redis::Value::Map(pairs) => pairs,
        redis::Value::Array(fields) => array_fields_to_pairs(fields)?,
        _ => {
            return Err(CacheError::Internal(
                "Unexpected module entry in MODULE LIST reply".to_string(),
            ));
        }
    };

    let mut info = ModuleInfo::default();
    for (key, value) in pairs {
        let Some(key) = value_to_string(&key).map(|s| s.to_ascii_lowercase()) else {
            continue;
        };
        match key.as_str() {
            "name" => {
                if let Some(name) = value_to_string(&value) {
                    info.name = name;
                }
            }
            "ver" | "version" => {
                if let Some(version) = value_to_i64(&value) {
                    info.version = version;
                }
            }
            "path" => {
                if let Some(path) = value_to_string(&value) {
                    info.path = path;
                }
            }
            "args" => {
                info.args = match value {
                    redis::Value::Array(values) => {
                        values.iter().filter_map(value_to_string).collect()
                    }
                    other => value_to_string(&other).into_iter().collect(),
                };
            }
            _ => {}
        }
    }

    Ok(info)
}

fn parse_module_list(value: redis::Value) -> Result<Vec<ModuleInfo>, CacheError> {
    let modules = match value {
        redis::Value::Array(modules) => modules,
        redis::Value::Nil => return Ok(Vec::new()),
        _ => {
            return Err(CacheError::Internal(
                "Unexpected reply shape from MODULE LIST".to_string(),
            ));
        }
    };

    modules.into_iter().map(parse_module_info).collect()
}

/// Parse INFO output into ServerInfo
fn parse_server_info(info: &str) -> ServerInfo {
    let mut response = ServerInfo::default();

    for line in info.lines() {
        if let Some((key, value)) = line.split_once(':') {
            match key {
                "redis_version" => response.redis_version = value.to_string(),
                "uptime_in_seconds" => response.uptime_seconds = value.parse().unwrap_or(0),
                "connected_clients" => response.connected_clients = value.parse().unwrap_or(0),
                "used_memory" => response.used_memory = value.parse().unwrap_or(0),
                "used_memory_human" => response.used_memory_human = value.to_string(),
                "total_system_memory" => response.total_system_memory = value.parse().unwrap_or(0),
                "used_memory_peak" => response.used_memory_peak = value.parse().unwrap_or(0),
                "expired_keys" => response.expired_keys = value.parse().unwrap_or(0),
                "keyspace_hits" => response.keyspace_hits = value.parse().unwrap_or(0),
                "keyspace_misses" => response.keyspace_misses = value.parse().unwrap_or(0),
                key if key.starts_with("db") => {
                    // Parse keyspace info like "db0:keys=5,expires=0,avg_ttl=0"
                    if let Some(keys_str) = value.split(',').next()
                        && let Some(count) = keys_str.strip_prefix("keys=")
                    {
                        response.total_keys += count.parse::<i64>().unwrap_or(0);
                    }
                }
                _ => {}
            }
        }
    }

    response
}

/// Parse MEMORY STATS output
fn parse_memory_stats(stats: &[redis::Value]) -> MemoryStats {
    let mut response = MemoryStats::default();

    let mut iter = stats.iter();
    while let Some(key) = iter.next() {
        if let redis::Value::BulkString(key_bytes) = key
            && let Ok(key_str) = std::str::from_utf8(key_bytes)
            && let Some(value) = iter.next()
        {
            match key_str {
                "peak.allocated" => {
                    if let redis::Value::Int(v) = value {
                        response.peak_allocated = *v;
                    }
                }
                "total.allocated" => {
                    if let redis::Value::Int(v) = value {
                        response.total_allocated = *v;
                    }
                }
                "startup.allocated" => {
                    if let redis::Value::Int(v) = value {
                        response.startup_allocated = *v;
                    }
                }
                "replication.backlog" => {
                    if let redis::Value::Int(v) = value {
                        response.replication_backlog = *v;
                    }
                }
                "clients.normal" => {
                    if let redis::Value::Int(v) = value {
                        response.clients_normal = *v;
                    }
                }
                "clients.slaves" => {
                    if let redis::Value::Int(v) = value {
                        response.clients_slaves = *v;
                    }
                }
                "aof.buffer" => {
                    if let redis::Value::Int(v) = value {
                        response.aof_buffer = *v;
                    }
                }
                "lua.caches" => {
                    if let redis::Value::Int(v) = value {
                        response.lua_caches = *v;
                    }
                }
                "overhead.total" => {
                    if let redis::Value::Int(v) = value {
                        response.overhead_total = *v;
                    }
                }
                "dataset.bytes" => {
                    if let redis::Value::Int(v) = value {
                        response.dataset_bytes = *v;
                    }
                }
                "dataset.percentage" => {
                    if let redis::Value::BulkString(v) = value
                        && let Ok(s) = std::str::from_utf8(v)
                    {
                        response.dataset_perc = s.parse().unwrap_or(0.0);
                    }
                }
                "peak.percentage" => {
                    if let redis::Value::BulkString(v) = value
                        && let Ok(s) = std::str::from_utf8(v)
                    {
                        response.peak_perc = s.parse().unwrap_or(0.0);
                    }
                }
                "fragmentation" => {
                    if let redis::Value::BulkString(v) = value
                        && let Ok(s) = std::str::from_utf8(v)
                    {
                        response.fragmentation = s.parse().unwrap_or(0.0);
                    }
                }
                _ => {}
            }
        }
    }

    response
}

/// Parse CLIENT LIST output
fn parse_client_list(output: &str) -> Vec<ClientInfo> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut info = ClientInfo::default();

            for part in line.split_whitespace() {
                if let Some((key, value)) = part.split_once('=') {
                    match key {
                        "id" => info.id = value.parse().unwrap_or(0),
                        "name" => info.name = value.to_string(),
                        "addr" => info.addr = value.to_string(),
                        "fd" => info.fd = value.parse().unwrap_or(0),
                        "age" => info.age = value.parse().unwrap_or(0),
                        "idle" => info.idle = value.parse().unwrap_or(0),
                        "flags" => info.flags = value.to_string(),
                        "db" => info.db = value.parse().unwrap_or(0),
                        "multi" => info.multi = value.parse().unwrap_or(-1),
                        "qbuf" => info.qbuf = value.parse().unwrap_or(0),
                        "qbuf-free" => info.qbuf_free = value.parse().unwrap_or(0),
                        "obl" => info.obl = value.parse().unwrap_or(0),
                        "oll" => info.oll = value.parse().unwrap_or(0),
                        "omem" => info.omem = value.parse().unwrap_or(0),
                        "cmd" => info.cmd = value.to_string(),
                        "lib-name" => info.lib_name = value.to_string(),
                        "lib-ver" => info.lib_ver = value.to_string(),
                        _ => {}
                    }
                }
            }

            info
        })
        .collect()
}

/// Parse SLOWLOG GET output
fn parse_slowlog_entries(entries: &[Vec<redis::Value>]) -> Vec<SlowlogEntry> {
    entries
        .iter()
        .filter_map(|entry| {
            if entry.len() >= 6 {
                let id = match &entry[0] {
                    redis::Value::Int(v) => *v,
                    _ => return None,
                };
                let timestamp = match &entry[1] {
                    redis::Value::Int(v) => *v,
                    _ => return None,
                };
                let duration_us = match &entry[2] {
                    redis::Value::Int(v) => *v,
                    _ => return None,
                };
                let command = match &entry[3] {
                    redis::Value::Array(arr) => arr
                        .iter()
                        .filter_map(|v| match v {
                            redis::Value::BulkString(b) => String::from_utf8(b.clone()).ok(),
                            _ => None,
                        })
                        .collect(),
                    _ => vec![],
                };
                let client_addr = match &entry[4] {
                    redis::Value::BulkString(b) => String::from_utf8(b.clone()).unwrap_or_default(),
                    _ => String::new(),
                };
                let client_name = match &entry[5] {
                    redis::Value::BulkString(b) => String::from_utf8(b.clone()).unwrap_or_default(),
                    _ => String::new(),
                };

                Some(SlowlogEntry {
                    id,
                    timestamp,
                    duration_us,
                    command,
                    client_addr,
                    client_name,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Parse LATENCY LATEST output
fn parse_latency_events(entries: &[Vec<redis::Value>]) -> Vec<LatencyEvent> {
    entries
        .iter()
        .filter_map(|entry| {
            if entry.len() >= 3 {
                let event = match &entry[0] {
                    redis::Value::BulkString(b) => String::from_utf8(b.clone()).ok()?,
                    _ => return None,
                };
                let timestamp = match &entry[1] {
                    redis::Value::Int(v) => *v,
                    _ => return None,
                };
                let latency_ms = match &entry[2] {
                    redis::Value::Int(v) => *v,
                    _ => return None,
                };

                Some(LatencyEvent {
                    event,
                    timestamp,
                    latency_ms,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Parse ACL LOG output
fn parse_acl_log_entries(entries: &[Vec<redis::Value>]) -> Vec<AclLogEntry> {
    entries
        .iter()
        .map(|entry| {
            let mut log_entry = AclLogEntry::default();

            let mut iter = entry.iter();
            while let Some(key) = iter.next() {
                if let redis::Value::BulkString(key_bytes) = key
                    && let Ok(key_str) = std::str::from_utf8(key_bytes)
                    && let Some(value) = iter.next()
                {
                    match key_str {
                        "count" => {
                            if let redis::Value::Int(v) = value {
                                log_entry.count = *v;
                            }
                        }
                        "reason" => {
                            if let redis::Value::BulkString(v) = value {
                                log_entry.reason = String::from_utf8(v.clone()).unwrap_or_default();
                            }
                        }
                        "context" => {
                            if let redis::Value::BulkString(v) = value {
                                log_entry.context =
                                    String::from_utf8(v.clone()).unwrap_or_default();
                            }
                        }
                        "object" => {
                            if let redis::Value::BulkString(v) = value {
                                log_entry.object = String::from_utf8(v.clone()).unwrap_or_default();
                            }
                        }
                        "username" => {
                            if let redis::Value::BulkString(v) = value {
                                log_entry.username =
                                    String::from_utf8(v.clone()).unwrap_or_default();
                            }
                        }
                        "age-seconds" => {
                            if let redis::Value::BulkString(v) = value
                                && let Ok(s) = std::str::from_utf8(v)
                            {
                                log_entry.age_seconds = s.parse().unwrap_or(0.0);
                            }
                        }
                        "client-info" => {
                            if let redis::Value::BulkString(v) = value {
                                log_entry.client_info =
                                    String::from_utf8(v.clone()).unwrap_or_default();
                            }
                        }
                        "entry-id" => {
                            if let redis::Value::Int(v) = value {
                                log_entry.entry_timestamp = *v;
                            }
                        }
                        "timestamp-created" => {
                            if let redis::Value::Int(v) = value {
                                log_entry.timestamp_us = *v;
                            }
                        }
                        _ => {}
                    }
                }
            }

            log_entry
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_server_info() {
        let info = "\
redis_version:7.2.0\n\
uptime_in_seconds:123\n\
connected_clients:4\n\
used_memory:1024\n\
used_memory_human:1K\n\
total_system_memory:2048\n\
used_memory_peak:1536\n\
expired_keys:1\n\
keyspace_hits:10\n\
keyspace_misses:2\n\
db0:keys=5,expires=0,avg_ttl=0\n";
        let parsed = parse_server_info(info);
        assert_eq!(parsed.redis_version, "7.2.0");
        assert_eq!(parsed.uptime_seconds, 123);
        assert_eq!(parsed.total_keys, 5);
        assert_eq!(parsed.keyspace_hits, 10);
    }

    #[test]
    fn test_parse_memory_stats() {
        let stats = vec![
            redis::Value::BulkString(b"peak.allocated".to_vec()),
            redis::Value::Int(10),
            redis::Value::BulkString(b"dataset.percentage".to_vec()),
            redis::Value::BulkString(b"80.5".to_vec()),
        ];
        let parsed = parse_memory_stats(&stats);
        assert_eq!(parsed.peak_allocated, 10);
        assert_eq!(parsed.dataset_perc, 80.5);
    }

    #[test]
    fn test_parse_client_list() {
        let output = "id=1 name=test addr=127.0.0.1:6379 fd=7 age=10 idle=5 flags=N db=0 multi=-1 qbuf=0 qbuf-free=0 obl=0 oll=0 omem=0 cmd=get\n";
        let clients = parse_client_list(output);
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].id, 1);
        assert_eq!(clients[0].name, "test");
        assert_eq!(clients[0].cmd, "get");
        // Older Redis CLIENT LIST output omits lib-name / lib-ver; the parser
        // must leave them as empty strings rather than failing.
        assert!(clients[0].lib_name.is_empty());
        assert!(clients[0].lib_ver.is_empty());
    }

    #[test]
    fn test_parse_client_list_with_lib_metadata() {
        // Redis 7.2+ CLIENT LIST output includes `lib-name=` / `lib-ver=`
        // fields when the connection issued `CLIENT SETINFO`.
        let output = "id=42 name= addr=127.0.0.1:6379 fd=7 age=1 idle=0 flags=N \
                      db=0 multi=-1 qbuf=0 qbuf-free=0 obl=0 oll=0 omem=0 cmd=ping \
                      lib-name=redis-caching-service lib-ver=0.1.0\n";
        let clients = parse_client_list(output);
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].lib_name, "redis-caching-service");
        assert_eq!(clients[0].lib_ver, "0.1.0");
    }

    #[test]
    fn test_parse_slowlog_entries() {
        let entry = vec![
            redis::Value::Int(1),
            redis::Value::Int(2),
            redis::Value::Int(3),
            redis::Value::Array(vec![redis::Value::BulkString(b"GET".to_vec())]),
            redis::Value::BulkString(b"127.0.0.1:6379".to_vec()),
            redis::Value::BulkString(b"client".to_vec()),
        ];
        let parsed = parse_slowlog_entries(&[entry]);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].command, vec!["GET".to_string()]);
    }

    #[test]
    fn test_parse_latency_events() {
        let entry = vec![
            redis::Value::BulkString(b"command".to_vec()),
            redis::Value::Int(10),
            redis::Value::Int(5),
        ];
        let parsed = parse_latency_events(&[entry]);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].event, "command");
    }

    #[test]
    fn test_parse_acl_log_entries() {
        let entry = vec![
            redis::Value::BulkString(b"count".to_vec()),
            redis::Value::Int(3),
            redis::Value::BulkString(b"reason".to_vec()),
            redis::Value::BulkString(b"invalid".to_vec()),
            redis::Value::BulkString(b"timestamp-created".to_vec()),
            redis::Value::Int(99),
        ];
        let parsed = parse_acl_log_entries(&[entry]);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].count, 3);
        assert_eq!(parsed[0].reason, "invalid");
        assert_eq!(parsed[0].timestamp_us, 99);
    }

    #[test]
    fn test_redis_value_to_json_nil() {
        let result = redis_value_to_json(redis::Value::Nil);
        assert!(result.is_null());
    }

    #[test]
    fn test_redis_value_to_json_int() {
        let result = redis_value_to_json(redis::Value::Int(42));
        assert_eq!(result, serde_json::json!(42));
    }

    #[test]
    fn test_redis_value_to_json_bulk_string() {
        let result = redis_value_to_json(redis::Value::BulkString(b"hello".to_vec()));
        assert_eq!(result, serde_json::json!("hello"));
    }

    #[test]
    fn test_redis_value_to_json_bulk_string_invalid_utf8() {
        let result = redis_value_to_json(redis::Value::BulkString(vec![0xff, 0xfe]));
        assert!(result.is_null());
    }

    #[test]
    fn test_redis_value_to_json_array() {
        let result = redis_value_to_json(redis::Value::Array(vec![
            redis::Value::Int(1),
            redis::Value::BulkString(b"two".to_vec()),
            redis::Value::Nil,
        ]));
        assert_eq!(result, serde_json::json!([1, "two", null]));
    }

    #[test]
    fn test_redis_value_to_json_simple_string() {
        let result = redis_value_to_json(redis::Value::SimpleString("OK".to_string()));
        assert_eq!(result, serde_json::json!("OK"));
    }

    #[test]
    fn test_redis_value_to_json_okay() {
        let result = redis_value_to_json(redis::Value::Okay);
        assert_eq!(result, serde_json::json!("OK"));
    }

    #[test]
    fn test_redis_value_to_json_map() {
        let result = redis_value_to_json(redis::Value::Map(vec![
            (
                redis::Value::BulkString(b"name".to_vec()),
                redis::Value::BulkString(b"GET".to_vec()),
            ),
            (
                redis::Value::SimpleString("arity".to_string()),
                redis::Value::Int(2),
            ),
        ]));
        let obj = result.as_object().unwrap();
        assert_eq!(obj.get("name").unwrap(), &serde_json::json!("GET"));
        assert_eq!(obj.get("arity").unwrap(), &serde_json::json!(2));
    }

    #[test]
    fn test_redis_value_to_json_map_non_string_key() {
        // Non-string keys should be filtered out
        let result = redis_value_to_json(redis::Value::Map(vec![
            (
                redis::Value::Int(1),
                redis::Value::BulkString(b"value".to_vec()),
            ),
            (
                redis::Value::BulkString(b"valid".to_vec()),
                redis::Value::Int(42),
            ),
        ]));
        let obj = result.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert_eq!(obj.get("valid").unwrap(), &serde_json::json!(42));
    }

    #[test]
    fn test_redis_value_to_json_nested() {
        let result = redis_value_to_json(redis::Value::Array(vec![redis::Value::Map(vec![(
            redis::Value::BulkString(b"key".to_vec()),
            redis::Value::Array(vec![redis::Value::Int(1), redis::Value::Int(2)]),
        )])]));
        assert_eq!(result, serde_json::json!([{"key": [1, 2]}]));
    }

    #[test]
    fn test_parse_module_list_from_array_reply() {
        let parsed = parse_module_list(redis::Value::Array(vec![redis::Value::Array(vec![
            redis::Value::BulkString(b"name".to_vec()),
            redis::Value::BulkString(b"search".to_vec()),
            redis::Value::BulkString(b"ver".to_vec()),
            redis::Value::Int(20814),
            redis::Value::BulkString(b"path".to_vec()),
            redis::Value::BulkString(b"/redisearch.so".to_vec()),
            redis::Value::BulkString(b"args".to_vec()),
            redis::Value::Array(vec![
                redis::Value::BulkString(b"MAXSEARCHRESULTS".to_vec()),
                redis::Value::BulkString(b"10000".to_vec()),
            ]),
        ])]))
        .expect("parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "search");
        assert_eq!(parsed[0].version, 20814);
        assert_eq!(parsed[0].path, "/redisearch.so");
        assert_eq!(parsed[0].args, vec!["MAXSEARCHRESULTS", "10000"]);
    }

    #[test]
    fn test_parse_module_list_from_map_reply() {
        let parsed = parse_module_list(redis::Value::Array(vec![redis::Value::Map(vec![
            (
                redis::Value::SimpleString("name".to_string()),
                redis::Value::SimpleString("timeseries".to_string()),
            ),
            (
                redis::Value::SimpleString("version".to_string()),
                redis::Value::BulkString(b"11200".to_vec()),
            ),
        ])]))
        .expect("parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "timeseries");
        assert_eq!(parsed[0].version, 11200);
        assert!(parsed[0].path.is_empty());
        assert!(parsed[0].args.is_empty());
    }

    #[test]
    fn test_parse_module_list_rejects_malformed_reply() {
        assert!(parse_module_list(redis::Value::Nil).unwrap().is_empty());
        assert!(matches!(
            parse_module_list(redis::Value::Int(1)),
            Err(CacheError::Internal(_))
        ));
        assert!(matches!(
            parse_module_list(redis::Value::Array(vec![redis::Value::Array(vec![
                redis::Value::BulkString(b"name".to_vec())
            ])])),
            Err(CacheError::Internal(_))
        ));
    }

    #[test]
    fn test_expand_hotkeys_slot_ranges_empty() {
        assert!(expand_hotkeys_slot_ranges(&[]).is_empty());
    }

    #[test]
    fn test_expand_hotkeys_slot_ranges_single_slot() {
        let ranges = vec![crate::domain::entities::HotkeysSlotRange { start: 42, end: 42 }];
        assert_eq!(expand_hotkeys_slot_ranges(&ranges), vec![42]);
    }

    #[test]
    fn test_expand_hotkeys_slot_ranges_inclusive_range() {
        let ranges = vec![crate::domain::entities::HotkeysSlotRange { start: 10, end: 14 }];
        assert_eq!(
            expand_hotkeys_slot_ranges(&ranges),
            vec![10, 11, 12, 13, 14]
        );
    }

    #[test]
    fn test_expand_hotkeys_slot_ranges_multiple_ranges_preserve_order() {
        let ranges = vec![
            crate::domain::entities::HotkeysSlotRange { start: 0, end: 2 },
            crate::domain::entities::HotkeysSlotRange {
                start: 100,
                end: 101,
            },
        ];
        assert_eq!(expand_hotkeys_slot_ranges(&ranges), vec![0, 1, 2, 100, 101]);
    }

    #[test]
    fn test_expand_hotkeys_slot_ranges_skips_inverted_range() {
        // Defensive: service-layer validation rejects inverted ranges, but the
        // helper must never panic if one slips through.
        let ranges = vec![
            crate::domain::entities::HotkeysSlotRange { start: 5, end: 3 },
            crate::domain::entities::HotkeysSlotRange { start: 7, end: 7 },
        ];
        assert_eq!(expand_hotkeys_slot_ranges(&ranges), vec![7]);
    }

    #[test]
    fn test_expand_hotkeys_slot_ranges_max_slot() {
        let ranges = vec![crate::domain::entities::HotkeysSlotRange {
            start: 16_380,
            end: 16_383,
        }];
        assert_eq!(
            expand_hotkeys_slot_ranges(&ranges),
            vec![16_380, 16_381, 16_382, 16_383]
        );
    }

    #[test]
    fn test_parse_keys_and_flags_accepts_bulk_and_simple_strings() {
        let parsed = parse_keys_and_flags(redis::Value::Array(vec![
            redis::Value::Array(vec![
                redis::Value::BulkString(b"key:1".to_vec()),
                redis::Value::Array(vec![
                    redis::Value::BulkString(b"RW".to_vec()),
                    redis::Value::SimpleString("access".to_string()),
                    redis::Value::Int(99),
                ]),
            ]),
            redis::Value::Array(vec![
                redis::Value::SimpleString("key:2".to_string()),
                redis::Value::SimpleString("not-an-array".to_string()),
            ]),
        ]))
        .expect("parse");

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].key, "key:1");
        assert_eq!(parsed[0].flags, vec!["RW", "access"]);
        assert_eq!(parsed[1].key, "key:2");
        assert!(parsed[1].flags.is_empty());
    }

    #[test]
    fn test_parse_keys_and_flags_rejects_malformed_replies() {
        assert!(matches!(
            parse_keys_and_flags(redis::Value::Int(1)),
            Err(CacheError::Internal(_))
        ));

        assert!(matches!(
            parse_keys_and_flags(redis::Value::Array(vec![redis::Value::Array(vec![
                redis::Value::BulkString(b"only-key".to_vec())
            ])])),
            Err(CacheError::Internal(_))
        ));

        assert!(matches!(
            parse_keys_and_flags(redis::Value::Array(vec![redis::Value::Array(vec![
                redis::Value::Int(1),
                redis::Value::Array(vec![])
            ])])),
            Err(CacheError::Internal(_))
        ));
    }
}
