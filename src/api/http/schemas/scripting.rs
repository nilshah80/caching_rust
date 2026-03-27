//! Scripting Schemas
//!
//! Request and response types for Redis Lua scripting API endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// ========== Request Types ==========

/// Request to evaluate a Lua script
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct EvalRequest {
    /// The Lua script to evaluate
    #[validate(length(min = 1, message = "Script cannot be empty"))]
    #[schema(example = "return redis.call('GET', KEYS[1])")]
    pub script: String,

    /// Keys that the script will access (used for cluster routing)
    #[serde(default)]
    #[schema(example = json!(["user:1", "user:2"]))]
    pub keys: Vec<String>,

    /// Arguments to pass to the script (ARGV)
    #[serde(default)]
    #[schema(example = json!(["arg1", 42, true]))]
    pub args: Vec<serde_json::Value>,

    /// Execute in read-only mode (EVAL_RO) - prevents writes
    #[serde(default)]
    #[schema(example = false)]
    pub readonly: bool,
}

/// Request to evaluate a cached script by SHA1 hash
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct EvalShaRequest {
    /// SHA1 hash of the cached script
    #[validate(length(equal = 40, message = "SHA must be 40 characters"))]
    #[schema(example = "6b1bf486c81ceb7edf3c093f4a73d3e117c0b169")]
    pub sha: String,

    /// Keys that the script will access (used for cluster routing)
    #[serde(default)]
    #[schema(example = json!(["user:1", "user:2"]))]
    pub keys: Vec<String>,

    /// Arguments to pass to the script (ARGV)
    #[serde(default)]
    #[schema(example = json!(["arg1", 42, true]))]
    pub args: Vec<serde_json::Value>,

    /// Execute in read-only mode (EVALSHA_RO) - prevents writes
    #[serde(default)]
    #[schema(example = false)]
    pub readonly: bool,
}

/// Request to load a script into the script cache
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct ScriptLoadRequest {
    /// The Lua script to cache
    #[validate(length(min = 1, message = "Script cannot be empty"))]
    #[schema(example = "return redis.call('GET', KEYS[1])")]
    pub script: String,
}

/// Request to check if scripts exist in cache
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct ScriptExistsRequest {
    /// SHA1 hashes to check
    #[validate(length(min = 1, message = "At least one SHA is required"))]
    #[schema(example = json!(["6b1bf486c81ceb7edf3c093f4a73d3e117c0b169"]))]
    pub shas: Vec<String>,
}

/// Request to flush scripts from cache
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ScriptFlushRequest {
    /// Flush mode: ASYNC or SYNC
    #[serde(default)]
    #[schema(example = "ASYNC")]
    pub mode: Option<FlushMode>,
}

/// Flush mode for SCRIPT FLUSH
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FlushMode {
    /// Flush asynchronously (default, non-blocking)
    #[default]
    Async,
    /// Flush synchronously (blocking)
    Sync,
}

/// Debug mode for SCRIPT DEBUG
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScriptDebugMode {
    /// Enable non-blocking asynchronous debugging
    Yes,
    /// Enable blocking synchronous debugging
    Sync,
    /// Disable debugging
    No,
}

/// Request to set script debug mode
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScriptDebugRequest {
    /// Debug mode to set
    pub mode: ScriptDebugMode,
}

// ========== Response Types ==========

/// Response containing script execution result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EvalResponse {
    /// The result of script execution (can be any JSON value)
    pub result: serde_json::Value,
}

/// Response from SCRIPT LOAD
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScriptLoadResponse {
    /// SHA1 hash of the loaded script
    #[schema(example = "6b1bf486c81ceb7edf3c093f4a73d3e117c0b169")]
    pub sha: String,
}

/// Result for a single SHA existence check
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScriptExistsResult {
    /// The SHA1 hash that was checked
    #[schema(example = "6b1bf486c81ceb7edf3c093f4a73d3e117c0b169")]
    pub sha: String,
    /// Whether the script exists in cache
    pub exists: bool,
}

/// Response from SCRIPT EXISTS
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScriptExistsResponse {
    /// Results for each SHA checked
    pub results: Vec<ScriptExistsResult>,
}

/// Response from SCRIPT FLUSH
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScriptFlushResponse {
    /// Whether the flush was successful
    pub success: bool,
}

/// Response from SCRIPT KILL
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScriptKillResponse {
    /// Whether the kill was successful
    pub success: bool,
}

