//! HTTP Server Setup
//!
//! Axum server configuration with middleware and graceful shutdown.

use std::time::Duration;

use tokio::net::TcpListener;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use axum::http::StatusCode;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

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
