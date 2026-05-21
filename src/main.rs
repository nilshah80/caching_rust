//! Redis Caching Service - Entry Point

use redis_caching_service::api::http::server;
use redis_caching_service::infrastructure::config::Settings;
use redis_caching_service::infrastructure::logging;
use redis_caching_service::infrastructure::redis::cluster_connection::ClusterPool;
use redis_caching_service::infrastructure::redis::connection::InstrumentedPool;
use redis_caching_service::infrastructure::redis::sentinel_watcher;
use redis_caching_service::shared::app_state::AppState;

use std::sync::Arc;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let settings = Settings::load()?;

    // Initialize logging
    logging::init(&settings)?;

    let mode = if settings.redis.cluster_enabled {
        "cluster"
    } else if settings.redis.sentinel_enabled {
        "sentinel"
    } else {
        "standalone"
    };

    info!(
        version = env!("CARGO_PKG_VERSION"),
        mode, "Starting Redis Caching Service"
    );

    // Create Redis connection pool (sentinel mode resolves master automatically)
    // For cluster mode, we also create a standalone pool for admin/health commands
    let mut pool = InstrumentedPool::new(&settings.redis, &settings.pool).await?;

    // In cluster mode, create and wire a ClusterPool into InstrumentedPool.
    // This makes pool.get() return cluster-routed connections for data commands,
    // while get_standalone() still returns direct connections for admin/health.
    let cluster_pool = if settings.redis.cluster_enabled {
        let cp =
            ClusterPool::with_timeout_config(&settings.redis, &settings.pool, &settings.blocking)
                .map_err(|e| anyhow::anyhow!("Failed to create cluster pool: {e}"))?;
        info!("Testing cluster connection...");
        cp.get()
            .await
            .map_err(|e| anyhow::anyhow!("Cluster connection failed: {e}"))?;
        info!("Cluster connection established");
        let cp_for_state = Arc::new(cp.clone());
        pool.set_cluster_pool(cp);
        Some(cp_for_state)
    } else {
        None
    };

    // Detect Redis capabilities and store in pool for sentinel failover comparison
    let capabilities = pool.detect_capabilities().await?;
    pool.store_capabilities(Arc::new(capabilities.clone()));
    info!(?capabilities, mode, "Redis capabilities detected");

    // Security warnings and production safeguards
    let is_production = std::env::var("ENVIRONMENT")
        .or_else(|_| std::env::var("ENV"))
        .map(|v| v == "production" || v == "prod")
        .unwrap_or(false);

    if settings.admin.api_key == "changeme-admin-key" {
        if is_production {
            anyhow::bail!(
                "ADMIN__API_KEY is set to the default value — refusing to start in production. \
                 Set a strong, unique API key via ADMIN__API_KEY"
            );
        }
        warn!(
            "Admin API key is set to the default value — change ADMIN__API_KEY before deploying to production"
        );
    }

    if settings.server.cors_origins == "*" && is_production {
        anyhow::bail!(
            "SERVER__CORS_ORIGINS is set to wildcard '*' — refusing to start in production. \
             Set explicit origins via SERVER__CORS_ORIGINS"
        );
    }

    let pool = Arc::new(pool);

    // Start sentinel failover watcher if in sentinel mode
    if settings.redis.sentinel_enabled {
        info!(
            poll_interval_secs = settings.redis.sentinel_poll_interval_secs,
            "Starting sentinel failover watcher"
        );
        sentinel_watcher::spawn_sentinel_watcher(
            pool.clone(),
            settings.redis.clone(),
            settings.pool.clone(),
        );
    }

    // Create application state
    let state = AppState::new_with_cluster(
        pool,
        Arc::new(settings.clone()),
        Arc::new(capabilities),
        cluster_pool,
    );

    // Start HTTP server
    server::run(state, &settings.server).await?;

    Ok(())
}
