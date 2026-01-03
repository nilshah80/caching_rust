//! Cache Error Types
//!
//! Domain-specific error types with HTTP status code mapping.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;
use chrono::{DateTime, Utc};
use utoipa::ToSchema;

/// Domain error type for caching operations
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Operation timeout")]
    Timeout,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Pool error: {0}")]
    PoolError(String),

    #[error("Module not available: {0}")]
    ModuleNotAvailable(String),

    #[error("Subscription limit reached")]
    SubscriptionLimitReached,

    #[error("Blocking timeout - no data available")]
    BlockingTimeout,

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Script error: {0}")]
    ScriptError(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Type mismatch: key {key} is of type {actual_type}, expected {expected_type}")]
    TypeMismatch {
        key: String,
        expected_type: String,
        actual_type: String,
    },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl CacheError {
    /// Map error to HTTP status code
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::KeyNotFound(_) => StatusCode::NOT_FOUND,
            Self::InvalidInput(_) => StatusCode::BAD_REQUEST,
            Self::Timeout => StatusCode::GATEWAY_TIMEOUT,
            Self::BlockingTimeout => StatusCode::NO_CONTENT,
            Self::ModuleNotAvailable(_) => StatusCode::NOT_IMPLEMENTED,
            Self::SubscriptionLimitReached => StatusCode::SERVICE_UNAVAILABLE,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::TypeMismatch { .. } => StatusCode::CONFLICT,
            Self::ConnectionFailed(_) | Self::PoolError(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::TransactionFailed(_) => StatusCode::CONFLICT,
            Self::ScriptError(_) => StatusCode::BAD_REQUEST,
            Self::RedisError(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get error code string for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::KeyNotFound(_) => "KEY_NOT_FOUND",
            Self::ConnectionFailed(_) => "CONNECTION_FAILED",
            Self::Timeout => "TIMEOUT",
            Self::InvalidInput(_) => "INVALID_INPUT",
            Self::RedisError(_) => "REDIS_ERROR",
            Self::PoolError(_) => "POOL_ERROR",
            Self::ModuleNotAvailable(_) => "MODULE_NOT_AVAILABLE",
            Self::SubscriptionLimitReached => "SUBSCRIPTION_LIMIT_REACHED",
            Self::BlockingTimeout => "BLOCKING_TIMEOUT",
            Self::TransactionFailed(_) => "TRANSACTION_FAILED",
            Self::ScriptError(_) => "SCRIPT_ERROR",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::TypeMismatch { .. } => "TYPE_MISMATCH",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

/// Error response body structure
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub success: bool,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub error: ErrorDetail,
}

/// Error detail structure
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new error response from a CacheError
    pub fn from_error(error: &CacheError, request_id: Option<String>) -> Self {
        Self {
            success: false,
            timestamp: Utc::now(),
            request_id,
            error: ErrorDetail {
                code: error.error_code().to_string(),
                message: error.to_string(),
                details: None,
            },
        }
    }
}

impl IntoResponse for CacheError {
    fn into_response(self) -> Response {
        let status = self.status_code();

        // For 204 No Content, don't send a body
        if status == StatusCode::NO_CONTENT {
            return status.into_response();
        }

        let body = ErrorResponse::from_error(&self, None);
        (status, Json(body)).into_response()
    }
}

// Conversion from deadpool errors
impl From<deadpool_redis::PoolError> for CacheError {
    fn from(err: deadpool_redis::PoolError) -> Self {
        CacheError::PoolError(err.to_string())
    }
}

impl From<deadpool_redis::CreatePoolError> for CacheError {
    fn from(err: deadpool_redis::CreatePoolError) -> Self {
        CacheError::ConnectionFailed(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            CacheError::KeyNotFound("test".into()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            CacheError::InvalidInput("bad".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            CacheError::BlockingTimeout.status_code(),
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            CacheError::Unauthorized.status_code(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(
            CacheError::KeyNotFound("test".into()).error_code(),
            "KEY_NOT_FOUND"
        );
        assert_eq!(
            CacheError::ModuleNotAvailable("json".into()).error_code(),
            "MODULE_NOT_AVAILABLE"
        );
    }
}
