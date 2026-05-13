//! Admin Repository Trait
//!
//! Abstract interface for admin operations.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::domain::entities::{
    AclDryrunResult, AclLogEntry, BgRewriteAofResult, BgSaveResult, ClientInfo, ClientKillOptions,
    ClientPauseOptions, CopyKeyOptions, FlushOptions, FlushResult, HotkeysReport,
    HotkeysStartOptions, KeyAndFlags, LatencyEvent, MemoryStats, MemoryUsage, MoveKeyOptions,
    ServerInfo, ServerTime, SlowlogEntry, WaitAofResult,
};
use crate::domain::errors::CacheError;

/// Repository trait for Redis admin operations
#[async_trait]
pub trait AdminRepository: Send + Sync {
    // ========================================================================
    // Server Operations
    // ========================================================================

    /// Get server information (INFO command)
    async fn get_server_info(&self) -> Result<ServerInfo, CacheError>;

    /// Get server time (TIME command)
    async fn get_server_time(&self) -> Result<ServerTime, CacheError>;

    /// Get database size (DBSIZE command)
    async fn get_db_size(&self) -> Result<i64, CacheError>;

    /// Get last save timestamp (LASTSAVE command)
    async fn get_last_save(&self) -> Result<i64, CacheError>;

    /// Get limited debug information for a key (DEBUG OBJECT command)
    async fn debug_object(&self, key: &str) -> Result<String, CacheError>;

    /// Shutdown the Redis server (SHUTDOWN command)
    async fn shutdown(&self, save: bool, now: bool) -> Result<(), CacheError>;

    // ========================================================================
    // Memory Operations
    // ========================================================================

    /// Get memory statistics (MEMORY STATS command)
    async fn get_memory_stats(&self) -> Result<MemoryStats, CacheError>;

    /// Get memory usage for a key (MEMORY USAGE command)
    async fn get_memory_usage(&self, key: &str, samples: u32) -> Result<MemoryUsage, CacheError>;

    /// Get memory doctor report (MEMORY DOCTOR command)
    async fn memory_doctor(&self) -> Result<String, CacheError>;

    /// Purge memory (MEMORY PURGE command)
    async fn memory_purge(&self) -> Result<(), CacheError>;

    // ========================================================================
    // Database Operations
    // ========================================================================

    /// Flush current database (FLUSHDB command)
    async fn flush_db(&self, options: FlushOptions) -> Result<FlushResult, CacheError>;

    /// Flush all databases (FLUSHALL command)
    async fn flush_all(&self, options: FlushOptions) -> Result<FlushResult, CacheError>;

    /// Copy a key (COPY command)
    async fn copy_key(&self, options: CopyKeyOptions) -> Result<bool, CacheError>;

    /// Move a key to another database (MOVE command)
    async fn move_key(&self, options: MoveKeyOptions) -> Result<bool, CacheError>;

    /// Swap two databases (SWAPDB command)
    async fn swap_db(&self, db1: u8, db2: u8) -> Result<(), CacheError>;

    // ========================================================================
    // Configuration Operations
    // ========================================================================

    /// Get configuration parameters (CONFIG GET command)
    async fn config_get(&self, pattern: &str) -> Result<HashMap<String, String>, CacheError>;

    /// Set configuration parameter (CONFIG SET command)
    async fn config_set(&self, parameter: &str, value: &str) -> Result<(), CacheError>;

    /// Rewrite configuration file (CONFIG REWRITE command)
    async fn config_rewrite(&self) -> Result<(), CacheError>;

    /// Reset server statistics (CONFIG RESETSTAT command)
    async fn config_resetstat(&self) -> Result<(), CacheError>;

    // ========================================================================
    // Persistence Operations
    // ========================================================================

    /// Synchronous save to disk (SAVE command)
    async fn save(&self) -> Result<(), CacheError>;

    /// Asynchronous background save (BGSAVE command).
    ///
    /// When `schedule` is true, Redis enqueues the save instead of refusing
    /// it if another `BGSAVE`/`BGREWRITEAOF` is already running (Redis 3.2+).
    async fn bgsave(&self, schedule: bool) -> Result<BgSaveResult, CacheError>;

    /// Background AOF rewrite (BGREWRITEAOF command)
    async fn bgrewriteaof(&self) -> Result<BgRewriteAofResult, CacheError>;

