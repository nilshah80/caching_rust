//! String Schemas
//!
//! Request/response schemas for string operations.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::collections::HashMap;
use validator::Validate;

/// Request to set a string value
#[allow(clippy::struct_excessive_bools)] // Mirrors Redis SET command options
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SetStringRequest {
    /// The value to set
    #[validate(length(min = 0))]
    pub value: String,

    /// TTL in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,

    /// TTL in milliseconds (takes precedence over `ttl_seconds`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,

    /// Only set if key does not exist (NX)
    #[serde(default)]
    pub nx: bool,

    /// Only set if key exists (XX)
    #[serde(default)]
    pub xx: bool,

    /// Return the previous value (GET)
    #[serde(default)]
    pub get: bool,

    /// Keep the existing TTL (KEEPTTL)
    #[serde(default)]
    pub keep_ttl: bool,
}

/// Response after setting a string
#[derive(Debug, Serialize, ToSchema)]
pub struct SetStringResponse {
    /// The key that was set
    pub key: String,

    /// Whether the operation was successful
    pub success: bool,

    /// Previous value if GET option was used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_value: Option<String>,
}

/// Request for MGET operation
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MGetRequest {
    /// Keys to retrieve
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
}

/// Response for MGET operation
#[derive(Debug, Serialize, ToSchema)]
pub struct MGetResponse {
    /// Keys that were found with their values
    pub found: HashMap<String, String>,

    /// Keys that were not found
    pub missing: Vec<String>,

    /// Total count of keys requested
    pub total_requested: usize,

    /// Count of keys found
    pub found_count: usize,
}

/// Request for MSET operation
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MSetRequest {
    /// Key-value pairs to set
    #[validate(length(min = 1, message = "At least one pair is required"))]
    pub pairs: HashMap<String, String>,

    /// Only set if none of the keys exist (NX mode)
    #[serde(default)]
    pub nx: bool,
}

/// Response for MSET operation
#[derive(Debug, Serialize, ToSchema)]
pub struct MSetResponse {
    /// Number of keys set
    pub count: usize,

    /// Keys that were set
    pub keys: Vec<String>,

    /// Whether operation succeeded (for MSETNX)
    pub success: bool,
}

/// Request to increment a value
#[derive(Debug, Deserialize, ToSchema)]
pub struct IncrementRequest {
    /// Amount to increment by (default: 1)
    #[serde(default = "default_increment")]
    pub delta: i64,

    /// Whether to use float increment
    #[serde(default)]
    pub float: bool,

    /// Float delta (used when float is true)
    pub float_delta: Option<f64>,
}

const fn default_increment() -> i64 {
    1
}

/// Response for increment operation
#[derive(Debug, Serialize, ToSchema)]
pub struct IncrementResponse {
    /// The key that was incremented
    pub key: String,

    /// New value after increment
    pub new_value: String,
}

/// Request to append to a string
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AppendRequest {
    /// Value to append
    #[validate(length(min = 1))]
    pub value: String,
}

/// Response for append operation
#[derive(Debug, Serialize, ToSchema)]
pub struct AppendResponse {
    /// The key that was appended to
    pub key: String,

    /// New length of the string
    pub new_length: i64,
}

/// Query parameters for GETRANGE
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetRangeParams {
    /// Start index (can be negative)
    pub start: i64,

    /// End index (can be negative)
    pub end: i64,
}

/// Response for GETRANGE operation
#[derive(Debug, Serialize, ToSchema)]
pub struct GetRangeResponse {
    /// The key
    pub key: String,

    /// The substring value
    pub value: String,

    /// Start index used
    pub start: i64,

    /// End index used
    pub end: i64,
}

/// Request for SETRANGE operation
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SetRangeRequest {
    /// Offset to start writing at
    #[validate(range(min = 0))]
    pub offset: i64,

    /// Value to write
    pub value: String,
}

/// Response for SETRANGE operation
#[derive(Debug, Serialize, ToSchema)]
pub struct SetRangeResponse {
    /// The key
    pub key: String,

    /// New length of the string
    pub new_length: i64,
}

/// Query parameters for GETEX
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetExParams {
    /// New TTL in seconds
    pub ttl_seconds: Option<u64>,

    /// New TTL in milliseconds
    pub ttl_ms: Option<u64>,

    /// Remove the TTL (PERSIST)
    #[serde(default)]
    pub persist: bool,
}

/// Response for GETDEL operation
#[derive(Debug, Serialize, ToSchema)]
pub struct GetDelResponse {
    /// The key that was deleted
    pub key: String,

    /// The value that was stored (None if key didn't exist)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Whether the key existed
    pub existed: bool,
}

/// Response for STRLEN operation
#[derive(Debug, Serialize, ToSchema)]
pub struct StrLenResponse {
    /// The key
    pub key: String,

    /// Length of the string
    pub length: i64,
}
