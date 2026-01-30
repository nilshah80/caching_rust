//! Scripting Routes
//!
//! HTTP endpoints for Redis Lua scripting operations.

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use validator::Validate;

use crate::api::http::schemas::scripting::{
    EvalRequest, EvalResponse, EvalShaRequest, ScriptDebugRequest, ScriptDebugResponse,
    ScriptExistsRequest, ScriptExistsResponse, ScriptFlushRequest, ScriptFlushResponse,
    ScriptKillResponse, ScriptLoadRequest, ScriptLoadResponse,
};
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create Scripting routes
pub fn scripting_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/scripts/eval", post(eval))
        .route("/api/v1/scripts/evalsha", post(evalsha))
        .route("/api/v1/scripts/load", post(script_load))
        .route("/api/v1/scripts/exists", post(script_exists))
        .route("/api/v1/scripts/flush", post(script_flush))
        .route("/api/v1/scripts/kill", post(script_kill))
        .route("/api/v1/scripts/debug", post(script_debug))
}

/// POST /api/v1/scripts/eval
///
/// Evaluate a Lua script with the provided keys and arguments.
/// Use `readonly: true` for read-only operations (EVAL_RO).
///
/// # Error Codes
/// - 400 Bad Request: Invalid script, too many keys/args, or Lua syntax error
/// - 500 Internal Server Error: Redis connection or execution error
#[utoipa::path(
    post,
    path = "/api/v1/scripts/eval",
    tag = "Scripting",
    request_body = EvalRequest,
    responses(
        (status = 200, description = "Script executed successfully", body = EvalResponse),
        (status = 400, description = "Invalid request - empty script, too many keys/args, or Lua error"),
        (status = 500, description = "Internal server error - Redis connection or execution error")
    )
)]
pub async fn eval(
    State(state): State<AppState>,
    Json(request): Json<EvalRequest>,
) -> Result<Json<ApiResponse<EvalResponse>>, CacheError> {
    request.validate().map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let response = state.scripting_service.eval(request).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// POST /api/v1/scripts/evalsha
///
/// Evaluate a cached script by its SHA1 hash.
/// More efficient than EVAL as the script doesn't need to be transmitted.
/// Use `readonly: true` for read-only operations (EVALSHA_RO).
///
/// # Error Codes
/// - 400 Bad Request: Invalid SHA format or script not in cache (NOSCRIPT)
/// - 500 Internal Server Error: Redis connection or execution error
#[utoipa::path(
    post,
    path = "/api/v1/scripts/evalsha",
    tag = "Scripting",
    request_body = EvalShaRequest,
    responses(
        (status = 200, description = "Cached script executed successfully", body = EvalResponse),
        (status = 400, description = "Invalid request - invalid SHA or script not found (use SCRIPT LOAD first)"),
        (status = 500, description = "Internal server error - Redis connection or execution error")
    )
)]
pub async fn evalsha(
    State(state): State<AppState>,
    Json(request): Json<EvalShaRequest>,
) -> Result<Json<ApiResponse<EvalResponse>>, CacheError> {
    request.validate().map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let response = state.scripting_service.evalsha(request).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// POST /api/v1/scripts/load
///
/// Load a script into the Redis script cache and return its SHA1 hash.
/// The script can then be executed using EVALSHA with the returned hash.
///
/// # Error Codes
/// - 400 Bad Request: Empty script or Lua syntax error
/// - 500 Internal Server Error: Redis connection error
#[utoipa::path(
    post,
    path = "/api/v1/scripts/load",
    tag = "Scripting",
    request_body = ScriptLoadRequest,
    responses(
        (status = 200, description = "Script loaded successfully", body = ScriptLoadResponse),
        (status = 400, description = "Invalid request - empty script or syntax error"),
        (status = 500, description = "Internal server error - Redis connection error")
    )
)]
pub async fn script_load(
    State(state): State<AppState>,
    Json(request): Json<ScriptLoadRequest>,
) -> Result<Json<ApiResponse<ScriptLoadResponse>>, CacheError> {
    request.validate().map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let response = state.scripting_service.script_load(request).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// POST /api/v1/scripts/exists
///
/// Check if one or more scripts exist in the script cache.
///
/// # Error Codes
/// - 400 Bad Request: Empty SHA list or invalid SHA format
/// - 500 Internal Server Error: Redis connection error
#[utoipa::path(
    post,
    path = "/api/v1/scripts/exists",
    tag = "Scripting",
    request_body = ScriptExistsRequest,
    responses(
        (status = 200, description = "Script existence check completed", body = ScriptExistsResponse),
        (status = 400, description = "Invalid request - empty SHA list or invalid SHA format"),
        (status = 500, description = "Internal server error - Redis connection error")
    )
)]
pub async fn script_exists(
    State(state): State<AppState>,
    Json(request): Json<ScriptExistsRequest>,
) -> Result<Json<ApiResponse<ScriptExistsResponse>>, CacheError> {
    request.validate().map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let response = state.scripting_service.script_exists(request).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// POST /api/v1/scripts/flush
///
/// Flush all scripts from the script cache.
/// When no mode is specified, Redis uses its default behavior (SYNC for Redis 6.2+).
/// Explicitly specify ASYNC for non-blocking or SYNC for blocking flush.
///
/// # Error Codes
/// - 500 Internal Server Error: Redis connection error
#[utoipa::path(
    post,
    path = "/api/v1/scripts/flush",
    tag = "Scripting",
    request_body = ScriptFlushRequest,
    responses(
        (status = 200, description = "Script cache flushed successfully", body = ScriptFlushResponse),
        (status = 500, description = "Internal server error - Redis connection error")
    )
)]
pub async fn script_flush(
    State(state): State<AppState>,
    Json(request): Json<ScriptFlushRequest>,
) -> Result<Json<ApiResponse<ScriptFlushResponse>>, CacheError> {
    let response = state.scripting_service.script_flush(request).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// POST /api/v1/scripts/kill
///
/// Kill the currently executing Lua script.
/// Only succeeds if the script has not yet performed any write operations.
///
/// # Error Codes
/// - 400 Bad Request: No script running or script has performed writes
/// - 500 Internal Server Error: Redis connection error
#[utoipa::path(
    post,
    path = "/api/v1/scripts/kill",
    tag = "Scripting",
    responses(
        (status = 200, description = "Running script killed successfully", body = ScriptKillResponse),
        (status = 400, description = "No script running or script has performed writes (cannot be killed)"),
        (status = 500, description = "Internal server error - Redis connection error")
    )
)]
pub async fn script_kill(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ScriptKillResponse>>, CacheError> {
    let response = state.scripting_service.script_kill().await?;
    Ok(Json(ApiResponse::success(response)))
}

/// POST /api/v1/scripts/debug
///
/// Set the script debug mode for Lua debugging.
/// This is primarily for development use.
///
/// # Modes
/// - YES: Enable non-blocking async debugging
/// - SYNC: Enable blocking sync debugging
/// - NO: Disable debugging
///
/// # Warning
/// Debugging should be disabled in production environments.
///
/// # Error Codes
/// - 500 Internal Server Error: Redis connection error
#[utoipa::path(
    post,
    path = "/api/v1/scripts/debug",
    tag = "Scripting",
    request_body = ScriptDebugRequest,
    responses(
        (status = 200, description = "Debug mode set successfully", body = ScriptDebugResponse),
        (status = 500, description = "Internal server error - Redis connection error")
    )
)]
pub async fn script_debug(
    State(state): State<AppState>,
    Json(request): Json<ScriptDebugRequest>,
) -> Result<Json<ApiResponse<ScriptDebugResponse>>, CacheError> {
    let response = state.scripting_service.script_debug(request).await?;
    Ok(Json(ApiResponse::success(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http::schemas::scripting::{
        EvalRequest, EvalShaRequest, FlushMode, ScriptDebugMode, ScriptDebugRequest,
        ScriptExistsRequest, ScriptFlushRequest, ScriptLoadRequest,
    };
    use crate::infrastructure::config::Settings;
    use crate::test_support::test_state_with_config;

    #[test]
    fn test_scripting_routes_creation() {
        let _routes = scripting_routes();
    }

    #[test]
    fn test_eval_request_parsing() {
        let json = r#"{
            "script": "return KEYS[1]",
            "keys": ["key1"],
            "args": ["arg1"]
        }"#;
        let request: EvalRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.script, "return KEYS[1]");
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
    fn test_evalsha_request_parsing() {
        let json = r#"{
            "sha": "6b1bf486c81ceb7edf3c093f4a73d3e117c0b169",
            "keys": [],
            "args": [],
            "readonly": true
        }"#;
        let request: EvalShaRequest = serde_json::from_str(json).unwrap();
        assert!(request.readonly);
    }

    #[test]
    fn test_script_load_request_parsing() {
        let json = r#"{
            "script": "return 1"
        }"#;
        let request: ScriptLoadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.script, "return 1");
    }

    #[test]
    fn test_script_exists_request_parsing() {
        let json = r#"{
            "shas": ["6b1bf486c81ceb7edf3c093f4a73d3e117c0b169"]
        }"#;
        let request: ScriptExistsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.shas.len(), 1);
    }

    #[test]
    fn test_script_flush_request_parsing() {
        let json = r#"{"mode": "SYNC"}"#;
        let request: ScriptFlushRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.mode, Some(FlushMode::Sync)));
    }

    #[test]
    fn test_script_flush_request_default() {
        let json = r#"{}"#;
        let request: ScriptFlushRequest = serde_json::from_str(json).unwrap();
        assert!(request.mode.is_none());
    }

    #[test]
    fn test_script_debug_request_parsing() {
        let json = r#"{"mode": "YES"}"#;
        let request: ScriptDebugRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.mode, ScriptDebugMode::Yes));

        let json = r#"{"mode": "SYNC"}"#;
        let request: ScriptDebugRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.mode, ScriptDebugMode::Sync));

        let json = r#"{"mode": "NO"}"#;
        let request: ScriptDebugRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.mode, ScriptDebugMode::No));
    }

    #[tokio::test]
    async fn test_eval_validation_error() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state_with_config(Settings::default());
        let request = EvalRequest {
            script: "".to_string(),
            keys: Vec::new(),
            args: Vec::new(),
            readonly: false,
        };
        let result = eval(State(state), Json(request)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_eval_service_error_path() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state_with_config(Settings::default());
        let request = EvalRequest {
            script: "return 1".to_string(),
            keys: Vec::new(),
            args: Vec::new(),
            readonly: false,
        };
        let result = eval(State(state), Json(request)).await;
        assert!(matches!(
            result,
            Err(CacheError::PoolError(_)) | Err(CacheError::ConnectionFailed(_)) | Err(CacheError::RedisError(_))
        ));
    }

    #[tokio::test]
    async fn test_evalsha_validation_error() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state_with_config(Settings::default());
        let request = EvalShaRequest {
            sha: "bad".to_string(),
            keys: Vec::new(),
            args: Vec::new(),
            readonly: false,
        };
        let result = evalsha(State(state), Json(request)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_evalsha_service_error_path() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state_with_config(Settings::default());
        let request = EvalShaRequest {
            sha: "6b1bf486c81ceb7edf3c093f4a73d3e117c0b169".to_string(),
            keys: Vec::new(),
            args: Vec::new(),
            readonly: false,
        };
        let result = evalsha(State(state), Json(request)).await;
        assert!(matches!(
            result,
            Err(CacheError::PoolError(_)) | Err(CacheError::ConnectionFailed(_)) | Err(CacheError::RedisError(_))
        ));
    }

    #[tokio::test]
    async fn test_script_load_validation_error() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state_with_config(Settings::default());
        let request = ScriptLoadRequest {
            script: "".to_string(),
        };
        let result = script_load(State(state), Json(request)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_script_load_service_error_path() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state_with_config(Settings::default());
        let request = ScriptLoadRequest {
            script: "return 1".to_string(),
        };
        let result = script_load(State(state), Json(request)).await;
        assert!(matches!(
            result,
            Err(CacheError::PoolError(_)) | Err(CacheError::ConnectionFailed(_)) | Err(CacheError::RedisError(_))
        ));
    }

    #[tokio::test]
    async fn test_script_exists_validation_error() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state_with_config(Settings::default());
        let request = ScriptExistsRequest { shas: Vec::new() };
        let result = script_exists(State(state), Json(request)).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_script_exists_service_error_path() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state_with_config(Settings::default());
        let request = ScriptExistsRequest {
            shas: vec!["6b1bf486c81ceb7edf3c093f4a73d3e117c0b169".to_string()],
        };
        let result = script_exists(State(state), Json(request)).await;
        assert!(matches!(
            result,
            Err(CacheError::PoolError(_)) | Err(CacheError::ConnectionFailed(_)) | Err(CacheError::RedisError(_))
        ));
    }

    #[tokio::test]
    async fn test_script_flush_service_error_path() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state_with_config(Settings::default());
        let request = ScriptFlushRequest {
            mode: Some(FlushMode::Sync),
        };
        let result = script_flush(State(state), Json(request)).await;
        assert!(matches!(
            result,
            Err(CacheError::PoolError(_)) | Err(CacheError::ConnectionFailed(_)) | Err(CacheError::RedisError(_))
        ));
    }

    #[tokio::test]
    async fn test_script_kill_service_error_path() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state_with_config(Settings::default());
        let result = script_kill(State(state)).await;
        assert!(matches!(
            result,
            Err(CacheError::PoolError(_)) | Err(CacheError::ConnectionFailed(_)) | Err(CacheError::RedisError(_)) | Err(CacheError::ScriptError(_))
        ));
    }

    #[tokio::test]
    async fn test_script_debug_service_error_path() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state_with_config(Settings::default());
        let request = ScriptDebugRequest {
            mode: ScriptDebugMode::No,
        };
        let result = script_debug(State(state), Json(request)).await;
        assert!(matches!(
            result,
            Err(CacheError::PoolError(_)) | Err(CacheError::ConnectionFailed(_)) | Err(CacheError::RedisError(_))
        ));
    }
}
