//! Redis Functions routes.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use validator::Validate;

use crate::api::http::middleware::admin_auth::{ADMIN_API_KEY_HEADER, validate_admin_key};
use crate::api::http::schemas::functions::{
    FunctionCallRequest, FunctionCallResponse, FunctionDumpResponse, FunctionFlushModeSchema,
    FunctionFlushRequest, FunctionListQuery, FunctionListResponse, FunctionLoadRequest,
    FunctionLoadResponse, FunctionRestorePolicySchema, FunctionRestoreRequest,
    FunctionStatsResponse, FunctionSuccessResponse,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::{FunctionFlushMode, FunctionRestorePolicy};
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

pub fn functions_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/functions", get(function_list))
        .route("/api/v1/functions/load", post(function_load))
        .route("/api/v1/functions/flush", post(function_flush))
        .route("/api/v1/functions/call", post(function_call))
        .route("/api/v1/functions/dump", get(function_dump))
        .route("/api/v1/functions/restore", post(function_restore))
        .route("/api/v1/functions/stats", get(function_stats))
        .route("/api/v1/functions/kill", post(function_kill))
        .route("/api/v1/functions/{name}", delete(function_delete))
}

fn require_functions(state: &AppState) -> Result<(), CacheError> {
    if !state.capabilities.features.functions {
        return Err(CacheError::ModuleNotAvailable(
            "Redis Functions require Redis 7.0+".to_string(),
        ));
    }
    Ok(())
}

