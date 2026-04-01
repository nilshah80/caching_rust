//! HTTP Server Setup
//!
//! Axum server configuration with middleware and graceful shutdown.

#[cfg(not(test))]
use std::net::SocketAddr;
#[cfg(not(test))]
use std::time::Duration;

#[cfg(not(test))]
use axum::Router;
#[cfg(not(test))]
use axum::extract::DefaultBodyLimit;
#[cfg(not(test))]
use axum::http::StatusCode;
#[cfg(not(test))]
use axum::http::{HeaderValue, Method, header};
#[cfg(not(test))]
use tokio::net::TcpListener;
#[cfg(not(test))]
use tokio::signal;
#[cfg(not(test))]
use tower::ServiceBuilder;
#[cfg(not(test))]
use tower_http::cors::CorsLayer;
#[cfg(not(test))]
use tower_http::set_header::SetResponseHeaderLayer;
#[cfg(not(test))]
use tower_http::timeout::TimeoutLayer;
#[cfg(not(test))]
use tower_http::trace::TraceLayer;
#[cfg(not(test))]
use tracing::{info, warn};

#[cfg(not(test))]
use axum::middleware as axum_mw;

#[cfg(not(test))]
use crate::api::http::middleware::{
    logging_middleware, metrics_middleware, request_id_middleware,
};
#[cfg(not(test))]
use crate::api::http::middleware::rate_limit::{RateLimitState, create_rate_limiter, rate_limit_middleware};
#[cfg(not(test))]
use crate::api::http::routes::{build_router, operational_routes};
use crate::infrastructure::config::ServerConfig;
use crate::shared::app_state::AppState;

/// Run the HTTP server
///
/// # Errors
///
/// Returns an error if:
/// - The TCP listener fails to bind to the specified address
/// - The server encounters a fatal error during operation
#[cfg(not(test))]
pub async fn run(state: AppState, config: &ServerConfig) -> anyhow::Result<()> {
    // Build API routes (rate-limited) and operational routes (exempt)
    let mut api_router = build_router(state.clone());
    let rate_limit = &state.config.rate_limit;
    if rate_limit.enabled {
        let limiter = create_rate_limiter(rate_limit.requests_per_second, rate_limit.burst_size);
        let rate_limit_state = RateLimitState {
            limiter,
            trust_proxy: config.trust_proxy,
        };
        api_router =
            api_router.layer(axum_mw::from_fn_with_state(rate_limit_state, rate_limit_middleware));
    }

    // Health/metrics/readiness are never rate-limited — Kubernetes probes
    // and Prometheus scrapes must always succeed, especially under load.
    let app = Router::new()
        .merge(operational_routes().with_state(state.clone()))
        .merge(api_router);

    // Add middleware
    let app = app
        .layer(axum_mw::from_fn(metrics_middleware))
        .layer(axum_mw::from_fn(logging_middleware))
        .layer(axum_mw::from_fn(request_id_middleware))
        .layer(
            ServiceBuilder::new()
            // Add request body size limit (prevents OOM from large payloads)
            .layer(DefaultBodyLimit::max(config.max_body_size_bytes))
            // Add tracing
            .layer(TraceLayer::new_for_http())
            // Add request timeout (returns 408 Request Timeout on timeout)
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_millis(config.request_timeout_ms),
            ))
            // Add CORS (origin is configurable via SERVER__CORS_ORIGINS)
            .layer({
                let cors = CorsLayer::new()
                    .allow_methods([
                        Method::GET,
                        Method::POST,
                        Method::PUT,
                        Method::PATCH,
                        Method::DELETE,
                        Method::OPTIONS,
                    ])
                    .allow_headers([
                        header::CONTENT_TYPE,
                        header::AUTHORIZATION,
                        header::ACCEPT,
                        header::HeaderName::from_static("x-admin-api-key"),
                        header::HeaderName::from_static("x-request-id"),
                    ]);

                if config.cors_origins == "*" {
                    cors.allow_origin(tower_http::cors::Any)
                } else {
                    let origins: Vec<HeaderValue> = config
                        .cors_origins
                        .split(',')
                        .filter_map(|o| o.trim().parse().ok())
                        .collect();
                    cors.allow_origin(origins)
                }
            })
            // Security headers
            .layer(SetResponseHeaderLayer::overriding(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ))
            .layer(SetResponseHeaderLayer::overriding(
                header::HeaderName::from_static("x-frame-options"),
                HeaderValue::from_static("DENY"),
            ))
            .layer(SetResponseHeaderLayer::overriding(
                header::HeaderName::from_static("x-xss-protection"),
                HeaderValue::from_static("1; mode=block"),
            ))
            .layer(SetResponseHeaderLayer::overriding(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-store"),
            )),
        );

    if config.cors_origins == "*" {
        warn!("CORS allows any origin — set SERVER__CORS_ORIGINS for production");
    }

    info!(
        max_body_size_mb = config.max_body_size_bytes / 1024 / 1024,
        max_batch_size = config.max_batch_size,
        rate_limit_rps = state.config.rate_limit.requests_per_second,
        rate_limit_burst = state.config.rate_limit.burst_size,
        rate_limit_enabled = state.config.rate_limit.enabled,
        cors_origins = %config.cors_origins,
        "Request limits configured"
    );

    // Create listener
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;

    info!(address = %addr, "Starting HTTP server");

    // Run with graceful shutdown.
    // Use into_make_service_with_connect_info so ConnectInfo<SocketAddr> is available
    // in middleware/handlers for accurate per-IP rate limiting.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Wait for shutdown signal
#[allow(clippy::expect_used)] // Signal handlers must be installed; panic on failure is acceptable
#[cfg(not(test))]
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            warn!("Received Ctrl+C, initiating graceful shutdown");
        }
        () = terminate => {
            warn!("Received terminate signal, initiating graceful shutdown");
        }
    }
}

#[cfg(test)]
pub async fn run(_state: AppState, _config: &ServerConfig) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
async fn shutdown_signal() {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state;

    #[tokio::test]
    async fn test_run_stub() {
        let (state, _, _, _) = test_state();
        let config = ServerConfig::default();
        run(state, &config).await.expect("run");
    }

    #[tokio::test]
    async fn test_shutdown_signal_stub() {
        shutdown_signal().await;
    }
}
