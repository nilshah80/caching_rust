//! Error Handler Middleware
//!
//! Global error handling for unhandled errors.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

use crate::domain::errors::ErrorResponse;
use crate::shared::request_context;

/// Handle unhandled errors and convert to proper responses
pub async fn error_handler(error: Box<dyn std::error::Error + Send + Sync>) -> Response {
    error!(error = %error, "Unhandled error");

    let response = ErrorResponse {
        success: false,
        timestamp: chrono::Utc::now(),
        request_id: request_context::current_request_id(),
        error: crate::domain::errors::ErrorDetail {
            code: "INTERNAL_ERROR".to_string(),
            message: "An unexpected error occurred".to_string(),
            details: None,
        },
    };

    (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_error_handler_response() {
        let error: Box<dyn std::error::Error + Send + Sync> = "boom".to_string().into();
        let response = error_handler(error).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
