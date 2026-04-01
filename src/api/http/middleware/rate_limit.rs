//! Rate Limiting Middleware
//!
//! Per-IP token bucket rate limiter using the `governor` crate.
//! Each client IP gets its own independent rate limit bucket.
//! Returns 429 Too Many Requests when the rate limit is exceeded.

use axum::extract::ConnectInfo;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::DefaultClock;
use governor::state::keyed::DashMapStateStore;
use governor::{Quota, RateLimiter};
use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Combined state for the rate limiter: the limiter itself plus whether to trust proxy headers.
#[derive(Clone)]
pub struct RateLimitState {
    pub limiter: Arc<RateLimiter<IpAddr, DashMapStateStore<IpAddr>, DefaultClock>>,
    pub trust_proxy: bool,
}

/// Per-IP rate limiter state keyed by client IP address.
pub type SharedRateLimiter = Arc<RateLimiter<IpAddr, DashMapStateStore<IpAddr>, DefaultClock>>;

/// Create a new per-IP rate limiter with the given RPS and burst size.
pub fn create_rate_limiter(rps: u64, burst: u32) -> SharedRateLimiter {
    let quota = Quota::per_second(NonZeroU32::new(rps as u32).unwrap_or(NonZeroU32::MIN))
        .allow_burst(NonZeroU32::new(burst).unwrap_or(NonZeroU32::MIN));
    Arc::new(RateLimiter::dashmap(quota))
}

/// Extract client IP from request.
///
/// When `trust_proxy` is true, checks X-Forwarded-For first (for trusted reverse proxies),
/// then falls back to the socket address.
/// When `trust_proxy` is false, only uses the socket address — never trusts client-supplied headers.
fn extract_client_ip(request: &Request, trust_proxy: bool) -> IpAddr {
    // Only check X-Forwarded-For when behind a trusted reverse proxy
    if trust_proxy {
        if let Some(forwarded) = request.headers().get("x-forwarded-for")
            && let Ok(value) = forwarded.to_str()
            && let Some(first_ip) = value.split(',').next()
            && let Ok(ip) = first_ip.trim().parse::<IpAddr>()
        {
            return ip;
        }
    }

    // Use the actual TCP socket address (requires into_make_service_with_connect_info)
    request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]))
}

/// Middleware that enforces a per-IP rate limit.
/// Must be used with `axum::middleware::from_fn_with_state`.
pub async fn rate_limit_middleware(
    axum::extract::State(state): axum::extract::State<RateLimitState>,
    request: Request,
    next: Next,
) -> Response {
    let client_ip = extract_client_ip(&request, state.trust_proxy);

    match state.limiter.check_key(&client_ip) {
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
        let ip = IpAddr::from([1, 2, 3, 4]);
        assert!(limiter.check_key(&ip).is_ok());
    }

    #[test]
    fn test_rate_limiter_burst_per_ip() {
        let limiter = create_rate_limiter(10, 5);
        let ip1 = IpAddr::from([1, 1, 1, 1]);
        let ip2 = IpAddr::from([2, 2, 2, 2]);

        // Exhaust IP1's bucket
        for _ in 0..5 {
            assert!(limiter.check_key(&ip1).is_ok());
        }
        assert!(limiter.check_key(&ip1).is_err());

        // IP2 should still have its own bucket
        assert!(limiter.check_key(&ip2).is_ok());
    }

    fn test_state(limiter: SharedRateLimiter) -> RateLimitState {
        RateLimitState {
            limiter,
            trust_proxy: false,
        }
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_allows_request() {
        let limiter = create_rate_limiter(10, 5);
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(axum_mw::from_fn_with_state(
                test_state(limiter),
                rate_limit_middleware,
            ));

        let resp = app
            .oneshot(Request::get("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_returns_429() {
        let limiter = create_rate_limiter(10, 1); // burst of 1
        let ip = IpAddr::from([127, 0, 0, 1]);
        // Exhaust the single token for 127.0.0.1 (default fallback IP)
        limiter.check_key(&ip).unwrap();

        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(axum_mw::from_fn_with_state(
                test_state(limiter),
                rate_limit_middleware,
            ));

        let resp = app
            .oneshot(Request::get("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(resp.headers().contains_key("retry-after"));
    }
}
