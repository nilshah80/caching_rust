//! Logging Middleware
//!
//! Structured logging for HTTP requests and responses.

use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
#[cfg(not(test))]
use tracing::{info, warn};

use super::request_id::RequestId;

/// Middleware for structured request/response logging
#[cfg_attr(test, allow(unused_variables))]
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
    #[cfg(not(test))]
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
        #[cfg(not(test))]
        info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request completed"
        );
    } else if status.is_client_error() {
        #[cfg(not(test))]
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Client error"
        );
    } else {
        #[cfg(not(test))]
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, StatusCode};
    use axum::{Router, middleware, routing::get};
    use tower::ServiceExt;

    async fn ok_handler() -> StatusCode {
        StatusCode::OK
    }

    async fn bad_request_handler() -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    async fn error_handler() -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    #[tokio::test]
    async fn test_logging_middleware_branches() {
        let app = Router::new()
            .route("/ok", get(ok_handler))
            .route("/bad", get(bad_request_handler))
            .route("/err", get(error_handler))
            .layer(middleware::from_fn(logging_middleware));

        let ok = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/ok")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("ok response");
        assert_eq!(ok.status(), StatusCode::OK);

        let bad = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/bad")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("bad response");
        assert_eq!(bad.status(), StatusCode::BAD_REQUEST);

        let err = app
            .oneshot(
                Request::builder()
                    .uri("/err")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("err response");
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
