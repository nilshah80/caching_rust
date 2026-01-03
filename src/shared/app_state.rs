//! Application State
//!
//! Shared state passed to all request handlers.

use std::sync::Arc;

use crate::application::services::{AdminService, StringService};
use crate::infrastructure::config::Settings;
use crate::infrastructure::redis::capabilities::RedisCapabilities;
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// Instrumented Redis connection pool for commands
    pub pool: Arc<InstrumentedPool>,

    /// Application settings
    pub config: Arc<Settings>,

    /// Detected Redis capabilities
    pub capabilities: Arc<RedisCapabilities>,

    /// String operations service
    pub string_service: Arc<StringService>,

    /// Admin operations service
    pub admin_service: Arc<AdminService>,
}

impl AppState {
    /// Create new application state
    pub fn new(
        pool: Arc<InstrumentedPool>,
        config: Arc<Settings>,
        capabilities: Arc<RedisCapabilities>,
    ) -> Self {
        let string_service = Arc::new(StringService::new(pool.clone()));
        let admin_service = Arc::new(AdminService::new(pool.clone()));

        Self {
            pool,
            config,
            capabilities,
            string_service,
            admin_service,
        }
    }
}
