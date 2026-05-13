//! String Value Entity
//!
//! Domain entity representing a Redis string value.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// String value with metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StringValue {
    /// The key name
    pub key: String,

    /// The string value
    pub value: String,

    /// Redis data type (always "string")
    #[serde(rename = "type")]
    pub data_type: String,

    /// Time-to-live in seconds (None if no expiry)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i64>,

    /// Length of the value in bytes
    pub length: usize,

    /// Internal Redis encoding (embstr, int, raw)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

impl StringValue {
    /// Create a new StringValue
    pub fn new(key: String, value: String) -> Self {
        let length = value.len();
        Self {
            key,
            value,
            data_type: "string".to_string(),
            ttl: None,
            length,
            encoding: None,
        }
    }

    /// Create with TTL
    pub fn with_ttl(mut self, ttl: Option<i64>) -> Self {
        self.ttl = ttl;
        self
    }

    /// Create with encoding
    pub fn with_encoding(mut self, encoding: Option<String>) -> Self {
        self.encoding = encoding;
        self
    }
}

/// Options for SET command
#[derive(Debug, Clone, Default)]
pub struct SetOptions {
    /// Only set if key does not exist (NX)
    pub nx: bool,

    /// Only set if key exists (XX)
    pub xx: bool,

    /// Return the old value (GET)
    pub get: bool,

    /// Expiry mode (EX, PX, EXAT, PXAT)
    pub expiry_mode: Option<ExpiryMode>,

    /// Expiry value (seconds or milliseconds depending on mode)
    pub expiry_value: Option<u64>,

    /// Keep existing TTL (KEEPTTL)
    pub keep_ttl: bool,

    /// Conditional predicate (IFEQ, IFNE, IFDEQ, IFDNE; Redis 8.4+)
    pub condition: Option<SetCondition>,
}

/// Conditional predicate for the SET command (Redis 8.4+).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetCondition {
    /// IFEQ — set only when the current value equals the supplied string
    IfEq(String),
    /// IFNE — set only when the current value is not equal to the supplied string
    IfNe(String),
    /// IFDEQ — set only when the current value's XXH3 digest matches
    IfDeq(String),
    /// IFDNE — set only when the current value's XXH3 digest does not match
    IfDne(String),
}

/// Expiry mode for SET command
#[derive(Debug, Clone, Copy)]
pub enum ExpiryMode {
    /// Seconds
    Ex,
    /// Milliseconds
    Px,
    /// Unix timestamp in seconds
    ExAt,
    /// Unix timestamp in milliseconds
    PxAt,
}

impl ExpiryMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExpiryMode::Ex => "EX",
            ExpiryMode::Px => "PX",
            ExpiryMode::ExAt => "EXAT",
            ExpiryMode::PxAt => "PXAT",
        }
    }
}

/// Result of SET operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SetResult {
    /// The key that was set
    pub key: String,

    /// Whether the operation was successful
    pub success: bool,

    /// Previous value if GET option was used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_value: Option<String>,
}

/// Result of MGET operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MGetResult {
    /// Keys that were found with their values
    pub found: std::collections::HashMap<String, String>,

    /// Keys that were not found
    pub missing: Vec<String>,
}

/// Result of increment operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IncrementResult {
    /// The key that was incremented
    pub key: String,

    /// The new value after increment
    pub new_value: String,
}

/// Result of append operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AppendResult {
    /// The key that was appended to
    pub key: String,

    /// New length of the string
    pub new_length: i64,
}

/// Result of range operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RangeResult {
    /// The key
    pub key: String,

    /// The substring value
    pub value: String,

    /// Start index
    pub start: i64,

    /// End index
    pub end: i64,
}

/// Result of SETRANGE operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SetRangeResult {
    /// The key
    pub key: String,

    /// New length of the string
    pub new_length: i64,
}

/// Options for GETEX command
#[derive(Debug, Clone)]
pub struct GetExOptions {
    /// Expiry mode
    pub expiry_mode: Option<ExpiryMode>,

    /// Expiry value
    pub expiry_value: Option<u64>,

    /// Remove TTL (PERSIST)
    pub persist: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_value_new() {
        let value = StringValue::new("key".to_string(), "value".to_string());
        assert_eq!(value.key, "key");
        assert_eq!(value.value, "value");
        assert_eq!(value.data_type, "string");
        assert_eq!(value.length, 5);
        assert!(value.ttl.is_none());
        assert!(value.encoding.is_none());
    }

    #[test]
    fn test_string_value_with_ttl_and_encoding() {
        let value = StringValue::new("key".to_string(), "value".to_string())
            .with_ttl(Some(10))
            .with_encoding(Some("embstr".to_string()));
        assert_eq!(value.ttl, Some(10));
        assert_eq!(value.encoding.as_deref(), Some("embstr"));
    }

    #[test]
    fn test_expiry_mode_as_str() {
        assert_eq!(ExpiryMode::Ex.as_str(), "EX");
        assert_eq!(ExpiryMode::Px.as_str(), "PX");
        assert_eq!(ExpiryMode::ExAt.as_str(), "EXAT");
        assert_eq!(ExpiryMode::PxAt.as_str(), "PXAT");
    }
}
