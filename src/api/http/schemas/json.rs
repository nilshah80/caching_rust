//! JSON Schemas
//!
//! Request/response schemas for RedisJSON operations.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use validator::Validate;

// ==================== Core Operation Schemas ====================

/// Request to set a JSON value
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct JsonSetRequest {
    /// The JSON value to set
    pub value: Value,

    /// JSONPath where to set the value (default: "$" for root)
    #[serde(default = "default_path")]
    pub path: String,

    /// Only set if key does not exist (NX)
    #[serde(default)]
    pub nx: bool,

    /// Only set if key exists (XX)
    #[serde(default)]
    pub xx: bool,
}

/// Response for JSON.SET operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonSetResponse {
    /// The key that was set
    pub key: String,

    /// The path where the value was set
    pub path: String,

    /// Whether the operation was successful
    pub success: bool,
}

/// Query parameters for JSON.GET operation
///
/// Supports multiple paths via repeated query params: `?path=$.a&path=$.b`
/// or a single path: `?path=$`
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonGetParams {
    /// JSONPath(s) to retrieve (use repeated params for multiple, default: "$")
    #[serde(default = "default_path")]
    pub path: String,
}

/// Response for JSON.GET operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonGetResponse {
    /// The key
    pub key: String,

    /// The path(s) queried
    pub paths: Vec<String>,

    /// The JSON value
    pub value: Value,
}

/// Request for JSON.MGET operation
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct JsonMGetRequest {
    /// Keys to retrieve
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,

    /// JSONPath to retrieve (default: "$")
    #[serde(default = "default_path")]
    pub path: String,
}

/// Single item in MGET response
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonMGetItem {
    /// The key
    pub key: String,

    /// The value (None if key doesn't exist or path not found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

/// Response for JSON.MGET operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonMGetResponse {
    /// Results for each key
    pub results: Vec<JsonMGetItem>,

    /// Path that was queried
    pub path: String,
}

/// Single item for JSON.MSET request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JsonMSetItemRequest {
    /// The key
    pub key: String,

    /// JSONPath where to set (default: "$")
    #[serde(default = "default_path")]
    pub path: String,

    /// The JSON value to set
    pub value: Value,
}

/// Request for JSON.MSET operation
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct JsonMSetRequest {
    /// Items to set
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<JsonMSetItemRequest>,
}

/// Query parameters for JSON.DEL operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonDelParams {
    /// JSONPath to delete (default: "$" for entire document)
    #[serde(default = "default_path")]
    pub path: String,
}

/// Response for JSON.DEL operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonDelResponse {
    /// The key
    pub key: String,

    /// The path that was deleted
    pub path: String,

    /// Number of paths deleted
    pub deleted_count: i64,
}

/// Query parameters for JSON.TYPE operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonTypeParams {
    /// JSONPath to check (default: "$")
    #[serde(default = "default_path")]
    pub path: String,
}

/// Response for JSON.TYPE operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonTypeResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// JSON type(s) at path
    pub types: Vec<Option<String>>,
}

// ==================== String Operation Schemas ====================

/// Query parameters for JSON.STRLEN operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonStrLenParams {
    /// JSONPath to the string (default: "$")
    #[serde(default = "default_path")]
    pub path: String,
}

/// Response for JSON.STRLEN operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonStrLenResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// String length(s) (None if not a string)
    pub lengths: Vec<Option<i64>>,
}

/// Request for JSON.STRAPPEND operation
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct JsonStrAppendRequest {
    /// JSONPath to the string (default: "$")
    #[serde(default = "default_path")]
    pub path: String,

    /// String to append
    #[validate(length(min = 1, message = "Value cannot be empty"))]
    pub value: String,
}

/// Response for JSON.STRAPPEND operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonStrAppendResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// New string length(s) after append
    pub new_lengths: Vec<Option<i64>>,
}

// ==================== Numeric Operation Schemas ====================

/// Request for JSON.NUMINCRBY operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonNumIncrByRequest {
    /// JSONPath to the number (default: "$")
    #[serde(default = "default_path")]
    pub path: String,

    /// Amount to increment by (can be negative)
    pub value: f64,
}

/// Request for JSON.NUMMULTBY operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonNumMultByRequest {
    /// JSONPath to the number (default: "$")
    #[serde(default = "default_path")]
    pub path: String,

    /// Multiplier
    pub value: f64,
}

/// Response for JSON.NUMINCRBY/NUMMULTBY operations
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonNumResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// New value(s) after operation
    pub values: Value,
}

/// Query parameters for JSON.TOGGLE operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonToggleParams {
    /// JSONPath to the boolean (default: "$")
    #[serde(default = "default_path")]
    pub path: String,
}

/// Response for JSON.TOGGLE operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonToggleResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// New boolean value(s) (None if not a boolean)
    pub values: Vec<Option<bool>>,
}

/// Query parameters for JSON.CLEAR operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonClearParams {
    /// JSONPath to clear (default: "$")
    #[serde(default = "default_path")]
    pub path: String,
}

/// Response for JSON.CLEAR operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonClearResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Number of values cleared
    pub cleared_count: i64,
}

// ==================== Array Operation Schemas ====================

/// Query parameters for JSON.ARRLEN operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonArrLenParams {
    /// JSONPath to the array (default: "$")
    #[serde(default = "default_path")]
    pub path: String,
}

