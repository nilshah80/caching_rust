//! Logging Middleware
//!
//! Structured logging for HTTP requests and responses.

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{info, warn};

use super::request_id::RequestId;

/// Middleware for structured request/response logging
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();

    // Extract request info
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();

    // Get request ID if available
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map_or_else(|| "unknown".to_string(), |r| r.0.clone());

    // Log request
    info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        "Incoming request"
    );

    // Process request
    let response = next.run(request).await;

    // Calculate duration
    let duration = start.elapsed();
    let status = response.status();

    // Log response
    if status.is_success() {
        info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request completed"
        );
    } else if status.is_client_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Client error"
        );
    } else {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Server error"
        );
    }

    response
}