/// Response from SCRIPT DEBUG
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScriptDebugResponse {
    /// Whether the debug mode was set successfully
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_request_deserialization() {
        let json = r#"{
            "script": "return KEYS[1]",
            "keys": ["key1"],
            "args": ["arg1", 42]
        }"#;
        let request: EvalRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.script, "return KEYS[1]");
        assert_eq!(request.keys.len(), 1);
        assert_eq!(request.args.len(), 2);
        assert!(!request.readonly);
    }

    #[test]
    fn test_eval_request_with_readonly() {
        let json = r#"{
            "script": "return KEYS[1]",
            "keys": [],
            "args": [],
            "readonly": true
        }"#;
        let request: EvalRequest = serde_json::from_str(json).unwrap();
        assert!(request.readonly);
    }

    #[test]
    fn test_eval_request_defaults() {
        let json = r#"{"script": "return 1"}"#;
        let request: EvalRequest = serde_json::from_str(json).unwrap();
        assert!(request.keys.is_empty());
        assert!(request.args.is_empty());
        assert!(!request.readonly);
    }

    #[test]
    fn test_evalsha_request_deserialization() {
        let json = r#"{
            "sha": "6b1bf486c81ceb7edf3c093f4a73d3e117c0b169",
            "keys": ["key1"]
        }"#;
        let request: EvalShaRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.sha.len(), 40);
        assert!(!request.readonly);
    }

    #[test]
    fn test_script_load_request() {
        let json = r#"{"script": "return 1"}"#;
        let request: ScriptLoadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.script, "return 1");
    }

    #[test]
    fn test_script_load_response() {
        let response = ScriptLoadResponse {
            sha: "6b1bf486c81ceb7edf3c093f4a73d3e117c0b169".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("sha"));
    }

    #[test]
    fn test_script_exists_request() {
        let json = r#"{
            "shas": ["6b1bf486c81ceb7edf3c093f4a73d3e117c0b169", "abc123"]
        }"#;
        let request: ScriptExistsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.shas.len(), 2);
    }

    #[test]
    fn test_script_exists_response() {
        let response = ScriptExistsResponse {
            results: vec![
                ScriptExistsResult {
                    sha: "abc123".to_string(),
                    exists: true,
                },
                ScriptExistsResult {
                    sha: "def456".to_string(),
                    exists: false,
                },
            ],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("results"));
        assert!(json.contains("exists"));
    }

    #[test]
    fn test_flush_mode_serialization() {
        let mode = FlushMode::Async;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"ASYNC\"");

        let mode = FlushMode::Sync;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"SYNC\"");
    }

    #[test]
    fn test_script_flush_request_default() {
        let json = r#"{}"#;
        let request: ScriptFlushRequest = serde_json::from_str(json).unwrap();
        assert!(request.mode.is_none());
    }

    #[test]
    fn test_script_flush_request_with_mode() {
        let json = r#"{"mode": "SYNC"}"#;
        let request: ScriptFlushRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.mode, Some(FlushMode::Sync)));
    }

    #[test]
    fn test_script_debug_mode_serialization() {
        let mode = ScriptDebugMode::Yes;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"YES\"");

        let mode = ScriptDebugMode::Sync;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"SYNC\"");

        let mode = ScriptDebugMode::No;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"NO\"");
    }

    #[test]
    fn test_script_debug_request() {
        let json = r#"{"mode": "YES"}"#;
        let request: ScriptDebugRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.mode, ScriptDebugMode::Yes));
    }

    #[test]
    fn test_eval_response_serialization() {
        let response = EvalResponse {
            result: serde_json::json!({"key": "value", "count": 42}),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("result"));
        assert!(json.contains("key"));
    }

    #[test]
    fn test_eval_response_with_array() {
        let response = EvalResponse {
            result: serde_json::json!([1, 2, 3, "hello"]),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("[1,2,3,\"hello\"]"));
    }

    #[test]
    fn test_eval_response_with_null() {
        let response = EvalResponse {
            result: serde_json::Value::Null,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("null"));
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn evalsha_short_sha_fails() {
        let req = EvalShaRequest {
            sha: "abc123".into(),
            keys: vec![],
            args: vec![],
            readonly: false,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn evalsha_long_sha_fails() {
        let req = EvalShaRequest {
            sha: "a".repeat(41),
            keys: vec![],
            args: vec![],
            readonly: false,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn evalsha_valid_40_char_sha_passes() {
        let req = EvalShaRequest {
            sha: "6b1bf486c81ceb7edf3c093f4a73d3e117c0b169".into(),
            keys: vec![],
            args: vec![],
            readonly: false,
        };
        assert!(req.validate().is_ok());
    }
}
