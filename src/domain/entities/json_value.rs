//! JSON Value Entity
//!
//! Domain entities for RedisJSON operations.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Options for JSON.SET command
#[derive(Debug, Clone, Default)]
pub struct JsonSetOptions {
    /// Only set if key does not exist (NX)
    pub nx: bool,

    /// Only set if key exists (XX)
    pub xx: bool,
}

/// Result of JSON.SET operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonSetResult {
    /// The key that was set
    pub key: String,

    /// The path where the value was set
    pub path: String,

    /// Whether the operation was successful
    pub success: bool,
}

/// Result of JSON.GET operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonGetResult {
    /// The key
    pub key: String,

    /// The path(s) queried
    pub paths: Vec<String>,

    /// The JSON value (can be any JSON type)
    pub value: serde_json::Value,
}

/// Result of JSON.MGET operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonMGetResult {
    /// Keys with their JSON values (None for keys that don't exist)
    pub results: Vec<JsonMGetItem>,

    /// Path that was queried
    pub path: String,
}

/// Single item in MGET result
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonMGetItem {
    /// The key
    pub key: String,

    /// The value (None if key doesn't exist or path not found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

/// Result of JSON.DEL operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonDelResult {
    /// The key
    pub key: String,

    /// The path that was deleted
    pub path: String,

    /// Number of paths deleted
    pub deleted_count: i64,
}

/// Result of JSON.TYPE operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonTypeResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// The JSON type(s) at path (can be multiple for array results)
    pub types: Vec<Option<String>>,
}

/// Result of JSON.STRLEN operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonStrLenResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// String length(s) at path (None if not a string)
    pub lengths: Vec<Option<i64>>,
}

/// Result of JSON.STRAPPEND operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonStrAppendResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// New string length(s) after append
    pub new_lengths: Vec<Option<i64>>,
}

/// Result of JSON.NUMINCRBY/NUMMULTBY operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonNumResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// The new value(s) after operation
    pub values: serde_json::Value,
}

/// Result of JSON.TOGGLE operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonToggleResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// New boolean value(s) (None if not a boolean)
    pub values: Vec<Option<bool>>,
}

/// Result of JSON.CLEAR operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonClearResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Number of values cleared
    pub cleared_count: i64,
}

/// Result of JSON.ARRLEN operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonArrLenResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Array length(s) at path (None if not an array)
    pub lengths: Vec<Option<i64>>,
}

/// Result of JSON.ARRAPPEND operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonArrAppendResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// New array length(s) after append
    pub new_lengths: Vec<Option<i64>>,
}

/// Result of JSON.ARRINDEX operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonArrIndexResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Index of element (-1 if not found, None if not an array)
    pub indices: Vec<Option<i64>>,
}

/// Result of JSON.ARRINSERT operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonArrInsertResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// New array length(s) after insert
    pub new_lengths: Vec<Option<i64>>,
}

/// Result of JSON.ARRPOP operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonArrPopResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Popped value(s) (None if array is empty or not an array)
    pub values: Vec<Option<serde_json::Value>>,
}

/// Result of JSON.ARRTRIM operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonArrTrimResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// New array length(s) after trim
    pub new_lengths: Vec<Option<i64>>,
}

/// Result of JSON.OBJLEN operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonObjLenResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Object key count(s) at path (None if not an object)
    pub lengths: Vec<Option<i64>>,
}

/// Result of JSON.OBJKEYS operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonObjKeysResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Object keys at path (None if not an object)
    pub keys: Vec<Option<Vec<String>>>,
}

/// Result of JSON.DEBUG MEMORY operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonDebugMemoryResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Memory usage in bytes
    pub memory_bytes: Vec<Option<i64>>,
}

/// Result of JSON.RESP operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonRespResult {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// RESP representation of the value
    pub resp: serde_json::Value,
}

/// Item for JSON.MSET operation
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct JsonMSetItem {
    /// The key
    pub key: String,

    /// The path (defaults to "$")
    #[serde(default = "default_json_path")]
    pub path: String,

    /// The JSON value to set
    pub value: serde_json::Value,
}

fn default_json_path() -> String {
    "$".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_set_options_default() {
        let options = JsonSetOptions::default();
        assert!(!options.nx);
        assert!(!options.xx);
    }

    #[test]
    fn test_json_set_result() {
        let result = JsonSetResult {
            key: "user:1".to_string(),
            path: "$".to_string(),
            success: true,
        };
        assert_eq!(result.key, "user:1");
        assert!(result.success);
    }

    #[test]
    fn test_json_get_result() {
        let result = JsonGetResult {
            key: "user:1".to_string(),
            paths: vec!["$.name".to_string()],
            value: serde_json::json!("John"),
        };
        assert_eq!(result.key, "user:1");
        assert_eq!(result.paths.len(), 1);
    }

    #[test]
    fn test_json_mset_item_default_path() {
        let json = r#"{"key": "test", "value": "hello"}"#;
        let item: JsonMSetItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.path, "$");
    }

    #[test]
    fn test_json_mset_item_custom_path() {
        let json = r#"{"key": "test", "path": "$.name", "value": "hello"}"#;
        let item: JsonMSetItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.path, "$.name");
    }
}