    // ========================================================================
    // Client Operations
    // ========================================================================

    /// Get list of connected clients (CLIENT LIST command)
    async fn client_list(&self) -> Result<Vec<ClientInfo>, CacheError>;

    /// Kill client connections (CLIENT KILL command)
    async fn client_kill(&self, options: ClientKillOptions) -> Result<i64, CacheError>;

    /// Pause client processing (CLIENT PAUSE command)
    async fn client_pause(&self, options: ClientPauseOptions) -> Result<(), CacheError>;

    /// Unpause client processing (CLIENT UNPAUSE command)
    async fn client_unpause(&self) -> Result<(), CacheError>;

    /// Set client connection name (CLIENT SETNAME command)
    async fn client_setname(&self, name: &str) -> Result<(), CacheError>;

    /// Get client connection name (CLIENT GETNAME command)
    async fn client_getname(&self) -> Result<Option<String>, CacheError>;

    /// Get client ID (CLIENT ID command)
    async fn client_id(&self) -> Result<i64, CacheError>;

    /// Get information about the current client (CLIENT INFO command)
    async fn client_info(&self) -> Result<ClientInfo, CacheError>;

    // ========================================================================
    // Slowlog Operations
    // ========================================================================

    /// Get slowlog entries (SLOWLOG GET command)
    async fn slowlog_get(&self, count: i64) -> Result<Vec<SlowlogEntry>, CacheError>;

    /// Get slowlog length (SLOWLOG LEN command)
    async fn slowlog_len(&self) -> Result<i64, CacheError>;

    /// Reset slowlog (SLOWLOG RESET command)
    async fn slowlog_reset(&self) -> Result<(), CacheError>;

    // ========================================================================
    // Latency Operations
    // ========================================================================

    /// Get latest latency events (LATENCY LATEST command)
    async fn latency_latest(&self) -> Result<Vec<LatencyEvent>, CacheError>;

    /// Get latency history for an event (LATENCY HISTORY command)
    async fn latency_history(&self, event: &str) -> Result<Vec<LatencyEvent>, CacheError>;

    /// Get latency doctor report (LATENCY DOCTOR command)
    async fn latency_doctor(&self) -> Result<String, CacheError>;

    /// Reset latency events (LATENCY RESET command)
    async fn latency_reset(&self, events: &[String]) -> Result<(), CacheError>;

    /// Get latency graph output for an event (LATENCY GRAPH command)
    async fn latency_graph(&self, event: &str) -> Result<String, CacheError>;

    // ========================================================================
    // ACL Operations
    // ========================================================================

    /// List all ACL rules (ACL LIST command)
    async fn acl_list(&self) -> Result<Vec<String>, CacheError>;

    /// List all ACL users (ACL USERS command)
    async fn acl_users(&self) -> Result<Vec<String>, CacheError>;

    /// Get current ACL user (ACL WHOAMI command)
    async fn acl_whoami(&self) -> Result<String, CacheError>;

    /// Get ACL categories or commands (ACL CAT command)
    async fn acl_cat(&self, category: Option<&str>) -> Result<Vec<String>, CacheError>;

    /// Generate a secure password (ACL GENPASS command)
    async fn acl_genpass(&self, bits: u32) -> Result<String, CacheError>;

    /// Get ACL log entries (ACL LOG command)
    async fn acl_log(
        &self,
        count: Option<i64>,
        reset: bool,
    ) -> Result<Vec<AclLogEntry>, CacheError>;

    /// Test if a user has permission to run a command (ACL DRYRUN command, Redis 7.0+)
    async fn acl_dryrun(
        &self,
        username: &str,
        command: &[String],
    ) -> Result<AclDryrunResult, CacheError>;

    /// Set or update an ACL user (ACL SETUSER command)
    async fn acl_setuser(&self, username: &str, rules: &[String]) -> Result<(), CacheError>;

    /// Delete ACL users (ACL DELUSER command)
    async fn acl_deluser(&self, usernames: &[String]) -> Result<i64, CacheError>;

    /// Reload ACL rules from disk (ACL LOAD command)
    async fn acl_load(&self) -> Result<(), CacheError>;

