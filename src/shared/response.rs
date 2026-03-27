//! Common Response Types
//!
//! Standard response wrappers for API endpoints.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Standard API success response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// Whether the operation was successful
    pub success: bool,

    /// Response timestamp
    pub timestamp: DateTime<Utc>,

    /// Request ID for tracing (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a success response with data
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            timestamp: Utc::now(),
            request_id: None,
            data: Some(data),
            meta: None,
        }
    }

    /// Create a success response with data and metadata
    pub fn success_with_meta(data: T, meta: serde_json::Value) -> Self {
        Self {
            success: true,
            timestamp: Utc::now(),
            request_id: None,
            data: Some(data),
            meta: Some(meta),
        }
    }

    /// Add request ID to response
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Create a new response (alias for success)
    pub fn new(data: T) -> Self {
        Self::success(data)
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Response for operations that don't return data
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl SuccessResponse {
    /// Create a simple success response
    pub fn new() -> Self {
        Self {
            success: true,
            timestamp: Utc::now(),
            message: None,
        }
    }

    /// Create a success response with message
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            success: true,
            timestamp: Utc::now(),
            message: Some(message.into()),
        }
    }
}

impl Default for SuccessResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoResponse for SuccessResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Response for created resources
#[derive(Debug, Serialize)]
pub struct CreatedResponse<T: Serialize> {
    pub success: bool,
    pub timestamp: DateTime<Utc>,
    pub data: T,
}

impl<T: Serialize> CreatedResponse<T> {
    pub fn new(data: T) -> Self {
        Self {
            success: true,
            timestamp: Utc::now(),
            data,
        }
    }
}

impl<T: Serialize> IntoResponse for CreatedResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_builders() {
        let resp = ApiResponse::success("ok");
        assert!(resp.success);
        assert_eq!(resp.data, Some("ok"));

        let meta = serde_json::json!({"count": 1});
        let resp = ApiResponse::success_with_meta("ok", meta.clone());
        assert_eq!(resp.meta, Some(meta));

        let resp = ApiResponse::new("value").with_request_id("req-1".to_string());
        assert_eq!(resp.request_id.as_deref(), Some("req-1"));
    }

    #[test]
    fn test_api_response_into_response() {
        let response = ApiResponse::success("ok").into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_success_response_builders() {
        let resp = SuccessResponse::new();
        assert!(resp.success);
        assert!(resp.message.is_none());

        let resp = SuccessResponse::with_message("done");
        assert_eq!(resp.message.as_deref(), Some("done"));
    }

    #[test]
    fn test_success_response_default_into_response() {
        let resp = SuccessResponse::default();
        assert!(resp.success);

        let response = resp.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_created_response_into_response() {
        let resp = CreatedResponse::new("data").into_response();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }
}
