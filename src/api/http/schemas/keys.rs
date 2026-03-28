//! Key Schemas
//!
//! Request/response schemas for key management operations.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request to delete keys
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct DeleteKeysRequest {
    /// Keys to delete
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,

    /// Use async deletion (UNLINK) for large keys
    #[serde(default)]
    pub async_delete: bool,
}

/// Response for delete operation
#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteKeysResponse {
    /// Keys that were deleted
    pub deleted: Vec<String>,

    /// Keys that didn't exist
    pub not_found: Vec<String>,

    /// Number of keys deleted
    pub count: usize,
}

/// Request to check key existence
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ExistsRequest {
    /// Keys to check
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
}

/// Response for exists operation
#[derive(Debug, Serialize, ToSchema)]
pub struct ExistsResponse {
    /// Keys that exist
    pub existing: Vec<String>,

    /// Keys that don't exist
    pub missing: Vec<String>,

    /// Count of existing keys
    pub count: usize,
}

/// Request to set expiration
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExpireRequest {
    /// TTL in seconds (for EXPIRE)
    pub seconds: Option<i64>,

    /// TTL in milliseconds (for PEXPIRE)
    pub milliseconds: Option<i64>,

    /// Unix timestamp in seconds (for EXPIREAT)
    pub expire_at: Option<i64>,

    /// Unix timestamp in milliseconds (for PEXPIREAT)
    pub pexpire_at: Option<i64>,

    /// Only set expiry if key has no expiry (NX)
    #[serde(default)]
    pub nx: bool,

    /// Only set expiry if key already has an expiry (XX)
    #[serde(default)]
    pub xx: bool,

    /// Only set expiry if new expiry > current (GT)
    #[serde(default)]
    pub gt: bool,

    /// Only set expiry if new expiry < current (LT)
    #[serde(default)]
    pub lt: bool,
}

/// Response for expire operation
#[derive(Debug, Serialize, ToSchema)]
pub struct ExpireResponse {
    /// The key
    pub key: String,

    /// Whether the TTL was set
    pub success: bool,

    /// The new TTL in seconds (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_ttl: Option<i64>,
}

/// Response for TTL query
#[derive(Debug, Serialize, ToSchema)]
pub struct TtlResponse {
    /// The key
    pub key: String,

    /// TTL in seconds (-1 = no expiry, -2 = key doesn't exist)
    pub ttl: i64,

    /// TTL in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pttl: Option<i64>,
}

/// Response for persist operation
#[derive(Debug, Serialize, ToSchema)]
pub struct PersistResponse {
    /// The key
    pub key: String,

    /// Whether the TTL was removed
    pub success: bool,
}

/// Response for type query
#[derive(Debug, Serialize, ToSchema)]
pub struct TypeResponse {
    /// The key
    pub key: String,

    /// The Redis data type (string, list, set, zset, hash, stream, none)
    #[serde(rename = "type")]
    pub key_type: String,
}

/// Request to rename a key
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RenameRequest {
    /// New key name
    #[validate(length(min = 1, message = "New key name is required"))]
    pub new_key: String,

    /// Only rename if new key doesn't exist (NX)
    #[serde(default)]
    pub nx: bool,
}

/// Response for rename operation
#[derive(Debug, Serialize, ToSchema)]
pub struct RenameResponse {
    /// Original key name
    pub old_key: String,

    /// New key name
    pub new_key: String,

    /// Whether the rename succeeded
    pub success: bool,
}

/// Request to copy a key
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CopyRequest {
    /// Destination key name
    #[validate(length(min = 1, message = "Destination key is required"))]
    pub destination: String,

    /// Target database index (optional)
    pub db: Option<i64>,

    /// Replace existing key
    #[serde(default)]
    pub replace: bool,
}

/// Response for copy operation
#[derive(Debug, Serialize, ToSchema)]
pub struct CopyResponse {
    /// Source key
    pub source: String,

    /// Destination key
    pub destination: String,

    /// Whether the copy succeeded
    pub success: bool,
}

/// Query parameters for SCAN
#[derive(Debug, Deserialize, ToSchema)]
pub struct ScanParams {
    /// Cursor position (0 to start)
    #[serde(default)]
    pub cursor: u64,

    /// Pattern to match
    pub pattern: Option<String>,

    /// Number of keys to return per iteration
    pub count: Option<u64>,

    /// Filter by key type
    #[serde(rename = "type")]
    pub key_type: Option<String>,
}

/// Response for SCAN operation
#[derive(Debug, Serialize, ToSchema)]
pub struct ScanResponse {
    /// Next cursor (0 means complete)
    pub cursor: u64,

    /// Keys found in this iteration
    pub keys: Vec<String>,

    /// Count of keys returned
    pub count: usize,
}

/// Query parameters for KEYS
#[derive(Debug, Deserialize, ToSchema)]
pub struct KeysParams {
    /// Pattern to match (e.g., "user:*")
    pub pattern: String,
}

/// Response for KEYS operation
#[derive(Debug, Serialize, ToSchema)]
pub struct KeysResponse {
    /// Matching keys
    pub keys: Vec<String>,

    /// Count of keys found
    pub count: usize,
}

/// Request to touch keys
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TouchRequest {
    /// Keys to touch
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
}

/// Response for touch operation
#[derive(Debug, Serialize, ToSchema)]
pub struct TouchResponse {
    /// Number of keys touched
    pub count: usize,
}

