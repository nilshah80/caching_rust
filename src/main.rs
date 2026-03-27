//! Redis Caching Service - Entry Point

use redis_caching_service::api::http::server;
use redis_caching_service::infrastructure::config::Settings;
use redis_caching_service::infrastructure::logging;
use redis_caching_service::infrastructure::redis::connection::InstrumentedPool;
use redis_caching_service::shared::app_state::AppState;

use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let settings = Settings::load()?;

    // Initialize logging
    logging::init(&settings)?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting Redis Caching Service"
    );

    // Create Redis connection pool
    let pool = InstrumentedPool::new(&settings.redis, &settings.pool).await?;

    // Detect Redis capabilities
    let capabilities = pool.detect_capabilities().await?;
    info!(?capabilities, "Redis capabilities detected");

    // Create application state
    let state = AppState::new(
        Arc::new(pool),
        Arc::new(settings.clone()),
        Arc::new(capabilities),
    );

    // Start HTTP server
    server::run(state, &settings.server).await?;

    Ok(())
}
