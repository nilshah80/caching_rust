//! Redis Functions schemas.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request to load a Redis library.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct FunctionLoadRequest {
    #[validate(length(min = 1, message = "Code cannot be empty"))]
    pub code: String,
    #[serde(default)]
    pub replace: bool,
}

/// Response from `FUNCTION LOAD`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionLoadResponse {
    pub library_name: String,
}

/// Flush mode for `FUNCTION FLUSH`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FunctionFlushModeSchema {
    Async,
    Sync,
}

/// Request for `FUNCTION FLUSH`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct FunctionFlushRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<FunctionFlushModeSchema>,
}

/// Response for boolean success operations.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionSuccessResponse {
    pub success: bool,
}

/// Query for `FUNCTION LIST`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct FunctionListQuery {
    #[serde(default)]
    pub with_code: bool,
}

/// Response for `FUNCTION LIST`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionListResponse {
    pub libraries: serde_json::Value,
}

/// Request to call a Redis function.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct FunctionCallRequest {
    #[validate(length(min = 1, message = "Function name is required"))]
    pub function: String,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub args: Vec<serde_json::Value>,
    #[serde(default)]
    pub readonly: bool,
}

/// Response from `FCALL` or `FCALL_RO`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionCallResponse {
    pub result: serde_json::Value,
}

/// Response for `FUNCTION DUMP` (base64-encoded binary).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionDumpResponse {
    pub data: String,
}

/// Restore policy for `FUNCTION RESTORE`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FunctionRestorePolicySchema {
    Append,
    Flush,
    Replace,
}

/// Request for `FUNCTION RESTORE`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct FunctionRestoreRequest {
    #[validate(length(min = 1, message = "Data cannot be empty"))]
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<FunctionRestorePolicySchema>,
}

/// Response for `FUNCTION STATS`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionStatsResponse {
    pub stats: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_load_request_validation() {
        let request = FunctionLoadRequest {
            code: "".to_string(),
            replace: false,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_function_call_request_validation() {
        let request = FunctionCallRequest {
            function: "".to_string(),
            keys: vec![],
            args: vec![],
            readonly: false,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_function_list_query_defaults() {
        let query: FunctionListQuery = serde_json::from_str("{}").expect("query");
        assert!(!query.with_code);
    }
}
