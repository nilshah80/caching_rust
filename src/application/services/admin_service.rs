//! Admin Service
//!
//! Business logic layer for admin operations.

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::entities::{
    AclDryrunResult, AclLogEntry, BgRewriteAofResult, BgSaveResult, ClientInfo, ClientKillOptions,
    ClientPauseOptions, CopyKeyOptions, FlushOptions, FlushResult, HotkeysReport,
    HotkeysStartOptions, KeyAndFlags, LatencyEvent, MemoryStats, MemoryUsage, ModuleInfo,
    MoveKeyOptions, ServerInfo, ServerTime, SlowlogEntry, WaitAofResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::AdminRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisAdminRepository;
use crate::shared::blocking::BlockingTimeoutEnforcer;
use std::time::Duration;

/// Service for admin operations
pub struct AdminService {
    repository: Arc<dyn AdminRepository>,
    timeout_enforcer: BlockingTimeoutEnforcer,
}

impl AdminService {
    /// Create a new AdminService
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisAdminRepository::new(pool)))
    }

    /// Create an AdminService with a custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn AdminRepository>) -> Self {
        Self {
            repository,
            timeout_enforcer: BlockingTimeoutEnforcer::new(),
        }
    }

    /// Set custom max blocking timeout (for testing or configuration).
    pub fn with_max_blocking_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_enforcer = BlockingTimeoutEnforcer::with_max(timeout.as_secs());
        self
    }

    fn enforce_wait_aof_timeout_ms(&self, timeout_ms: u64) -> u64 {
        let enforced = self
            .timeout_enforcer
            .enforce(Duration::from_millis(timeout_ms));
        enforced.as_millis() as u64
    }

    // ========================================================================
    // Server Operations
    // ========================================================================

    /// Get server information
    pub async fn get_server_info(&self) -> Result<ServerInfo, CacheError> {
        self.repository.get_server_info().await
    }

    /// Get server time
    pub async fn get_server_time(&self) -> Result<ServerTime, CacheError> {
        self.repository.get_server_time().await
    }

    /// Get database size
    pub async fn get_db_size(&self) -> Result<i64, CacheError> {
        self.repository.get_db_size().await
    }

    /// Get last save timestamp
    pub async fn get_last_save(&self) -> Result<i64, CacheError> {
        self.repository.get_last_save().await
    }

    /// Get limited debug information for a key
    pub async fn debug_object(&self, key: &str) -> Result<String, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        self.repository.debug_object(key).await
    }

    /// Shutdown the Redis server
    pub async fn shutdown(&self, save: bool, now: bool) -> Result<(), CacheError> {
        self.repository.shutdown(save, now).await
    }

    /// List loaded Redis modules.
    pub async fn module_list(&self) -> Result<Vec<ModuleInfo>, CacheError> {
        self.repository.module_list().await
    }

    // ========================================================================
    // Memory Operations
    // ========================================================================

    /// Get memory statistics
    pub async fn get_memory_stats(&self) -> Result<MemoryStats, CacheError> {
        self.repository.get_memory_stats().await
    }

    /// Get memory usage for a key
    pub async fn get_memory_usage(
        &self,
        key: &str,
        samples: Option<u32>,
    ) -> Result<MemoryUsage, CacheError> {
        let samples = samples.unwrap_or(5);
        self.repository.get_memory_usage(key, samples).await
    }

    /// Get memory doctor report
    pub async fn memory_doctor(&self) -> Result<String, CacheError> {
        self.repository.memory_doctor().await
    }

    /// Purge memory
    pub async fn memory_purge(&self) -> Result<(), CacheError> {
        self.repository.memory_purge().await
    }

    // ========================================================================
    // Database Operations
    // ========================================================================

    /// Flush current database
    pub async fn flush_db(&self, async_mode: bool) -> Result<FlushResult, CacheError> {
        let options = FlushOptions { async_mode };
        self.repository.flush_db(options).await
    }

    /// Flush all databases
    pub async fn flush_all(&self, async_mode: bool) -> Result<FlushResult, CacheError> {
        let options = FlushOptions { async_mode };
        self.repository.flush_all(options).await
    }

    /// Copy a key
    pub async fn copy_key(
        &self,
        source: String,
        destination: String,
        db: Option<u8>,
        replace: bool,
    ) -> Result<bool, CacheError> {
        if source.is_empty() {
            return Err(CacheError::InvalidInput(
                "Source key cannot be empty".to_string(),
            ));
        }
        if destination.is_empty() {
            return Err(CacheError::InvalidInput(
                "Destination key cannot be empty".to_string(),
            ));
        }

        let options = CopyKeyOptions {
            source,
            destination,
            db,
            replace,
        };
        self.repository.copy_key(options).await
    }

    /// Move a key to another database
    pub async fn move_key(&self, key: String, db: u8) -> Result<bool, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }

        let options = MoveKeyOptions { key, db };
        self.repository.move_key(options).await
    }

    /// Swap two databases
    pub async fn swap_db(&self, db1: u8, db2: u8) -> Result<(), CacheError> {
        self.repository.swap_db(db1, db2).await
    }

    // ========================================================================
    // Configuration Operations
    // ========================================================================

    /// Get configuration parameters
    pub async fn config_get(&self, pattern: &str) -> Result<HashMap<String, String>, CacheError> {
        if pattern.is_empty() {
            return Err(CacheError::InvalidInput(
                "Pattern cannot be empty".to_string(),
            ));
        }
        self.repository.config_get(pattern).await
    }

    /// Set configuration parameter
    pub async fn config_set(&self, parameter: &str, value: &str) -> Result<(), CacheError> {
        if parameter.is_empty() {
            return Err(CacheError::InvalidInput(
                "Parameter name cannot be empty".to_string(),
            ));
        }
        self.repository.config_set(parameter, value).await
    }

    /// Rewrite configuration file
    pub async fn config_rewrite(&self) -> Result<(), CacheError> {
        self.repository.config_rewrite().await
    }

    /// Reset server statistics
    pub async fn config_resetstat(&self) -> Result<(), CacheError> {
        self.repository.config_resetstat().await
    }

    // ========================================================================
    // Persistence Operations
    // ========================================================================

    /// Synchronous save to disk
    pub async fn save(&self) -> Result<(), CacheError> {
        self.repository.save().await
    }

    /// Asynchronous background save.
    ///
    /// Pass `schedule=true` to use the `SCHEDULE` option (Redis 3.2+), which
    /// enqueues the save when another background persistence task is already
    /// running instead of returning an error.
    pub async fn bgsave(&self, schedule: bool) -> Result<BgSaveResult, CacheError> {
        self.repository.bgsave(schedule).await
    }

    /// Block until `numlocal` fsync acks and `numreplicas` replica fsync acks
    /// are observed, with a millisecond timeout (`WAITAOF`, Redis 7.2+).
    ///
    /// **Connection-scoped, diagnostic semantics.** Redis defines `WAITAOF`
    /// against the writes issued on the *current* connection. Because the
    /// REST service borrows pooled connections, the ack counts reflect
    /// fsync progress on whatever writes that pooled connection happened to
    /// carry — not the caller's prior HTTP requests. See the handler
    /// docstring in `routes/admin.rs::waitaof` for the full caveat.
    pub async fn wait_aof(
        &self,
        numlocal: u64,
        numreplicas: u64,
        timeout_ms: u64,
    ) -> Result<WaitAofResult, CacheError> {
        let timeout_ms = self.enforce_wait_aof_timeout_ms(timeout_ms);
        self.repository
            .wait_aof(numlocal, numreplicas, timeout_ms)
            .await
    }

    /// Background AOF rewrite
    pub async fn bgrewriteaof(&self) -> Result<BgRewriteAofResult, CacheError> {
        self.repository.bgrewriteaof().await
    }

    // ========================================================================
    // Client Operations
    // ========================================================================

    /// Get list of connected clients
    pub async fn client_list(&self) -> Result<Vec<ClientInfo>, CacheError> {
        self.repository.client_list().await
    }

    /// Kill client connections
    pub async fn client_kill(
        &self,
        id: Option<i64>,
        addr: Option<String>,
        client_type: Option<String>,
    ) -> Result<i64, CacheError> {
        let options = ClientKillOptions {
            id,
            addr,
            client_type,
        };
        self.repository.client_kill(options).await
    }

    /// Pause client processing
    pub async fn client_pause(
        &self,
        timeout_ms: u64,
        mode: Option<String>,
    ) -> Result<(), CacheError> {
        let options = ClientPauseOptions {
            timeout_ms,
            mode: mode.unwrap_or_else(|| "write".to_string()),
        };
        self.repository.client_pause(options).await
    }

    /// Unpause client processing
    pub async fn client_unpause(&self) -> Result<(), CacheError> {
        self.repository.client_unpause().await
    }

    /// Set client connection name
    pub async fn client_setname(&self, name: &str) -> Result<(), CacheError> {
        self.repository.client_setname(name).await
    }

    /// Get client connection name
    pub async fn client_getname(&self) -> Result<Option<String>, CacheError> {
        self.repository.client_getname().await
    }

    /// Get client ID
    pub async fn client_id(&self) -> Result<i64, CacheError> {
        self.repository.client_id().await
    }

    /// Get information about the current client
    pub async fn client_info(&self) -> Result<ClientInfo, CacheError> {
        self.repository.client_info().await
    }

    /// Unblock a client blocked on a blocking command (`CLIENT UNBLOCK`,
    /// Redis 5.0+). `client_id` is the target connection's ID (as reported by
    /// CLIENT LIST / CLIENT ID); `error=true` returns `UNBLOCKED` to the
    /// blocked caller instead of the default `TIMEOUT`. Returns 1 if the
    /// client was unblocked, 0 if no such client was found / it wasn't blocked.
    pub async fn client_unblock(&self, client_id: i64, error: bool) -> Result<i64, CacheError> {
        if client_id <= 0 {
            return Err(CacheError::InvalidInput(
                "client_id must be a positive Redis client ID".to_string(),
            ));
        }
        self.repository.client_unblock(client_id, error).await
    }

    // ========================================================================
    // Slowlog Operations
    // ========================================================================

    /// Get slowlog entries
    pub async fn slowlog_get(&self, count: Option<i64>) -> Result<Vec<SlowlogEntry>, CacheError> {
        let count = count.unwrap_or(10);
        self.repository.slowlog_get(count).await
    }

    /// Get slowlog length
    pub async fn slowlog_len(&self) -> Result<i64, CacheError> {
        self.repository.slowlog_len().await
    }

    /// Reset slowlog
    pub async fn slowlog_reset(&self) -> Result<(), CacheError> {
        self.repository.slowlog_reset().await
    }

    // ========================================================================
    // Latency Operations
    // ========================================================================

    /// Get latest latency events
    pub async fn latency_latest(&self) -> Result<Vec<LatencyEvent>, CacheError> {
        self.repository.latency_latest().await
    }

    /// Get latency history for an event
    pub async fn latency_history(&self, event: &str) -> Result<Vec<LatencyEvent>, CacheError> {
        if event.is_empty() {
            return Err(CacheError::InvalidInput(
                "Event name cannot be empty".to_string(),
            ));
        }
        self.repository.latency_history(event).await
    }

    /// Get latency doctor report
    pub async fn latency_doctor(&self) -> Result<String, CacheError> {
        self.repository.latency_doctor().await
    }

    /// Reset latency events
    pub async fn latency_reset(&self, events: Vec<String>) -> Result<(), CacheError> {
        self.repository.latency_reset(&events).await
    }

    /// Get latency graph output for an event
    pub async fn latency_graph(&self, event: &str) -> Result<String, CacheError> {
        if event.is_empty() {
            return Err(CacheError::InvalidInput(
                "Event name cannot be empty".to_string(),
            ));
        }
        self.repository.latency_graph(event).await
    }

    // ========================================================================
    // ACL Operations
    // ========================================================================

    /// List all ACL rules
    pub async fn acl_list(&self) -> Result<Vec<String>, CacheError> {
        self.repository.acl_list().await
    }

    /// List all ACL users
    pub async fn acl_users(&self) -> Result<Vec<String>, CacheError> {
        self.repository.acl_users().await
    }

    /// Get current ACL user
    pub async fn acl_whoami(&self) -> Result<String, CacheError> {
        self.repository.acl_whoami().await
    }

    /// Get ACL categories or commands
    pub async fn acl_cat(&self, category: Option<&str>) -> Result<Vec<String>, CacheError> {
        self.repository.acl_cat(category).await
    }

    /// Generate a secure password
    pub async fn acl_genpass(&self, bits: Option<u32>) -> Result<String, CacheError> {
        let bits = bits.unwrap_or(256);
        self.repository.acl_genpass(bits).await
    }

    /// Get ACL log entries
    pub async fn acl_log(
        &self,
        count: Option<i64>,
        reset: bool,
    ) -> Result<Vec<AclLogEntry>, CacheError> {
        self.repository.acl_log(count, reset).await
    }

    /// Test if a user has permission to run a command (ACL DRYRUN)
    pub async fn acl_dryrun(
        &self,
        username: &str,
        command: &[String],
    ) -> Result<AclDryrunResult, CacheError> {
        if username.is_empty() {
            return Err(CacheError::InvalidInput(
                "Username cannot be empty".to_string(),
            ));
        }
        if command.is_empty() {
            return Err(CacheError::InvalidInput(
                "Command cannot be empty".to_string(),
            ));
        }
        self.repository.acl_dryrun(username, command).await
    }

    /// Set or update an ACL user
    pub async fn acl_setuser(&self, username: &str, rules: &[String]) -> Result<(), CacheError> {
        if username.is_empty() {
            return Err(CacheError::InvalidInput(
                "Username cannot be empty".to_string(),
            ));
        }
        if rules.is_empty() {
            return Err(CacheError::InvalidInput(
                "ACL rules cannot be empty".to_string(),
            ));
        }
        self.repository.acl_setuser(username, rules).await
    }

    /// Delete ACL users
    pub async fn acl_deluser(&self, usernames: &[String]) -> Result<i64, CacheError> {
        if usernames.is_empty() {
            return Err(CacheError::InvalidInput(
                "At least one username is required".to_string(),
            ));
        }
        self.repository.acl_deluser(usernames).await
    }

    /// Reload ACL rules from disk
    pub async fn acl_load(&self) -> Result<(), CacheError> {
        self.repository.acl_load().await
    }

    /// Save ACL rules to disk
    pub async fn acl_save(&self) -> Result<(), CacheError> {
        self.repository.acl_save().await
    }

    // ========================================================================
    // Command Introspection Operations
    // ========================================================================

    /// List all commands, optionally filtered by pattern
    pub async fn command_list(&self, filter: Option<&str>) -> Result<Vec<String>, CacheError> {
        self.repository.command_list(filter).await
    }

    /// Get total number of commands
    pub async fn command_count(&self) -> Result<i64, CacheError> {
        self.repository.command_count().await
    }

    /// Get command documentation
    pub async fn command_docs(&self, commands: &[String]) -> Result<serde_json::Value, CacheError> {
        if commands.is_empty() {
            return Err(CacheError::InvalidInput(
                "At least one command name required".to_string(),
            ));
        }
        self.repository.command_docs(commands).await
    }

    /// Get command info
    pub async fn command_info(&self, commands: &[String]) -> Result<serde_json::Value, CacheError> {
        if commands.is_empty() {
            return Err(CacheError::InvalidInput(
                "At least one command name required".to_string(),
            ));
        }
        self.repository.command_info(commands).await
    }

    /// Extract keys from a command
    pub async fn command_getkeys(&self, command: &[String]) -> Result<Vec<String>, CacheError> {
        if command.is_empty() {
            return Err(CacheError::InvalidInput("Command is required".to_string()));
        }
        self.repository.command_getkeys(command).await
    }

    /// Extract keys + access flags from a command (Redis 7.0+)
    pub async fn command_getkeysandflags(
        &self,
        command: &[String],
    ) -> Result<Vec<KeyAndFlags>, CacheError> {
        if command.is_empty() {
            return Err(CacheError::InvalidInput("Command is required".to_string()));
        }
        self.repository.command_getkeysandflags(command).await
    }

    /// Per-command cumulative latency histogram (Redis 7.0+).
    /// Empty input means "all commands" — Redis accepts no arguments.
    pub async fn latency_histogram(
        &self,
        commands: &[String],
    ) -> Result<serde_json::Value, CacheError> {
        self.repository.latency_histogram(commands).await
    }

    /// jemalloc allocator statistics (Redis 4.0+)
    pub async fn memory_malloc_stats(&self) -> Result<String, CacheError> {
        self.repository.memory_malloc_stats().await
    }

    // ========================================================================
    // Hot Key Monitoring (Redis 8.6+)
    // ========================================================================

    /// Start hot-key tracking. Requires at least one of CPU / NET metrics; the
    /// optional knobs are validated to stay within Redis-accepted ranges so
    /// invalid input is rejected before reaching the server.
    pub async fn hotkeys_start(&self, options: HotkeysStartOptions) -> Result<(), CacheError> {
        if !options.cpu && !options.net {
            return Err(CacheError::InvalidInput(
                "At least one of CPU or NET metrics must be enabled".to_string(),
            ));
        }
        if let Some(k) = options.top_k
            && k == 0
        {
            return Err(CacheError::InvalidInput(
                "top_k must be at least 1".to_string(),
            ));
        }
        if let Some(d) = options.duration_seconds
            && d == 0
        {
            return Err(CacheError::InvalidInput(
                "duration_seconds must be at least 1".to_string(),
            ));
        }
        if let Some(r) = options.sample_ratio
            && !(1..=100).contains(&r)
        {
            return Err(CacheError::InvalidInput(
                "sample_ratio must be between 1 and 100".to_string(),
            ));
        }
        for range in &options.slots {
            if range.start > range.end {
                return Err(CacheError::InvalidInput(format!(
                    "Slot range {}-{} is invalid (start must be ≤ end)",
                    range.start, range.end
                )));
            }
            if range.end > 16_383 {
                return Err(CacheError::InvalidInput(format!(
                    "Slot {} is out of range (max 16383)",
                    range.end
                )));
            }
        }
        self.repository.hotkeys_start(options).await
    }

    /// Stop hot-key tracking (collected data is preserved).
    pub async fn hotkeys_stop(&self) -> Result<(), CacheError> {
        self.repository.hotkeys_stop().await
    }

    /// Fetch the latest tracking report.
    pub async fn hotkeys_get(&self) -> Result<HotkeysReport, CacheError> {
        self.repository.hotkeys_get().await
    }

    /// Release tracking resources.
    pub async fn hotkeys_reset(&self) -> Result<(), CacheError> {
        self.repository.hotkeys_reset().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::redis::connection::InstrumentedPool;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CaptureAdminRepo {
        samples: Mutex<Option<u32>>,
        pause: Mutex<Option<ClientPauseOptions>>,
        slowlog_count: Mutex<Option<i64>>,
        genpass_bits: Mutex<Option<u32>>,
        bgsave_schedule: Mutex<Option<bool>>,
        wait_aof_args: Mutex<Option<(u64, u64, u64)>>,
        client_unblock_args: Mutex<Option<(i64, bool)>>,
    }

    #[async_trait]
    impl AdminRepository for CaptureAdminRepo {
        async fn get_server_info(&self) -> Result<ServerInfo, CacheError> {
            Ok(ServerInfo::default())
        }
        async fn get_server_time(&self) -> Result<ServerTime, CacheError> {
            Ok(ServerTime {
                timestamp: 1,
                microseconds: 2,
            })
        }
        async fn get_db_size(&self) -> Result<i64, CacheError> {
            Ok(0)
        }
        async fn get_last_save(&self) -> Result<i64, CacheError> {
            Ok(0)
        }
        async fn debug_object(&self, key: &str) -> Result<String, CacheError> {
            Ok(format!("debug:{key}"))
        }
        async fn shutdown(&self, _save: bool, _now: bool) -> Result<(), CacheError> {
            Ok(())
        }
        async fn module_list(&self) -> Result<Vec<ModuleInfo>, CacheError> {
            Ok(vec![ModuleInfo {
                name: "mock-module".to_string(),
                version: 1,
                path: String::new(),
                args: vec![],
            }])
        }
        async fn get_memory_stats(&self) -> Result<MemoryStats, CacheError> {
            Ok(MemoryStats::default())
        }
        async fn get_memory_usage(
            &self,
            key: &str,
            samples: u32,
        ) -> Result<MemoryUsage, CacheError> {
            *self.samples.lock().expect("lock") = Some(samples);
            Ok(MemoryUsage {
                key: key.to_string(),
                bytes: Some(1),
            })
        }
        async fn memory_doctor(&self) -> Result<String, CacheError> {
            Ok("OK".to_string())
        }
        async fn memory_purge(&self) -> Result<(), CacheError> {
            Ok(())
        }
        async fn flush_db(&self, options: FlushOptions) -> Result<FlushResult, CacheError> {
            Ok(FlushResult {
                success: true,
                mode: if options.async_mode { "ASYNC" } else { "SYNC" }.to_string(),
            })
        }
        async fn flush_all(&self, options: FlushOptions) -> Result<FlushResult, CacheError> {
            Ok(FlushResult {
                success: true,
                mode: if options.async_mode { "ASYNC" } else { "SYNC" }.to_string(),
            })
        }
        async fn copy_key(&self, _options: CopyKeyOptions) -> Result<bool, CacheError> {
            Ok(true)
        }
        async fn move_key(&self, _options: MoveKeyOptions) -> Result<bool, CacheError> {
            Ok(true)
        }
        async fn swap_db(&self, _db1: u8, _db2: u8) -> Result<(), CacheError> {
            Ok(())
        }
        async fn config_get(&self, _pattern: &str) -> Result<HashMap<String, String>, CacheError> {
            Ok(HashMap::new())
        }
        async fn config_set(&self, _parameter: &str, _value: &str) -> Result<(), CacheError> {
            Ok(())
        }
        async fn config_rewrite(&self) -> Result<(), CacheError> {
            Ok(())
        }
        async fn config_resetstat(&self) -> Result<(), CacheError> {
            Ok(())
        }
        async fn save(&self) -> Result<(), CacheError> {
            Ok(())
        }
        async fn bgsave(&self, schedule: bool) -> Result<BgSaveResult, CacheError> {
            *self.bgsave_schedule.lock().expect("lock") = Some(schedule);
            Ok(BgSaveResult {
                started: true,
                message: if schedule { "SCHEDULED" } else { "OK" }.to_string(),
            })
        }
        async fn bgrewriteaof(&self) -> Result<BgRewriteAofResult, CacheError> {
            Ok(BgRewriteAofResult {
                started: true,
                message: "OK".to_string(),
            })
        }
        async fn client_list(&self) -> Result<Vec<ClientInfo>, CacheError> {
            Ok(vec![])
        }
        async fn client_kill(&self, _options: ClientKillOptions) -> Result<i64, CacheError> {
            Ok(0)
        }
        async fn client_pause(&self, options: ClientPauseOptions) -> Result<(), CacheError> {
            *self.pause.lock().expect("lock") = Some(options);
            Ok(())
        }
        async fn client_unpause(&self) -> Result<(), CacheError> {
            Ok(())
        }
        async fn client_setname(&self, _name: &str) -> Result<(), CacheError> {
            Ok(())
        }
        async fn client_getname(&self) -> Result<Option<String>, CacheError> {
            Ok(None)
        }
        async fn client_id(&self) -> Result<i64, CacheError> {
            Ok(0)
        }
        async fn client_info(&self) -> Result<ClientInfo, CacheError> {
            Ok(ClientInfo::default())
        }
        async fn slowlog_get(&self, count: i64) -> Result<Vec<SlowlogEntry>, CacheError> {
            *self.slowlog_count.lock().expect("lock") = Some(count);
            Ok(vec![])
        }
        async fn slowlog_len(&self) -> Result<i64, CacheError> {
            Ok(0)
        }
        async fn slowlog_reset(&self) -> Result<(), CacheError> {
            Ok(())
        }
        async fn latency_latest(&self) -> Result<Vec<LatencyEvent>, CacheError> {
            Ok(vec![])
        }
        async fn latency_history(&self, event: &str) -> Result<Vec<LatencyEvent>, CacheError> {
            Ok(vec![LatencyEvent {
                event: event.to_string(),
                timestamp: 1,
                latency_ms: 1,
            }])
        }
        async fn latency_doctor(&self) -> Result<String, CacheError> {
            Ok("OK".to_string())
        }
        async fn latency_reset(&self, _events: &[String]) -> Result<(), CacheError> {
            Ok(())
        }
        async fn latency_graph(&self, event: &str) -> Result<String, CacheError> {
            Ok(format!("graph:{event}"))
        }
        async fn acl_list(&self) -> Result<Vec<String>, CacheError> {
            Ok(vec![])
        }
        async fn acl_users(&self) -> Result<Vec<String>, CacheError> {
            Ok(vec![])
        }
        async fn acl_whoami(&self) -> Result<String, CacheError> {
            Ok("default".to_string())
        }
        async fn acl_cat(&self, _category: Option<&str>) -> Result<Vec<String>, CacheError> {
            Ok(vec![])
        }
        async fn acl_genpass(&self, bits: u32) -> Result<String, CacheError> {
            *self.genpass_bits.lock().expect("lock") = Some(bits);
            Ok("pass".to_string())
        }
        async fn acl_log(
            &self,
            _count: Option<i64>,
            _reset: bool,
        ) -> Result<Vec<AclLogEntry>, CacheError> {
            Ok(vec![])
        }
        async fn acl_dryrun(
            &self,
            _username: &str,
            _command: &[String],
        ) -> Result<AclDryrunResult, CacheError> {
            Ok(AclDryrunResult {
                allowed: true,
                reason: None,
            })
        }
        async fn acl_setuser(&self, _username: &str, _rules: &[String]) -> Result<(), CacheError> {
            Ok(())
        }
        async fn acl_deluser(&self, usernames: &[String]) -> Result<i64, CacheError> {
            Ok(usernames.len() as i64)
        }
        async fn acl_load(&self) -> Result<(), CacheError> {
            Ok(())
        }
        async fn acl_save(&self) -> Result<(), CacheError> {
            Ok(())
        }

        async fn command_list(&self, _filter: Option<&str>) -> Result<Vec<String>, CacheError> {
            Ok(vec!["get".to_string(), "set".to_string()])
        }

        async fn command_count(&self) -> Result<i64, CacheError> {
            Ok(200)
        }

        async fn command_docs(
            &self,
            _commands: &[String],
        ) -> Result<serde_json::Value, CacheError> {
            Ok(serde_json::json!({}))
        }

        async fn command_info(
            &self,
            _commands: &[String],
        ) -> Result<serde_json::Value, CacheError> {
            Ok(serde_json::json!([]))
        }

        async fn command_getkeys(&self, _command: &[String]) -> Result<Vec<String>, CacheError> {
            Ok(vec!["key".to_string()])
        }

        async fn command_getkeysandflags(
            &self,
            _command: &[String],
        ) -> Result<Vec<KeyAndFlags>, CacheError> {
            Ok(vec![KeyAndFlags {
                key: "key".to_string(),
                flags: vec!["RO".to_string()],
            }])
        }

        async fn latency_histogram(
            &self,
            _commands: &[String],
        ) -> Result<serde_json::Value, CacheError> {
            Ok(serde_json::json!({}))
        }

        async fn memory_malloc_stats(&self) -> Result<String, CacheError> {
            Ok(String::from("---OK---"))
        }

        async fn hotkeys_start(&self, _options: HotkeysStartOptions) -> Result<(), CacheError> {
            Ok(())
        }
        async fn hotkeys_stop(&self) -> Result<(), CacheError> {
            Ok(())
        }
        async fn hotkeys_get(&self) -> Result<HotkeysReport, CacheError> {
            Ok(HotkeysReport {
                data: serde_json::json!({}),
            })
        }
        async fn hotkeys_reset(&self) -> Result<(), CacheError> {
            Ok(())
        }
        async fn wait_aof(
            &self,
            numlocal: u64,
            numreplicas: u64,
            timeout_ms: u64,
        ) -> Result<WaitAofResult, CacheError> {
            *self.wait_aof_args.lock().expect("lock") = Some((numlocal, numreplicas, timeout_ms));
            Ok(WaitAofResult {
                local: numlocal as i64,
                replicas: numreplicas as i64,
            })
        }
        async fn client_unblock(&self, client_id: i64, error: bool) -> Result<i64, CacheError> {
            *self.client_unblock_args.lock().expect("lock") = Some((client_id, error));
            Ok(1)
        }
    }

    #[tokio::test]
    async fn test_admin_service_validation() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);

        let err = service
            .copy_key("".to_string(), "dest".to_string(), None, false)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .copy_key("src".to_string(), "".to_string(), None, false)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.move_key("".to_string(), 1).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.debug_object("").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.config_get("").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.config_set("", "v").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.latency_history("").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.latency_graph("").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .acl_dryrun("", &["GET".to_string()])
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.acl_dryrun("default", &[]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let rules = vec!["on".to_string()];
        let err = service.acl_setuser("", &rules).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.acl_setuser("default", &[]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.acl_deluser(&[]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_admin_service_defaults() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo.clone());

        service
            .get_memory_usage("k", None)
            .await
            .expect("memory usage");
        assert_eq!(*repo.samples.lock().expect("lock"), Some(5));

        service.client_pause(100, None).await.expect("pause");
        assert_eq!(
            repo.pause.lock().expect("lock").as_ref().unwrap().mode,
            "write"
        );

        service.slowlog_get(None).await.expect("slowlog");
        assert_eq!(*repo.slowlog_count.lock().expect("lock"), Some(10));

        service.acl_genpass(None).await.expect("genpass");
        assert_eq!(*repo.genpass_bits.lock().expect("lock"), Some(256));
    }

    #[tokio::test]
    async fn test_module_list_passthrough() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);

        let modules = service.module_list().await.expect("module list");
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].name, "mock-module");
    }

    #[tokio::test]
    async fn test_command_introspection_validation() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);

        // command_docs with empty commands
        let err = service.command_docs(&[]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // command_info with empty commands
        let err = service.command_info(&[]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // command_getkeysandflags with empty command
        let err = service.command_getkeysandflags(&[]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // command_getkeys with empty command
        let err = service.command_getkeys(&[]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_command_introspection_passthrough() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);

        let list = service.command_list(None).await.expect("command_list");
        assert_eq!(list, vec!["get".to_string(), "set".to_string()]);

        let list_filtered = service
            .command_list(Some("get*"))
            .await
            .expect("command_list filtered");
        assert_eq!(list_filtered, vec!["get".to_string(), "set".to_string()]);

        let count = service.command_count().await.expect("command_count");
        assert_eq!(count, 200);

        let docs = service
            .command_docs(&["GET".to_string()])
            .await
            .expect("command_docs");
        assert!(docs.is_object());

        let info = service
            .command_info(&["GET".to_string()])
            .await
            .expect("command_info");
        assert!(info.is_array());

        let keys = service
            .command_getkeys(&["GET".to_string(), "mykey".to_string()])
            .await
            .expect("command_getkeys");
        assert_eq!(keys, vec!["key".to_string()]);
    }

    #[test]
    fn test_admin_service_new() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = AdminService::new(pool);
    }

    #[tokio::test]
    async fn test_hotkeys_start_rejects_no_metrics() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);
        let err = service
            .hotkeys_start(HotkeysStartOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hotkeys_start_rejects_zero_top_k() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);
        let err = service
            .hotkeys_start(HotkeysStartOptions {
                cpu: true,
                top_k: Some(0),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hotkeys_start_rejects_zero_duration() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);
        let err = service
            .hotkeys_start(HotkeysStartOptions {
                cpu: true,
                duration_seconds: Some(0),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hotkeys_start_rejects_sample_ratio_out_of_range() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);
        let err = service
            .hotkeys_start(HotkeysStartOptions {
                cpu: true,
                sample_ratio: Some(150),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hotkeys_start_rejects_out_of_range_slot() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);
        let err = service
            .hotkeys_start(HotkeysStartOptions {
                cpu: true,
                slots: vec![crate::domain::entities::HotkeysSlotRange {
                    start: 16_000,
                    end: 17_000,
                }],
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_hotkeys_start_rejects_inverted_slot_range() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);
        let err = service
            .hotkeys_start(HotkeysStartOptions {
                cpu: true,
                slots: vec![crate::domain::entities::HotkeysSlotRange {
                    start: 200,
                    end: 100,
                }],
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_bgsave_forwards_schedule_flag() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo.clone());

        let plain = service.bgsave(false).await.expect("bgsave");
        assert!(plain.started);
        assert_eq!(plain.message, "OK");
        assert_eq!(*repo.bgsave_schedule.lock().expect("lock"), Some(false));

        let scheduled = service.bgsave(true).await.expect("bgsave schedule");
        assert!(scheduled.started);
        assert_eq!(scheduled.message, "SCHEDULED");
        assert_eq!(*repo.bgsave_schedule.lock().expect("lock"), Some(true));
    }

    #[tokio::test]
    async fn test_wait_aof_forwards_arguments() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo.clone());

        let result = service.wait_aof(2, 3, 1_500).await.expect("wait_aof");
        assert_eq!(result.local, 2);
        assert_eq!(result.replicas, 3);
        assert_eq!(
            *repo.wait_aof_args.lock().expect("lock"),
            Some((2, 3, 1_500))
        );
    }

    #[tokio::test]
    async fn test_wait_aof_clamps_timeout_to_blocking_bounds() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo.clone())
            .with_max_blocking_timeout(Duration::from_secs(2));

        service.wait_aof(1, 0, 0).await.expect("wait_aof min");
        assert_eq!(
            *repo.wait_aof_args.lock().expect("lock"),
            Some((1, 0, 1_000))
        );

        service.wait_aof(1, 0, 60_000).await.expect("wait_aof max");
        assert_eq!(
            *repo.wait_aof_args.lock().expect("lock"),
            Some((1, 0, 2_000))
        );
    }

    #[tokio::test]
    async fn test_client_unblock_rejects_non_positive_id() {
        let service = AdminService::new_with_repository(Arc::new(CaptureAdminRepo::default()));
        let err = service.client_unblock(0, false).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
        let err = service.client_unblock(-5, true).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_client_unblock_forwards_arguments() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo.clone());
        let reply = service.client_unblock(101, true).await.expect("unblock");
        assert_eq!(reply, 1);
        assert_eq!(
            *repo.client_unblock_args.lock().expect("lock"),
            Some((101, true))
        );
    }

    #[tokio::test]
    async fn test_hotkeys_lifecycle_passthrough() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);

        service
            .hotkeys_start(HotkeysStartOptions {
                cpu: true,
                net: true,
                top_k: Some(5),
                duration_seconds: Some(30),
                sample_ratio: Some(10),
                slots: vec![],
            })
            .await
            .expect("start");
        service.hotkeys_stop().await.expect("stop");
        let report = service.hotkeys_get().await.expect("get");
        assert!(report.data.is_object());
        service.hotkeys_reset().await.expect("reset");
    }
}
