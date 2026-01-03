//! Admin Repository Trait
//!
//! Abstract interface for admin operations.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::domain::entities::{
    AclLogEntry, BgRewriteAofResult, BgSaveResult, ClientInfo, ClientKillOptions,
    ClientPauseOptions, CopyKeyOptions, FlushOptions, FlushResult, LatencyEvent,
    MemoryStats, MemoryUsage, MoveKeyOptions, ServerInfo, ServerTime, SlowlogEntry,
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

    /// Asynchronous background save (BGSAVE command)
    async fn bgsave(&self) -> Result<BgSaveResult, CacheError>;

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
    async fn acl_log(&self, count: Option<i64>, reset: bool) -> Result<Vec<AclLogEntry>, CacheError>;
}
