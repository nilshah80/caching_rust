//! Rate Limiting Middleware
//!
//! Global token bucket rate limiter using the `governor` crate.
//! Applies a single shared limit across all clients.
//! Returns 429 Too Many Requests when the rate limit is exceeded.

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Shared rate limiter state
pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Create a new rate limiter with the given RPS and burst size.
pub fn create_rate_limiter(rps: u64, burst: u32) -> SharedRateLimiter {
    let quota = Quota::per_second(NonZeroU32::new(rps as u32).unwrap_or(NonZeroU32::MIN))
        .allow_burst(NonZeroU32::new(burst).unwrap_or(NonZeroU32::MIN));
    Arc::new(RateLimiter::direct(quota))
}

/// Middleware that enforces a global rate limit.
/// Must be used with `axum::middleware::from_fn_with_state`.
pub async fn rate_limit_middleware(
    axum::extract::State(limiter): axum::extract::State<SharedRateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    match limiter.check() {
        Ok(_) => next.run(request).await,
        Err(_not_until) => {
            let retry_after = _not_until
                .wait_time_from(governor::clock::Clock::now(
                    &governor::clock::DefaultClock::default(),
                ))
                .as_secs()
                .max(1);
            (
                StatusCode::TOO_MANY_REQUESTS,
                [("retry-after", retry_after.to_string())],
                "rate limit exceeded",
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::middleware as axum_mw;
    use axum::routing::get;
    use tower::ServiceExt;

    #[test]
    fn test_create_rate_limiter() {
        let limiter = create_rate_limiter(100, 50);
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_rate_limiter_burst() {
        let limiter = create_rate_limiter(10, 5);
        for _ in 0..5 {
            assert!(limiter.check().is_ok());
        }
        assert!(limiter.check().is_err());
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_allows_request() {
        let limiter = create_rate_limiter(10, 5);
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(axum_mw::from_fn_with_state(limiter, rate_limit_middleware));

        let resp = app
            .oneshot(Request::get("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_returns_429() {
        let limiter = create_rate_limiter(10, 1); // burst of 1
        // Exhaust the single token
        limiter.check().unwrap();

        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(axum_mw::from_fn_with_state(limiter, rate_limit_middleware));

        let resp = app
            .oneshot(Request::get("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(resp.headers().contains_key("retry-after"));
    }
}
