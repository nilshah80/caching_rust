//! Transaction Routes
//!
//! HTTP endpoints for Redis transaction operations.
//! Implements the single-request bundled transaction model.

use axum::{Json, Router, extract::State, routing::post};
use validator::Validate;

use crate::api::http::schemas::transactions::{
    CompareAndSetRequest, CompareAndSetResponse, HCompareAndSetRequest, TransactionRequest,
    TransactionResponse,
};
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create Transaction routes
pub fn transaction_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/transactions/execute", post(execute))
        .route("/api/v1/transactions/cas", post(compare_and_set))
        .route("/api/v1/transactions/hcas", post(hcompare_and_set))
}

/// POST /api/v1/transactions/execute
///
/// Execute multiple Redis commands atomically within a MULTI/EXEC transaction.
/// Optionally WATCH keys for optimistic locking.
///
/// # Error Codes
/// - 400 Bad Request: Invalid input (empty commands, too many commands/watch keys, invalid command format)
/// - 409 Conflict: Transaction aborted due to WATCH key modification by another client
/// - 500 Internal Server Error: Redis error during transaction execution
/// - 504 Gateway Timeout: Transaction execution exceeded 30 second timeout
#[utoipa::path(
    post,
    path = "/api/v1/transactions/execute",
    tag = "Transactions",
    request_body = TransactionRequest,
    responses(
        (status = 200, description = "Transaction executed successfully", body = TransactionResponse),
        (status = 400, description = "Invalid request - empty commands, too many commands (>100), too many watch keys (>20), or invalid command format"),
        (status = 409, description = "Transaction aborted - watched key was modified by another client (TRANSACTION_ABORTED)"),
        (status = 500, description = "Internal server error - Redis error during transaction execution"),
        (status = 504, description = "Gateway timeout - transaction execution exceeded 30 second timeout")
    )
)]
pub async fn execute(
    State(state): State<AppState>,
    Json(request): Json<TransactionRequest>,
) -> Result<Json<ApiResponse<TransactionResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let response = state.transaction_service.execute(request).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// POST /api/v1/transactions/cas
///
/// Compare-and-set operation for string values.
/// Atomically sets the value only if the current value matches the expected value.
/// This is useful for implementing optimistic locking patterns.
///
/// Uses a Lua script for atomicity, avoiding WATCH race conditions.
/// The response indicates whether the swap succeeded and the current value after the operation.
#[utoipa::path(
    post,
    path = "/api/v1/transactions/cas",
    tag = "Transactions",
    request_body = CompareAndSetRequest,
    responses(
        (status = 200, description = "Compare-and-set executed - check 'swapped' field for success", body = CompareAndSetResponse),
        (status = 400, description = "Invalid request - empty key or script error"),
        (status = 500, description = "Internal server error - Redis connection or execution error")
    )
)]
pub async fn compare_and_set(
    State(state): State<AppState>,
    Json(request): Json<CompareAndSetRequest>,
) -> Result<Json<ApiResponse<CompareAndSetResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let response = state.transaction_service.compare_and_set(request).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// POST /api/v1/transactions/hcas
///
/// Compare-and-set operation for hash field values.
/// Atomically sets the hash field only if the current value matches the expected value.
///
/// Uses a Lua script for atomicity, avoiding WATCH race conditions.
/// The response indicates whether the swap succeeded and the current value after the operation.
#[utoipa::path(
    post,
    path = "/api/v1/transactions/hcas",
    tag = "Transactions",
    request_body = HCompareAndSetRequest,
    responses(
        (status = 200, description = "Hash compare-and-set executed - check 'swapped' field for success", body = CompareAndSetResponse),
        (status = 400, description = "Invalid request - empty key, empty field, or script error"),
        (status = 500, description = "Internal server error - Redis connection or execution error")
    )
)]
pub async fn hcompare_and_set(
    State(state): State<AppState>,
    Json(request): Json<HCompareAndSetRequest>,
) -> Result<Json<ApiResponse<CompareAndSetResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let response = state.transaction_service.hcompare_and_set(request).await?;
    Ok(Json(ApiResponse::success(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http::schemas::transactions::{CommandResult, RedisCommand};
    use crate::infrastructure::config::Settings;
    use crate::test_support::test_state_with_config;

    #[test]
    fn test_transaction_routes_creation() {
        let _routes = transaction_routes();
    }

    #[test]
    fn test_transaction_request_parsing() {
        let json = r#"{
            "commands": [
                {"type": "SET", "key": "test", "value": "hello"},
                {"type": "GET", "key": "test"}
            ]
        }"#;
        let request: TransactionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.commands.len(), 2);
        assert!(request.watch_keys.is_none());
    }

    #[test]
    fn test_transaction_request_with_watch() {
        let json = r#"{
            "watch_keys": ["counter", "lock"],
            "commands": [
                {"type": "INCR", "key": "counter"},
                {"type": "SET", "key": "lock", "value": "acquired"}
            ]
        }"#;
        let request: TransactionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.commands.len(), 2);
        assert_eq!(request.watch_keys.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_cas_request_parsing() {
        let json = r#"{
            "key": "version",
            "expected_value": "1",
            "new_value": "2"
        }"#;
        let request: CompareAndSetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.key, "version");
        assert_eq!(request.expected_value, "1");
        assert_eq!(request.new_value, "2");
    }

    #[test]
    fn test_hcas_request_parsing() {
        let json = r#"{
            "key": "user:1",
            "field": "version",
            "expected_value": "1",
            "new_value": "2"
        }"#;
        let request: HCompareAndSetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.key, "user:1");
        assert_eq!(request.field, "version");
        assert_eq!(request.expected_value, "1");
        assert_eq!(request.new_value, "2");
    }

    #[tokio::test]
    async fn test_execute_validation_error() {
        let (state, _string_repo, _key_repo, _admin_repo) =
            test_state_with_config(Settings::default());
        let request = TransactionRequest {
            watch_keys: None,
            commands: Vec::new(),
        };
        let result = execute(State(state), Json(request)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_execute_service_error_path() {
        let (state, _string_repo, _key_repo, _admin_repo) =
            test_state_with_config(Settings::default());
        let request = TransactionRequest {
            watch_keys: None,
            commands: vec![RedisCommand::Get {
                key: "k".to_string(),
            }],
        };
        let result = execute(State(state), Json(request)).await;
        assert!(matches!(
            result,
            Err(CacheError::PoolError(_))
                | Err(CacheError::ConnectionFailed(_))
                | Err(CacheError::RedisError(_))
        ));
    }

    #[tokio::test]
    async fn test_compare_and_set_validation_error() {
        let (state, _string_repo, _key_repo, _admin_repo) =
            test_state_with_config(Settings::default());
        let request = CompareAndSetRequest {
            key: "".to_string(),
            expected_value: "1".to_string(),
            new_value: "2".to_string(),
        };
        let result = compare_and_set(State(state), Json(request)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_compare_and_set_service_error_path() {
        let (state, _string_repo, _key_repo, _admin_repo) =
            test_state_with_config(Settings::default());
        let request = CompareAndSetRequest {
            key: "version".to_string(),
            expected_value: "1".to_string(),
            new_value: "2".to_string(),
        };
        let result = compare_and_set(State(state), Json(request)).await;
        assert!(matches!(
            result,
            Err(CacheError::PoolError(_))
                | Err(CacheError::ConnectionFailed(_))
                | Err(CacheError::RedisError(_))
        ));
    }

    #[tokio::test]
    async fn test_hcompare_and_set_validation_error() {
        let (state, _string_repo, _key_repo, _admin_repo) =
            test_state_with_config(Settings::default());
        let request = HCompareAndSetRequest {
            key: "".to_string(),
            field: "".to_string(),
            expected_value: "1".to_string(),
            new_value: "2".to_string(),
        };
        let result = hcompare_and_set(State(state), Json(request)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_hcompare_and_set_service_error_path() {
        let (state, _string_repo, _key_repo, _admin_repo) =
            test_state_with_config(Settings::default());
        let request = HCompareAndSetRequest {
            key: "user:1".to_string(),
            field: "version".to_string(),
            expected_value: "1".to_string(),
            new_value: "2".to_string(),
        };
        let result = hcompare_and_set(State(state), Json(request)).await;
        assert!(matches!(
            result,
            Err(CacheError::PoolError(_))
                | Err(CacheError::ConnectionFailed(_))
                | Err(CacheError::RedisError(_))
        ));
    }

    #[test]
    fn test_command_result_serialization() {
        let result = CommandResult {
            index: 0,
            success: true,
            value: Some(serde_json::json!("hello")),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"index\":0"));
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"value\":\"hello\""));
        assert!(!json.contains("error")); // null error should be skipped
    }

    #[test]
    fn test_command_result_with_error() {
        let result = CommandResult {
            index: 1,
            success: false,
            value: None,
            error: Some("WRONGTYPE".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"WRONGTYPE\""));
    }

    #[test]
    fn test_all_command_types() {
        // Test that all command types deserialize correctly
        // Note: SCREAMING_SNAKE_CASE transforms MGet -> M_GET, ZAdd -> Z_ADD, etc.
        let commands = vec![
            r#"{"type": "GET", "key": "k"}"#,
            r#"{"type": "SET", "key": "k", "value": "v"}"#,
            r#"{"type": "SET", "key": "k", "value": "v", "ttl_seconds": 60}"#,
            r#"{"type": "INCR", "key": "k"}"#,
            r#"{"type": "INCR_BY", "key": "k", "delta": 5}"#,
            r#"{"type": "DECR", "key": "k"}"#,
            r#"{"type": "DECR_BY", "key": "k", "delta": 5}"#,
            r#"{"type": "APPEND", "key": "k", "value": "v"}"#,
            r#"{"type": "SET_NX", "key": "k", "value": "v"}"#,
            r#"{"type": "GET_SET", "key": "k", "value": "v"}"#,
            r#"{"type": "M_GET", "keys": ["k1", "k2"]}"#,
            r#"{"type": "M_SET", "entries": [{"key": "k1", "value": "v1"}]}"#,
            r#"{"type": "H_GET", "key": "k", "field": "f"}"#,
            r#"{"type": "H_SET", "key": "k", "field": "f", "value": "v"}"#,
            r#"{"type": "H_M_SET", "key": "k", "fields": [{"field": "f", "value": "v"}]}"#,
            r#"{"type": "H_M_GET", "key": "k", "fields": ["f1", "f2"]}"#,
            r#"{"type": "H_INCR_BY", "key": "k", "field": "f", "delta": 1}"#,
            r#"{"type": "H_INCR_BY_FLOAT", "key": "k", "field": "f", "delta": 1.5}"#,
            r#"{"type": "H_DEL", "key": "k", "fields": ["f"]}"#,
            r#"{"type": "H_EXISTS", "key": "k", "field": "f"}"#,
            r#"{"type": "H_GET_ALL", "key": "k"}"#,
            r#"{"type": "H_KEYS", "key": "k"}"#,
            r#"{"type": "H_VALS", "key": "k"}"#,
            r#"{"type": "H_LEN", "key": "k"}"#,
            r#"{"type": "H_SET_NX", "key": "k", "field": "f", "value": "v"}"#,
            r#"{"type": "L_PUSH", "key": "k", "values": ["v"]}"#,
            r#"{"type": "R_PUSH", "key": "k", "values": ["v"]}"#,
            r#"{"type": "L_POP", "key": "k"}"#,
            r#"{"type": "L_POP", "key": "k", "count": 2}"#,
            r#"{"type": "R_POP", "key": "k"}"#,
            r#"{"type": "L_LEN", "key": "k"}"#,
            r#"{"type": "L_INDEX", "key": "k", "index": 0}"#,
            r#"{"type": "L_RANGE", "key": "k", "start": 0, "stop": -1}"#,
            r#"{"type": "L_SET", "key": "k", "index": 0, "value": "v"}"#,
            r#"{"type": "L_TRIM", "key": "k", "start": 0, "stop": 10}"#,
            r#"{"type": "L_REM", "key": "k", "count": 1, "value": "v"}"#,
            r#"{"type": "S_ADD", "key": "k", "members": ["m"]}"#,
            r#"{"type": "S_REM", "key": "k", "members": ["m"]}"#,
            r#"{"type": "S_IS_MEMBER", "key": "k", "member": "m"}"#,
            r#"{"type": "S_MEMBERS", "key": "k"}"#,
            r#"{"type": "S_CARD", "key": "k"}"#,
            r#"{"type": "S_POP", "key": "k"}"#,
            r#"{"type": "S_POP", "key": "k", "count": 2}"#,
            r#"{"type": "S_MOVE", "source": "s", "destination": "d", "member": "m"}"#,
            r#"{"type": "Z_ADD", "key": "k", "members": [{"score": 1.0, "member": "m"}]}"#,
            r#"{"type": "Z_REM", "key": "k", "members": ["m"]}"#,
            r#"{"type": "Z_INCR_BY", "key": "k", "delta": 1.0, "member": "m"}"#,
            r#"{"type": "Z_SCORE", "key": "k", "member": "m"}"#,
            r#"{"type": "Z_RANK", "key": "k", "member": "m"}"#,
            r#"{"type": "Z_REV_RANK", "key": "k", "member": "m"}"#,
            r#"{"type": "Z_CARD", "key": "k"}"#,
            r#"{"type": "Z_COUNT", "key": "k", "min": "-inf", "max": "+inf"}"#,
            r#"{"type": "Z_RANGE", "key": "k", "start": 0, "stop": -1}"#,
            r#"{"type": "Z_RANGE", "key": "k", "start": 0, "stop": -1, "with_scores": true}"#,
            r#"{"type": "Z_REV_RANGE", "key": "k", "start": 0, "stop": -1}"#,
            r#"{"type": "DEL", "keys": ["k1", "k2"]}"#,
            r#"{"type": "EXISTS", "keys": ["k1", "k2"]}"#,
            r#"{"type": "EXPIRE", "key": "k", "seconds": 60}"#,
            r#"{"type": "P_EXPIRE", "key": "k", "milliseconds": 1000}"#,
            r#"{"type": "TTL", "key": "k"}"#,
            r#"{"type": "P_TTL", "key": "k"}"#,
            r#"{"type": "PERSIST", "key": "k"}"#,
            r#"{"type": "RENAME", "key": "k", "new_key": "k2"}"#,
            r#"{"type": "RENAME_NX", "key": "k", "new_key": "k2"}"#,
            r#"{"type": "TYPE", "key": "k"}"#,
        ];

        for (i, cmd_json) in commands.iter().enumerate() {
            let result: Result<RedisCommand, _> = serde_json::from_str(cmd_json);
            assert!(
                result.is_ok(),
                "Failed to parse command {}: {}",
                i,
                cmd_json
            );
        }
    }
}
