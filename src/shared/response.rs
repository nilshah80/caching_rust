//! Common Response Types
//!
//! Standard response wrappers for API endpoints.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
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
