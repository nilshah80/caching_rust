//! Request ID Middleware
//!
//! Adds unique request IDs to all requests for tracing.

use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

/// Header name for request ID
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Middleware to add request ID to requests
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    // Check if request already has an ID, otherwise generate one
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map_or_else(|| Uuid::new_v4().to_string(), String::from);

    // Add to request extensions for handlers to access
    request.extensions_mut().insert(RequestId(request_id.clone()));

    // Process request
    let mut response = next.run(request).await;

    // Add request ID to response headers - UUID strings are always valid ASCII
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, header_value);
    }

    response
}

/// Request ID extension
#[derive(Clone, Debug)]
pub struct RequestId(pub String);
