//! HTTP Server Setup
//!
//! Axum server configuration with middleware and graceful shutdown.

#[cfg(not(test))]
use std::time::Duration;

#[cfg(not(test))]
use tokio::net::TcpListener;
#[cfg(not(test))]
use tokio::signal;
#[cfg(not(test))]
use tower::ServiceBuilder;
#[cfg(not(test))]
use tower_http::cors::{Any, CorsLayer};
#[cfg(not(test))]
use axum::http::StatusCode;
#[cfg(not(test))]
use tower_http::timeout::TimeoutLayer;
#[cfg(not(test))]
use tower_http::trace::TraceLayer;
#[cfg(not(test))]
use tracing::{info, warn};

#[cfg(not(test))]
use crate::api::http::routes::build_router;
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
    // Build the router with all routes
    let app = build_router(state.clone());

    // Add middleware
    let app = app.layer(
        ServiceBuilder::new()
            // Add tracing
            .layer(TraceLayer::new_for_http())
            // Add request timeout (returns 408 Request Timeout on timeout)
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_millis(config.request_timeout_ms),
            ))
            // Add CORS
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            ),
    );

    // Create listener
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;

    info!(address = %addr, "Starting HTTP server");

    // Run with graceful shutdown
    axum::serve(listener, app)
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
        let (state, _, _) = test_state();
        let config = ServerConfig::default();
        run(state, &config).await.expect("run");
    }

    #[tokio::test]
    async fn test_shutdown_signal_stub() {
        shutdown_signal().await;
    }
}