/// Response for random key
#[derive(Debug, Serialize, ToSchema)]
pub struct RandomKeyResponse {
    /// Random key (null if database is empty)
    pub key: Option<String>,
}

/// Response for dump operation
#[derive(Debug, Serialize, ToSchema)]
pub struct DumpResponse {
    /// The key
    pub key: String,

    /// Serialized value (base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Request to restore a key
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RestoreRequest {
    /// TTL in milliseconds (0 = no expiry)
    #[serde(default)]
    pub ttl: i64,

    /// Serialized data (base64 encoded)
    #[validate(length(min = 1, message = "Data is required"))]
    pub data: String,

    /// Replace existing key
    #[serde(default)]
    pub replace: bool,
}

/// Response for restore operation
#[derive(Debug, Serialize, ToSchema)]
pub struct RestoreResponse {
    /// The key
    pub key: String,

    /// Whether the restore succeeded
    pub success: bool,
}

/// Response for object info
#[derive(Debug, Serialize, ToSchema)]
pub struct ObjectInfoResponse {
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

    /// Frequency counter (LFU)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freq: Option<u64>,
}

/// Response for key info (comprehensive)
#[derive(Debug, Serialize, ToSchema)]
pub struct KeyInfoResponse {
    /// The key name
    pub key: String,

    /// Redis data type
    #[serde(rename = "type")]
    pub key_type: String,

    /// TTL in seconds
    pub ttl: i64,

    /// TTL in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pttl: Option<i64>,

    /// Whether the key exists
    pub exists: bool,

    /// Memory usage in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_usage: Option<i64>,

    /// Internal encoding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,

    /// Idle time in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_time: Option<u64>,

    /// Reference count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_count: Option<u64>,
}

/// Sort order for SORT command
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum SortOrderSchema {
    #[default]
    Asc,
    Desc,
}

/// Request body for SORT / SORT_RO operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct SortRequest {
    /// BY pattern for external key sorting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,

    /// GET patterns to retrieve external key values
    #[serde(default)]
    pub get: Vec<String>,

    /// Offset for LIMIT
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,

    /// Count for LIMIT
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,

    /// Sort order (ASC or DESC)
    #[serde(default)]
    pub order: SortOrderSchema,

    /// Sort alphabetically instead of numerically
    #[serde(default)]
    pub alpha: bool,
}

/// Request body for SORT...STORE operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct SortStoreRequest {
    /// Destination key to store sorted results
    #[validate(length(min = 1, message = "Destination key is required"))]
    pub destination: String,

    /// Sort options
    #[serde(flatten)]
    pub options: SortRequest,
}

/// Response for SORT / SORT_RO operations
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SortResponse {
    /// Sorted values (None for nil entries from GET patterns)
    pub values: Vec<Option<String>>,
}

/// Response for SORT...STORE operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SortStoreResponse {
    /// Number of elements stored
    pub count: i64,
}

impl From<SortOrderSchema> for crate::domain::repositories::SortOrder {
    fn from(schema: SortOrderSchema) -> Self {
        match schema {
            SortOrderSchema::Asc => crate::domain::repositories::SortOrder::Asc,
            SortOrderSchema::Desc => crate::domain::repositories::SortOrder::Desc,
        }
    }
}

impl SortRequest {
    /// Convert to domain SortOptions
    pub fn into_sort_options(self) -> crate::domain::repositories::SortOptions {
        let limit = match (self.offset, self.count) {
            (Some(offset), Some(count)) => Some((offset, count)),
            _ => None,
        };
        crate::domain::repositories::SortOptions {
            by: self.by,
            get: self.get,
            limit,
            order: self.order.into(),
            alpha: self.alpha,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expire_request_defaults() {
        let json = r#"{"seconds": 100}"#;
        let req: ExpireRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.seconds, Some(100));
        assert!(!req.nx);
        assert!(!req.xx);
        assert!(!req.gt);
        assert!(!req.lt);
    }

    #[test]
    fn test_scan_params_defaults() {
        let params = ScanParams {
            cursor: 0,
            pattern: None,
            count: None,
            key_type: None,
        };
        assert_eq!(params.cursor, 0);
    }

    #[test]
    fn test_sort_store_request_empty_destination_fails_validation() {
        let req = SortStoreRequest {
            destination: "".to_string(),
            options: SortRequest {
                by: None,
                get: vec![],
                offset: None,
                count: None,
                order: SortOrderSchema::Asc,
                alpha: false,
            },
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_sort_store_request_valid_passes_validation() {
        let req = SortStoreRequest {
            destination: "dest_key".to_string(),
            options: SortRequest {
                by: None,
                get: vec![],
                offset: None,
                count: None,
                order: SortOrderSchema::Asc,
                alpha: false,
            },
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_sort_order_desc_conversion() {
        let order: crate::domain::repositories::SortOrder = SortOrderSchema::Desc.into();
        assert!(matches!(order, crate::domain::repositories::SortOrder::Desc));
    }

    #[test]
    fn test_sort_request_into_sort_options_with_limit() {
        let req = SortRequest {
            by: Some("weight_*".to_string()),
            get: vec!["#".to_string()],
            offset: Some(10),
            count: Some(5),
            order: SortOrderSchema::Desc,
            alpha: true,
        };
        let opts = req.into_sort_options();
        assert_eq!(opts.limit, Some((10, 5)));
        assert!(opts.alpha);
        assert!(matches!(opts.order, crate::domain::repositories::SortOrder::Desc));
    }
}
