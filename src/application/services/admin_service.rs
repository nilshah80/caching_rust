//! Admin Service
//!
//! Business logic layer for admin operations.

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::entities::{
    AclDryrunResult, AclLogEntry, BgRewriteAofResult, BgSaveResult, ClientInfo, ClientKillOptions,
    ClientPauseOptions, CopyKeyOptions, FlushOptions, FlushResult, LatencyEvent, MemoryStats,
    MemoryUsage, MoveKeyOptions, ServerInfo, ServerTime, SlowlogEntry,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::AdminRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisAdminRepository;

/// Service for admin operations
pub struct AdminService {
    repository: Arc<dyn AdminRepository>,
}

impl AdminService {
    /// Create a new AdminService
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisAdminRepository::new(pool)))
    }

    /// Create an AdminService with a custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn AdminRepository>) -> Self {
        Self { repository }
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
        async fn bgsave(&self) -> Result<BgSaveResult, CacheError> {
            Ok(BgSaveResult {
                started: true,
                message: "OK".to_string(),
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

        let err = service.config_get("").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.config_set("", "v").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.latency_history("").await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .acl_dryrun("", &["GET".to_string()])
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.acl_dryrun("default", &[]).await.unwrap_err();
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
    async fn test_command_introspection_validation() {
        let repo = Arc::new(CaptureAdminRepo::default());
        let service = AdminService::new_with_repository(repo);

        // command_docs with empty commands
        let err = service.command_docs(&[]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // command_info with empty commands
        let err = service.command_info(&[]).await.unwrap_err();
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
}