    /// Save ACL rules to disk (ACL SAVE command)
    async fn acl_save(&self) -> Result<(), CacheError>;

    // ========================================================================
    // Command Introspection Operations
    // ========================================================================

    /// List all commands, optionally filtered by pattern (COMMAND LIST command, Redis 7.0+)
    async fn command_list(&self, filter: Option<&str>) -> Result<Vec<String>, CacheError>;

    /// Get total number of commands (COMMAND COUNT command, Redis 2.8.13+)
    async fn command_count(&self) -> Result<i64, CacheError>;

    /// Get command documentation (COMMAND DOCS command, Redis 7.0+)
    async fn command_docs(&self, commands: &[String]) -> Result<serde_json::Value, CacheError>;

    /// Get command info (COMMAND INFO command, Redis 2.8.13+)
    async fn command_info(&self, commands: &[String]) -> Result<serde_json::Value, CacheError>;

    /// Extract keys from a command (COMMAND GETKEYS command, Redis 2.8.13+)
    async fn command_getkeys(&self, command: &[String]) -> Result<Vec<String>, CacheError>;

    /// Extract keys + per-key access flags (COMMAND GETKEYSANDFLAGS, Redis 7.0+)
    async fn command_getkeysandflags(
        &self,
        command: &[String],
    ) -> Result<Vec<KeyAndFlags>, CacheError>;

    // ========================================================================
    // Latency / Memory introspection (10.7)
    // ========================================================================

    /// Per-command cumulative latency histogram (LATENCY HISTOGRAM, Redis 7.0+).
    ///
    /// Pass an empty slice to retrieve the full set. The reply structure is
    /// nested and open-ended, so the trait returns the parsed JSON verbatim
    /// (matching the precedent set by `command_docs` / `command_info`).
    async fn latency_histogram(&self, commands: &[String])
    -> Result<serde_json::Value, CacheError>;

    /// Allocator statistics report (MEMORY MALLOC-STATS, Redis 4.0+).
    ///
    /// Returns a long bulk string when running under jemalloc, an empty/benign
    /// payload otherwise. Mirrors `memory_doctor`.
    async fn memory_malloc_stats(&self) -> Result<String, CacheError>;

    // ========================================================================
    // Hot Key Monitoring (Redis 8.6+)
    // ========================================================================

    /// Start hot-key tracking (HOTKEYS START).
    async fn hotkeys_start(&self, options: HotkeysStartOptions) -> Result<(), CacheError>;

    /// Stop hot-key tracking but keep the collected data (HOTKEYS STOP).
    async fn hotkeys_stop(&self) -> Result<(), CacheError>;

    /// Fetch the current or most recent tracking report (HOTKEYS GET).
    async fn hotkeys_get(&self) -> Result<HotkeysReport, CacheError>;

    /// Release tracking resources; can only run when tracking is stopped (HOTKEYS RESET).
    async fn hotkeys_reset(&self) -> Result<(), CacheError>;

    // ========================================================================
    // Durability (Redis 7.2+)
    // ========================================================================

    /// Block until `numlocal` fsync acks and `numreplicas` replica fsync acks
    /// are observed, with a millisecond `timeout` (`WAITAOF numlocal numreplicas timeout`).
    ///
    /// **Connection-scoped:** Redis evaluates this command against the writes
    /// issued on the *current connection*. Implementations that run on top of
    /// a shared connection pool therefore observe the ack state of whichever
    /// pooled connection is borrowed — not writes a caller made through an
    /// earlier HTTP request. Callers requiring a per-request durability
    /// guarantee must pin the connection at write time.
    async fn wait_aof(
        &self,
        numlocal: u64,
        numreplicas: u64,
        timeout_ms: u64,
    ) -> Result<WaitAofResult, CacheError>;

    /// Unblock a client that is blocked on a blocking command, by ID
    /// (`CLIENT UNBLOCK client-id [TIMEOUT | ERROR]`, Redis 5.0+).
    ///
    /// Returns `1` if the client was unblocked, `0` if no such client was
    /// found or the client wasn't blocked. `error` controls whether the
    /// blocked command returns a normal timeout (`TIMEOUT`, default) or an
    /// `UNBLOCKED` error (`ERROR`).
    async fn client_unblock(&self, client_id: i64, error: bool) -> Result<i64, CacheError>;
}