/// Verify admin API key from request headers.
/// All function endpoints require admin auth because they allow server-side code execution.
fn require_admin(headers: &HeaderMap, state: &AppState) -> Result<(), CacheError> {
    let token = headers
        .get(ADMIN_API_KEY_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or(CacheError::Unauthorized)?;
    validate_admin_key(state, token)
}

#[utoipa::path(
    get,
    path = "/api/v1/functions",
    params(("with_code" = Option<bool>, Query, description = "Include library source code")),
    responses(
        (status = 200, description = "Function libraries", body = FunctionListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 501, description = "Redis Functions not available")
    ),
    security(("api_key" = [])),
    tag = "Functions"
)]
pub async fn function_list(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<FunctionListQuery>,
) -> Result<Json<ApiResponse<FunctionListResponse>>, CacheError> {
    require_admin(&headers, &state)?;
    require_functions(&state)?;
    let libraries = state
        .function_service
        .function_list(query.with_code)
        .await?;
    Ok(Json(ApiResponse::success(FunctionListResponse {
        libraries,
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/functions/load",
    request_body = FunctionLoadRequest,
    responses(
        (status = 200, description = "Function library loaded", body = FunctionLoadResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 501, description = "Redis Functions not available")
    ),
    security(("api_key" = [])),
    tag = "Functions"
)]
pub async fn function_load(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<FunctionLoadRequest>,
) -> Result<Json<ApiResponse<FunctionLoadResponse>>, CacheError> {
    require_admin(&headers, &state)?;
    require_functions(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let library_name = state
        .function_service
        .function_load(&request.code, request.replace)
        .await?;
    Ok(Json(ApiResponse::success(FunctionLoadResponse {
        library_name,
    })))
}

#[utoipa::path(
    delete,
    path = "/api/v1/functions/{name}",
    params(("name" = String, Path, description = "Function library name")),
    responses(
        (status = 200, description = "Function library deleted", body = FunctionSuccessResponse),
        (status = 401, description = "Unauthorized"),
        (status = 501, description = "Redis Functions not available")
    ),
    security(("api_key" = [])),
    tag = "Functions"
)]
pub async fn function_delete(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<FunctionSuccessResponse>>, CacheError> {
    require_admin(&headers, &state)?;
    require_functions(&state)?;
    state.function_service.function_delete(&name).await?;
    Ok(Json(ApiResponse::success(FunctionSuccessResponse {
        success: true,
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/functions/flush",
    request_body = FunctionFlushRequest,
    responses(
        (status = 200, description = "Functions flushed", body = FunctionSuccessResponse),
        (status = 401, description = "Unauthorized"),
        (status = 501, description = "Redis Functions not available")
    ),
    security(("api_key" = [])),
    tag = "Functions"
)]
pub async fn function_flush(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<FunctionFlushRequest>,
) -> Result<Json<ApiResponse<FunctionSuccessResponse>>, CacheError> {
    require_admin(&headers, &state)?;
    require_functions(&state)?;
    let mode = request.mode.map(|mode| match mode {
        FunctionFlushModeSchema::Async => FunctionFlushMode::Async,
        FunctionFlushModeSchema::Sync => FunctionFlushMode::Sync,
    });
    state.function_service.function_flush(mode).await?;
    Ok(Json(ApiResponse::success(FunctionSuccessResponse {
        success: true,
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/functions/call",
    request_body = FunctionCallRequest,
    responses(
        (status = 200, description = "Function call result", body = FunctionCallResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 501, description = "Redis Functions not available")
    ),
    security(("api_key" = [])),
    tag = "Functions"
)]
pub async fn function_call(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<FunctionCallRequest>,
) -> Result<Json<ApiResponse<FunctionCallResponse>>, CacheError> {
    require_admin(&headers, &state)?;
    require_functions(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let result = state
        .function_service
        .fcall(
            &request.function,
            &request.keys,
            &request.args,
            request.readonly,
        )
        .await?;
    Ok(Json(ApiResponse::success(FunctionCallResponse { result })))
}

#[utoipa::path(
    get,
    path = "/api/v1/functions/dump",
    responses(
        (status = 200, description = "Base64-encoded function dump", body = FunctionDumpResponse),
        (status = 401, description = "Unauthorized"),
        (status = 501, description = "Redis Functions not available")
    ),
    security(("api_key" = [])),
    tag = "Functions"
)]
pub async fn function_dump(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<FunctionDumpResponse>>, CacheError> {
    require_admin(&headers, &state)?;
    require_functions(&state)?;
    let bytes = state.function_service.function_dump().await?;
    let encoded = BASE64.encode(&bytes);
    Ok(Json(ApiResponse::success(FunctionDumpResponse {
        data: encoded,
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/functions/restore",
    request_body = FunctionRestoreRequest,
    responses(
        (status = 200, description = "Functions restored", body = FunctionSuccessResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 501, description = "Redis Functions not available")
    ),
    security(("api_key" = [])),
    tag = "Functions"
)]
pub async fn function_restore(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<FunctionRestoreRequest>,
) -> Result<Json<ApiResponse<FunctionSuccessResponse>>, CacheError> {
    require_admin(&headers, &state)?;
    require_functions(&state)?;
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let bytes = BASE64
        .decode(&request.data)
        .map_err(|e| CacheError::InvalidInput(format!("Invalid base64: {e}")))?;
    let policy = request.policy.map(|p| match p {
        FunctionRestorePolicySchema::Append => FunctionRestorePolicy::Append,
        FunctionRestorePolicySchema::Flush => FunctionRestorePolicy::Flush,
        FunctionRestorePolicySchema::Replace => FunctionRestorePolicy::Replace,
    });
    state
        .function_service
        .function_restore(&bytes, policy)
        .await?;
    Ok(Json(ApiResponse::success(FunctionSuccessResponse {
        success: true,
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/functions/stats",
    responses(
        (status = 200, description = "Function statistics", body = FunctionStatsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 501, description = "Redis Functions not available")
    ),
    security(("api_key" = [])),
    tag = "Functions"
)]
pub async fn function_stats(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<FunctionStatsResponse>>, CacheError> {
    require_admin(&headers, &state)?;
    require_functions(&state)?;
    let stats = state.function_service.function_stats().await?;
    Ok(Json(ApiResponse::success(FunctionStatsResponse { stats })))
}

#[utoipa::path(
    post,
    path = "/api/v1/functions/kill",
    responses(
        (status = 200, description = "Running function killed", body = FunctionSuccessResponse),
        (status = 401, description = "Unauthorized"),
        (status = 501, description = "Redis Functions not available")
    ),
    security(("api_key" = [])),
    tag = "Functions"
)]
pub async fn function_kill(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<FunctionSuccessResponse>>, CacheError> {
    require_admin(&headers, &state)?;
    require_functions(&state)?;
    state.function_service.function_kill().await?;
    Ok(Json(ApiResponse::success(FunctionSuccessResponse {
        success: true,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state_with_function_repo;
    use axum::http::HeaderMap;

    fn admin_headers(api_key: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ADMIN_API_KEY_HEADER, api_key.parse().unwrap());
        headers
    }

    #[test]
    fn test_functions_routes_creation() {
        let _ = functions_routes();
    }

    #[tokio::test]
    async fn test_function_load_rejects_without_admin_key() {
        let (state, _) = test_state_with_function_repo();
        let result = function_load(
            HeaderMap::new(),
            State(state),
            Json(FunctionLoadRequest {
                code: "#!lua name=lib\nreturn 1".to_string(),
                replace: false,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::Unauthorized)));
    }

    #[tokio::test]
    async fn test_function_load_handler() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let response = function_load(
            headers,
            State(state),
            Json(FunctionLoadRequest {
                code: "#!lua name=lib\nredis.register_function('echo', function() return 1 end)"
                    .to_string(),
                replace: false,
            }),
        )
        .await
        .expect("load");
        assert_eq!(response.0.data.expect("data").library_name, "lib");
    }

    #[tokio::test]
    async fn test_function_call_handler() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let response = function_call(
            headers,
            State(state),
            Json(FunctionCallRequest {
                function: "lib.echo".to_string(),
                keys: vec![],
                args: vec![serde_json::json!("hello")],
                readonly: false,
            }),
        )
        .await
        .expect("call");
        assert_eq!(
            response.0.data.expect("data").result,
            serde_json::json!("hello")
        );
    }

    #[tokio::test]
    async fn test_function_list_handler() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let response = function_list(
            headers,
            State(state),
            Query(FunctionListQuery { with_code: true }),
        )
        .await
        .expect("list");
        assert!(response.0.data.expect("data").libraries.is_array());
    }

    #[tokio::test]
    async fn test_function_delete_handler() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let response = function_delete(headers, State(state), Path("lib".to_string()))
            .await
            .expect("delete");
        assert!(response.0.data.expect("data").success);
    }

    #[tokio::test]
    async fn test_function_flush_handler() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let response = function_flush(
            headers,
            State(state),
            Json(FunctionFlushRequest {
                mode: Some(FunctionFlushModeSchema::Sync),
            }),
        )
        .await
        .expect("flush");
        assert!(response.0.data.expect("data").success);
    }

    #[tokio::test]
    async fn test_function_call_readonly_handler() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let response = function_call(
            headers,
            State(state),
            Json(FunctionCallRequest {
                function: "lib.echo".to_string(),
                keys: vec!["k1".to_string()],
                args: vec![serde_json::json!("hello")],
                readonly: true,
            }),
        )
        .await
        .expect("call readonly");
        assert_eq!(
            response.0.data.expect("data").result,
            serde_json::json!("hello")
        );
    }

    #[tokio::test]
    async fn test_function_load_501_when_disabled() {
        let (mut state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let mut caps = (*state.capabilities).clone();
        caps.features.functions = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = function_load(
            headers,
            State(state),
            Json(FunctionLoadRequest {
                code: "#!lua name=lib\nreturn 1".to_string(),
                replace: false,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_function_call_501_when_disabled() {
        let (mut state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let mut caps = (*state.capabilities).clone();
        caps.features.functions = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = function_call(
            headers,
            State(state),
            Json(FunctionCallRequest {
                function: "lib.echo".to_string(),
                keys: vec![],
                args: vec![],
                readonly: false,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_function_delete_501_when_disabled() {
        let (mut state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let mut caps = (*state.capabilities).clone();
        caps.features.functions = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = function_delete(headers, State(state), Path("lib".to_string())).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_function_flush_501_when_disabled() {
        let (mut state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let mut caps = (*state.capabilities).clone();
        caps.features.functions = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = function_flush(
            headers,
            State(state),
            Json(FunctionFlushRequest { mode: None }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_function_flush_async_mode() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let response = function_flush(
            headers,
            State(state),
            Json(FunctionFlushRequest {
                mode: Some(FunctionFlushModeSchema::Async),
            }),
        )
        .await
        .expect("flush async");
        assert!(response.0.data.expect("data").success);
    }

    #[tokio::test]
    async fn test_function_flush_no_mode() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let response = function_flush(
            headers,
            State(state),
            Json(FunctionFlushRequest { mode: None }),
        )
        .await
        .expect("flush no mode");
        assert!(response.0.data.expect("data").success);
    }

    #[tokio::test]
    async fn test_function_list_501_when_disabled() {
        let (mut state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let mut caps = (*state.capabilities).clone();
        caps.features.functions = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result =
            function_list(headers, State(state), Query(FunctionListQuery::default())).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_function_dump_handler() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let response = function_dump(headers, State(state)).await.expect("dump");
        let data = response.0.data.expect("data").data;
        let decoded = BASE64.decode(&data).expect("decode base64");
        assert_eq!(decoded, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_function_dump_501_when_disabled() {
        let (mut state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let mut caps = (*state.capabilities).clone();
        caps.features.functions = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = function_dump(headers, State(state)).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_function_restore_handler() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let encoded = BASE64.encode([1, 2, 3]);
        let response = function_restore(
            headers,
            State(state),
            Json(FunctionRestoreRequest {
                data: encoded,
                policy: Some(FunctionRestorePolicySchema::Replace),
            }),
        )
        .await
        .expect("restore");
        assert!(response.0.data.expect("data").success);
    }

    #[tokio::test]
    async fn test_function_restore_append_and_flush_policies() {
        for policy in [
            FunctionRestorePolicySchema::Append,
            FunctionRestorePolicySchema::Flush,
        ] {
            let (state, _) = test_state_with_function_repo();
            let headers = admin_headers(&state.config.admin.api_key);
            let response = function_restore(
                headers,
                State(state),
                Json(FunctionRestoreRequest {
                    data: BASE64.encode([1, 2, 3]),
                    policy: Some(policy),
                }),
            )
            .await
            .expect("restore");
            assert!(response.0.data.expect("data").success);
        }
    }

    #[tokio::test]
    async fn test_function_restore_501_when_disabled() {
        let (mut state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let mut caps = (*state.capabilities).clone();
        caps.features.functions = false;
        state.capabilities = std::sync::Arc::new(caps);
        let encoded = BASE64.encode([1, 2, 3]);
        let result = function_restore(
            headers,
            State(state),
            Json(FunctionRestoreRequest {
                data: encoded,
                policy: None,
            }),
        )
        .await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_function_stats_handler() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let response = function_stats(headers, State(state)).await.expect("stats");
        assert_eq!(
            response.0.data.expect("data").stats,
            serde_json::json!({"running_script": null})
        );
    }

    #[tokio::test]
    async fn test_function_stats_501_when_disabled() {
        let (mut state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let mut caps = (*state.capabilities).clone();
        caps.features.functions = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = function_stats(headers, State(state)).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_function_kill_handler() {
        let (state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let response = function_kill(headers, State(state)).await.expect("kill");
        assert!(response.0.data.expect("data").success);
    }

    #[tokio::test]
    async fn test_function_kill_501_when_disabled() {
        let (mut state, _) = test_state_with_function_repo();
        let headers = admin_headers(&state.config.admin.api_key);
        let mut caps = (*state.capabilities).clone();
        caps.features.functions = false;
        state.capabilities = std::sync::Arc::new(caps);
        let result = function_kill(headers, State(state)).await;
        assert!(matches!(result, Err(CacheError::ModuleNotAvailable(_))));
    }
}
