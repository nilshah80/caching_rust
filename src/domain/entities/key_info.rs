//! Key Entity
//!
//! Domain entities representing Redis key operations and their results.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Information about a Redis key
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KeyInfo {
    /// The key name
    pub key: String,

    /// Redis data type (string, list, set, zset, hash, stream)
    #[serde(rename = "type")]
    pub key_type: String,

    /// Time-to-live in seconds (-1 if no expiry, -2 if key doesn't exist)
    pub ttl: i64,

    /// Time-to-live in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pttl: Option<i64>,

    /// Whether the key exists
    pub exists: bool,

    /// Memory usage in bytes (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_usage: Option<i64>,

    /// Internal encoding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,

    /// Idle time in seconds (time since last access)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_time: Option<u64>,

    /// Reference count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_count: Option<u64>,
}

impl KeyInfo {
    /// Create a new KeyInfo for a non-existent key
    pub fn not_found(key: String) -> Self {
        Self {
            key,
            key_type: "none".to_string(),
            ttl: -2,
            pttl: None,
            exists: false,
            memory_usage: None,
            encoding: None,
            idle_time: None,
            ref_count: None,
        }
    }

    /// Create a new KeyInfo for an existing key
    pub fn new(key: String, key_type: String, ttl: i64) -> Self {
        Self {
            key,
            key_type,
            ttl,
            pttl: None,
            exists: true,
            memory_usage: None,
            encoding: None,
            idle_time: None,
            ref_count: None,
        }
    }

    /// Add PTTL
    pub fn with_pttl(mut self, pttl: i64) -> Self {
        self.pttl = Some(pttl);
        self
    }

    /// Add memory usage
    pub fn with_memory_usage(mut self, bytes: i64) -> Self {
        self.memory_usage = Some(bytes);
        self
    }

    /// Add encoding
    pub fn with_encoding(mut self, encoding: String) -> Self {
        self.encoding = Some(encoding);
        self
    }

    /// Add idle time
    pub fn with_idle_time(mut self, seconds: u64) -> Self {
        self.idle_time = Some(seconds);
        self
    }

    /// Add reference count
    pub fn with_ref_count(mut self, count: u64) -> Self {
        self.ref_count = Some(count);
        self
    }
}

/// Result of EXISTS operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExistsResult {
    /// Keys that exist
    pub existing: Vec<String>,

    /// Keys that don't exist
    pub missing: Vec<String>,

    /// Count of existing keys
    pub count: usize,
}

/// Result of DELETE operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeleteResult {
    /// Keys that were deleted
    pub deleted: Vec<String>,

    /// Keys that didn't exist
    pub not_found: Vec<String>,

    /// Count of deleted keys
    pub count: usize,
}

/// Result of EXPIRE operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExpireResult {
    /// The key
    pub key: String,

    /// Whether the TTL was set
    pub success: bool,

    /// The new TTL in seconds (if queried)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_ttl: Option<i64>,
}

/// Result of RENAME operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RenameResult {
    /// Original key name
    pub old_key: String,

    /// New key name
    pub new_key: String,

    /// Whether the operation succeeded
    pub success: bool,
}

/// Result of SCAN operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScanResult {
    /// Cursor for next iteration (0 means complete)
    pub cursor: u64,

    /// Keys found in this iteration
    pub keys: Vec<String>,

    /// Count of keys returned
    pub count: usize,
}

/// Result of TOUCH operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TouchResult {
    /// Number of keys touched
    pub count: usize,
}

/// Result of PERSIST operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PersistResult {
    /// The key
    pub key: String,

    /// Whether TTL was removed
    pub success: bool,
}

/// Result of RANDOMKEY operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RandomKeyResult {
    /// The random key (None if database is empty)
    pub key: Option<String>,
}

/// Options for EXPIRE command
#[derive(Debug, Clone, Default)]
pub struct ExpireOptions {
    /// Only set expiry if key has no expiry (NX)
    pub nx: bool,

    /// Only set expiry if key already has an expiry (XX)
    pub xx: bool,

    /// Only set expiry if new expiry > current (GT)
    pub gt: bool,

    /// Only set expiry if new expiry < current (LT)
    pub lt: bool,
}

/// Options for COPY command
#[derive(Debug, Clone, Default)]
pub struct CopyOptions {
    /// Target database index
    pub db: Option<i64>,

    /// Replace existing key
    pub replace: bool,
}

/// Result of COPY operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CopyResult {
    /// Source key
    pub source: String,

    /// Destination key
    pub destination: String,

    /// Whether the copy succeeded
    pub success: bool,
}

/// Result of DUMP operation (serialized key)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DumpResult {
    /// The key
    pub key: String,

    /// Serialized value (base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Options forwarded to `RESTORE key ttl value [REPLACE] [ABSTTL] [IDLETIME s] [FREQ f]`.
///
/// `ttl` is in milliseconds and is interpreted as an absolute Unix-ms timestamp
/// when [`Self::absttl`] is true (Redis 5.0+). `idletime` (seconds) and `freq`
/// initialize the restored key's OBJECT metadata (Redis 5.0+).
#[derive(Debug, Clone, Copy, Default)]
pub struct RestoreOptions {
    pub ttl: i64,
    pub replace: bool,
    pub absttl: bool,
    pub idletime: Option<u64>,
    pub freq: Option<u8>,
}

/// Result of OBJECT operations
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ObjectInfoResult {
    /// The key
    pub key: String,

    /// Encoding type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,

    /// Reference count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_count: Option<u64>,

    /// Idle time in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_time: Option<u64>,

    /// Frequency counter (for LFU eviction)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freq: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_info_not_found() {
        let info = KeyInfo::not_found("test".to_string());
        assert_eq!(info.key, "test");
        assert_eq!(info.key_type, "none");
        assert_eq!(info.ttl, -2);
        assert!(!info.exists);
    }

    #[test]
    fn test_key_info_new() {
        let info = KeyInfo::new("test".to_string(), "string".to_string(), 100);
        assert_eq!(info.key, "test");
        assert_eq!(info.key_type, "string");
        assert_eq!(info.ttl, 100);
        assert!(info.exists);
    }

    #[test]
    fn test_key_info_builder() {
        let info = KeyInfo::new("test".to_string(), "string".to_string(), 100)
            .with_pttl(100_000)
            .with_memory_usage(256)
            .with_encoding("embstr".to_string())
            .with_idle_time(5)
            .with_ref_count(1);

        assert_eq!(info.pttl, Some(100_000));
        assert_eq!(info.memory_usage, Some(256));
        assert_eq!(info.encoding.as_deref(), Some("embstr"));
        assert_eq!(info.idle_time, Some(5));
        assert_eq!(info.ref_count, Some(1));
    }

    #[test]
    fn test_expire_options_default() {
        let opts = ExpireOptions::default();
        assert!(!opts.nx);
        assert!(!opts.xx);
        assert!(!opts.gt);
        assert!(!opts.lt);
    }
}
