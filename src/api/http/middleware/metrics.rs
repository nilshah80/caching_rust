//! Prometheus Metrics Middleware
//!
//! Records HTTP request count, duration, and status for every request.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use std::time::Instant;

use crate::infrastructure::metrics::record_http_request;

/// Middleware that records Prometheus metrics for every HTTP request.
pub async fn metrics_middleware(request: Request, next: Next) -> impl IntoResponse {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let start = Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed();

    let status = response.status().as_u16();
    record_http_request(&method, &normalize_path(&path), status, duration);

    response
}

/// Normalize dynamic path segments to reduce metric cardinality.
/// e.g. `/api/v1/strings/my-key` → `/api/v1/strings/:key`
fn normalize_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 3 {
        return path.to_string();
    }

    // Known API prefixes that have a dynamic key segment
    let api_prefixes = [
        "strings", "hashes", "lists", "sets", "sorted-sets", "bitmaps",
        "geo", "json", "bloom", "streams", "keys", "timeseries",
        "cms", "topk", "hyperloglog",
    ];

    let mut normalized = Vec::with_capacity(parts.len());
    let mut next_is_key = false;

    for part in &parts {
        if next_is_key && !part.is_empty() {
            normalized.push(":key");
            next_is_key = false;
            continue;
        }
        next_is_key = api_prefixes.contains(part);
        normalized.push(part);
    }

    normalized.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_static() {
        assert_eq!(normalize_path("/health"), "/health");
        assert_eq!(normalize_path("/health/ready"), "/health/ready");
    }

    #[test]
    fn test_normalize_path_dynamic_key() {
        assert_eq!(
            normalize_path("/api/v1/strings/my-key"),
            "/api/v1/strings/:key"
        );
        assert_eq!(
            normalize_path("/api/v1/hashes/user:1"),
            "/api/v1/hashes/:key"
        );
    }

    #[test]
    fn test_normalize_path_with_subresource() {
        assert_eq!(
            normalize_path("/api/v1/lists/my-list/range"),
            "/api/v1/lists/:key/range"
        );
        assert_eq!(
            normalize_path("/api/v1/sets/my-set/members"),
            "/api/v1/sets/:key/members"
        );
    }

    #[test]
    fn test_normalize_path_admin_untouched() {
        assert_eq!(
            normalize_path("/api/v1/admin/pool/stats"),
            "/api/v1/admin/pool/stats"
        );
    }

    #[tokio::test]
    async fn test_metrics_middleware_passes_through() {
        use axum::body::Body;
        use axum::http::Request;
        use axum::middleware as axum_mw;
        use axum::routing::get;
        use axum::Router;
        use tower::ServiceExt;

        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(axum_mw::from_fn(metrics_middleware));

        let resp = app
            .oneshot(Request::get("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }
}
