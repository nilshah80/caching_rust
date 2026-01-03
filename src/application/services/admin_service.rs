//! Admin Service
//!
//! Business logic layer for admin operations.

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::entities::{
    AclLogEntry, BgRewriteAofResult, BgSaveResult, ClientInfo, ClientKillOptions,
    ClientPauseOptions, CopyKeyOptions, FlushOptions, FlushResult, LatencyEvent,
    MemoryStats, MemoryUsage, MoveKeyOptions, ServerInfo, ServerTime, SlowlogEntry,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::AdminRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisAdminRepository;

/// Service for admin operations
pub struct AdminService {
    repository: RedisAdminRepository,
}

impl AdminService {
    /// Create a new AdminService
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self {
            repository: RedisAdminRepository::new(pool),
        }
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

    // ========================================================================
    // Memory Operations
    // ========================================================================

    /// Get memory statistics
    pub async fn get_memory_stats(&self) -> Result<MemoryStats, CacheError> {
        self.repository.get_memory_stats().await
    }

    /// Get memory usage for a key
    pub async fn get_memory_usage(&self, key: &str, samples: Option<u32>) -> Result<MemoryUsage, CacheError> {
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
            return Err(CacheError::InvalidInput("Source key cannot be empty".to_string()));
        }
        if destination.is_empty() {
            return Err(CacheError::InvalidInput("Destination key cannot be empty".to_string()));
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
            return Err(CacheError::InvalidInput("Pattern cannot be empty".to_string()));
        }
        self.repository.config_get(pattern).await
    }

    /// Set configuration parameter
    pub async fn config_set(&self, parameter: &str, value: &str) -> Result<(), CacheError> {
        if parameter.is_empty() {
            return Err(CacheError::InvalidInput("Parameter name cannot be empty".to_string()));
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

    /// Asynchronous background save
    pub async fn bgsave(&self) -> Result<BgSaveResult, CacheError> {
        self.repository.bgsave().await
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
    pub async fn client_pause(&self, timeout_ms: u64, mode: Option<String>) -> Result<(), CacheError> {
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
            return Err(CacheError::InvalidInput("Event name cannot be empty".to_string()));
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
    pub async fn acl_log(&self, count: Option<i64>, reset: bool) -> Result<Vec<AclLogEntry>, CacheError> {
        self.repository.acl_log(count, reset).await
    }
}
