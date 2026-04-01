//! Request ID Middleware
//!
//! Adds unique request IDs to all requests for tracing.

use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use uuid::Uuid;

use crate::shared::request_context;

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
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    // Process request inside a request-scoped context so response builders
    // and error conversion can access the active request ID.
    let mut response =
        request_context::scope_request_id(request_id.clone(), next.run(request)).await;

    // Add request ID to response headers - UUID strings are always valid ASCII
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(REQUEST_ID_HEADER, header_value);
    }

    response
}

/// Request ID extension
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, StatusCode};
    use axum::{Router, middleware, routing::get};
    use tower::ServiceExt;

    async fn ok_handler() -> StatusCode {
        StatusCode::OK
    }

    #[tokio::test]
    async fn test_request_id_generated() {
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn(request_id_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");

        assert!(response.headers().contains_key(REQUEST_ID_HEADER));
    }

    #[tokio::test]
    async fn test_request_id_preserved() {
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn(request_id_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(REQUEST_ID_HEADER, "fixed-id")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");

        assert_eq!(
            response
                .headers()
                .get(REQUEST_ID_HEADER)
                .and_then(|v| v.to_str().ok()),
            Some("fixed-id")
        );
    }
}
