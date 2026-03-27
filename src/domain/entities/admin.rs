//! Admin Domain Entities
//!
//! Core business objects for admin operations.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Server Information
// ============================================================================

/// Server information from Redis INFO command
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ServerInfo {
    pub redis_version: String,
    pub uptime_seconds: i64,
    pub connected_clients: i64,
    pub used_memory: i64,
    pub used_memory_human: String,
    pub total_system_memory: i64,
    pub used_memory_peak: i64,
    pub total_keys: i64,
    pub expired_keys: i64,
    pub keyspace_hits: i64,
    pub keyspace_misses: i64,
}

/// Server time response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerTime {
    pub timestamp: i64,
    pub microseconds: i64,
}

// ============================================================================
// Memory Information
// ============================================================================

/// Memory statistics from MEMORY STATS
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MemoryStats {
    pub peak_allocated: i64,
    pub total_allocated: i64,
    pub startup_allocated: i64,
    pub replication_backlog: i64,
    pub clients_normal: i64,
    pub clients_slaves: i64,
    pub aof_buffer: i64,
    pub lua_caches: i64,
    pub overhead_total: i64,
    pub dataset_bytes: i64,
    pub dataset_perc: f64,
    pub peak_perc: f64,
    pub fragmentation: f64,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            peak_allocated: 0,
            total_allocated: 0,
            startup_allocated: 0,
            replication_backlog: 0,
            clients_normal: 0,
            clients_slaves: 0,
            aof_buffer: 0,
            lua_caches: 0,
            overhead_total: 0,
            dataset_bytes: 0,
            dataset_perc: 0.0,
            peak_perc: 0.0,
            fragmentation: 0.0,
        }
    }
}

/// Memory usage for a specific key
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MemoryUsage {
    pub key: String,
    pub bytes: Option<i64>,
}

// ============================================================================
// Database Operations
// ============================================================================

/// Copy key options
#[derive(Debug, Clone, Default)]
pub struct CopyKeyOptions {
    pub source: String,
    pub destination: String,
    pub db: Option<u8>,
    pub replace: bool,
}

/// Move key options
#[derive(Debug, Clone)]
pub struct MoveKeyOptions {
    pub key: String,
    pub db: u8,
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration parameter
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConfigParameter {
    pub name: String,
    pub value: String,
}

// ============================================================================
// Persistence
// ============================================================================

/// Background save result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BgSaveResult {
    pub started: bool,
    pub message: String,
}

/// AOF rewrite result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BgRewriteAofResult {
    pub started: bool,
    pub message: String,
}

// ============================================================================
// Client Information
// ============================================================================

/// Client connection information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClientInfo {
    pub id: i64,
    pub name: String,
    pub addr: String,
    pub fd: i64,
    pub age: i64,
    pub idle: i64,
    pub flags: String,
    pub db: i64,
    pub multi: i64,
    pub qbuf: i64,
    pub qbuf_free: i64,
    pub obl: i64,
    pub oll: i64,
    pub omem: i64,
    pub cmd: String,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            addr: String::new(),
            fd: 0,
            age: 0,
            idle: 0,
            flags: String::new(),
            db: 0,
            multi: -1,
            qbuf: 0,
            qbuf_free: 0,
            obl: 0,
            oll: 0,
            omem: 0,
            cmd: String::new(),
        }
    }
}

/// Client kill options
#[derive(Debug, Clone, Default)]
pub struct ClientKillOptions {
    pub id: Option<i64>,
    pub addr: Option<String>,
    pub client_type: Option<String>,
}

/// Client pause options
#[derive(Debug, Clone)]
pub struct ClientPauseOptions {
    pub timeout_ms: u64,
    pub mode: String,
}

impl Default for ClientPauseOptions {
    fn default() -> Self {
        Self {
            timeout_ms: 0,
            mode: "write".to_string(),
        }
    }
}

// ============================================================================
// Slowlog
// ============================================================================

/// Slowlog entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SlowlogEntry {
    pub id: i64,
    pub timestamp: i64,
    pub duration_us: i64,
    pub command: Vec<String>,
    pub client_addr: String,
    pub client_name: String,
}

// ============================================================================
// Latency
// ============================================================================

/// Latency event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LatencyEvent {
    pub event: String,
    pub timestamp: i64,
    pub latency_ms: i64,
}

// ============================================================================
// ACL
// ============================================================================

/// ACL log entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AclLogEntry {
    pub count: i64,
    pub reason: String,
    pub context: String,
    pub object: String,
    pub username: String,
    pub age_seconds: f64,
    pub client_info: String,
    pub entry_timestamp: i64,
    pub timestamp_us: i64,
}

impl Default for AclLogEntry {
    fn default() -> Self {
        Self {
            count: 1,
            reason: String::new(),
            context: String::new(),
            object: String::new(),
            username: String::new(),
            age_seconds: 0.0,
            client_info: String::new(),
            entry_timestamp: 0,
            timestamp_us: 0,
        }
    }
}

/// Result of ACL DRYRUN command
#[derive(Debug, Clone)]
pub struct AclDryrunResult {
    /// Whether the command would be allowed
    pub allowed: bool,
    /// Reason for denial, if not allowed
    pub reason: Option<String>,
}

/// Flush options
#[derive(Debug, Clone, Default)]
pub struct FlushOptions {
    pub async_mode: bool,
}

/// Flush result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlushResult {
    pub success: bool,
    pub mode: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_info_default() {
        let info = ServerInfo::default();
        assert_eq!(info.redis_version, "");
        assert_eq!(info.total_keys, 0);
    }

    #[test]
    fn test_memory_stats_default() {
        let stats = MemoryStats::default();
        assert_eq!(stats.peak_allocated, 0);
        assert_eq!(stats.fragmentation, 0.0);
    }

    #[test]
    fn test_client_info_default() {
        let info = ClientInfo::default();
        assert_eq!(info.id, 0);
        assert_eq!(info.multi, -1);
    }

    #[test]
    fn test_client_pause_default() {
        let pause = ClientPauseOptions::default();
        assert_eq!(pause.mode, "write");
        assert_eq!(pause.timeout_ms, 0);
    }

    #[test]
    fn test_acl_log_entry_default() {
        let entry = AclLogEntry::default();
        assert_eq!(entry.count, 1);
        assert_eq!(entry.age_seconds, 0.0);
    }
}