/// Response for JSON.ARRLEN operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonArrLenResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Array length(s) (None if not an array)
    pub lengths: Vec<Option<i64>>,
}

/// Request for JSON.ARRAPPEND operation
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct JsonArrAppendRequest {
    /// JSONPath to the array (default: "$")
    #[serde(default = "default_path")]
    pub path: String,

    /// Values to append
    #[validate(length(min = 1, message = "At least one value is required"))]
    pub values: Vec<Value>,
}

/// Response for JSON.ARRAPPEND operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonArrAppendResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// New array length(s) after append
    pub new_lengths: Vec<Option<i64>>,
}

/// Request for JSON.ARRINDEX operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonArrIndexRequest {
    /// JSONPath to the array (default: "$")
    #[serde(default = "default_path")]
    pub path: String,

    /// Value to search for
    pub value: Value,

    /// Start index (optional)
    pub start: Option<i64>,

    /// Stop index (optional)
    pub stop: Option<i64>,
}

/// Response for JSON.ARRINDEX operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonArrIndexResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Index of element (-1 if not found, None if not an array)
    pub indices: Vec<Option<i64>>,
}

/// Request for JSON.ARRINSERT operation
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct JsonArrInsertRequest {
    /// JSONPath to the array (default: "$")
    #[serde(default = "default_path")]
    pub path: String,

    /// Index to insert at
    pub index: i64,

    /// Values to insert
    #[validate(length(min = 1, message = "At least one value is required"))]
    pub values: Vec<Value>,
}

/// Response for JSON.ARRINSERT operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonArrInsertResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// New array length(s) after insert
    pub new_lengths: Vec<Option<i64>>,
}

/// Request for JSON.ARRPOP operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonArrPopRequest {
    /// JSONPath to the array (default: "$")
    #[serde(default = "default_path")]
    pub path: String,

    /// Index to pop from (default: -1, last element)
    pub index: Option<i64>,
}

/// Response for JSON.ARRPOP operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonArrPopResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Popped value(s) (None if array is empty or not an array)
    pub values: Vec<Option<Value>>,
}

/// Request for JSON.ARRTRIM operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonArrTrimRequest {
    /// JSONPath to the array (default: "$")
    #[serde(default = "default_path")]
    pub path: String,

    /// Start index (inclusive)
    pub start: i64,

    /// Stop index (inclusive)
    pub stop: i64,
}

/// Response for JSON.ARRTRIM operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonArrTrimResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// New array length(s) after trim
    pub new_lengths: Vec<Option<i64>>,
}

// ==================== Object Operation Schemas ====================

/// Query parameters for JSON.OBJLEN operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonObjLenParams {
    /// JSONPath to the object (default: "$")
    #[serde(default = "default_path")]
    pub path: String,
}

/// Response for JSON.OBJLEN operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonObjLenResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Object key count(s) (None if not an object)
    pub lengths: Vec<Option<i64>>,
}

/// Query parameters for JSON.OBJKEYS operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonObjKeysParams {
    /// JSONPath to the object (default: "$")
    #[serde(default = "default_path")]
    pub path: String,
}

/// Response for JSON.OBJKEYS operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonObjKeysResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Object keys (None if not an object)
    pub keys: Vec<Option<Vec<String>>>,
}

// ==================== Debug Operation Schemas ====================

/// Query parameters for JSON.DEBUG MEMORY operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonDebugMemoryParams {
    /// JSONPath (default: "$")
    #[serde(default = "default_path")]
    pub path: String,
}

/// Response for JSON.DEBUG MEMORY operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonDebugMemoryResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// Memory usage in bytes
    pub memory_bytes: Vec<Option<i64>>,
}

/// Query parameters for JSON.RESP operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct JsonRespParams {
    /// JSONPath (default: "$")
    #[serde(default = "default_path")]
    pub path: String,
}

/// Response for JSON.RESP operation
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonRespResponse {
    /// The key
    pub key: String,

    /// The path
    pub path: String,

    /// RESP representation
    pub resp: Value,
}

// ==================== Helper Functions ====================

fn default_path() -> String {
    "$".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_path() {
        assert_eq!(default_path(), "$");
    }

    #[test]
    fn test_json_set_request_deserialize() {
        let json = r#"{"value": {"name": "John"}, "nx": true}"#;
        let request: JsonSetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.path, "$");
        assert!(request.nx);
        assert!(!request.xx);
    }

    #[test]
    fn test_json_get_params_default() {
        let json = r#"{}"#;
        let params: JsonGetParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.path, "$");
    }

    #[test]
    fn test_json_get_params_single_path() {
        let json = r#"{"path": "$.name"}"#;
        let params: JsonGetParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.path, "$.name");
    }

    #[test]
    fn test_json_get_params_path_with_comma() {
        // Path containing a comma is preserved (not split)
        let json = r#"{"path": "$['a,b']"}"#;
        let params: JsonGetParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.path, "$['a,b']");
    }

    #[test]
    fn test_json_mget_request() {
        let json = r#"{"keys": ["key1", "key2"], "path": "$.name"}"#;
        let request: JsonMGetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.keys.len(), 2);
        assert_eq!(request.path, "$.name");
    }

    #[test]
    fn test_json_arr_append_request() {
        let json = r#"{"path": "$.tags", "values": ["tag1", "tag2"]}"#;
        let request: JsonArrAppendRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.path, "$.tags");
        assert_eq!(request.values.len(), 2);
    }
}
