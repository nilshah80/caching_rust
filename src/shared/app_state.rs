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

        Self::new_with_services(pool, config, capabilities, string_service, admin_service)
    }

    /// Create new application state with custom services (useful for testing)
    pub fn new_with_services(
        pool: Arc<InstrumentedPool>,
        config: Arc<Settings>,
        capabilities: Arc<RedisCapabilities>,
        string_service: Arc<StringService>,
        admin_service: Arc<AdminService>,
    ) -> Self {
        Self {
            pool,
            config,
            capabilities,
            string_service,
            admin_service,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockAdminRepository, MockStringRepository};

    #[test]
    fn test_new_with_services() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let config = Arc::new(Settings::default());
        let capabilities = Arc::new(RedisCapabilities::default_capabilities());
        let string_service = Arc::new(StringService::new_with_repository(Arc::new(MockStringRepository::new())));
        let admin_service = Arc::new(AdminService::new_with_repository(Arc::new(MockAdminRepository::default())));

        let state = AppState::new_with_services(
            pool.clone(),
            config.clone(),
            capabilities.clone(),
            string_service.clone(),
            admin_service.clone(),
        );

        assert_eq!(state.config.admin.api_key, config.admin.api_key);
        assert_eq!(state.pool.get_stats().max_size, 1);
    }

    #[test]
    fn test_new_state() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let config = Arc::new(Settings::default());
        let capabilities = Arc::new(RedisCapabilities::default_capabilities());

        let state = AppState::new(pool.clone(), config.clone(), capabilities.clone());

        assert_eq!(state.config.admin.api_key, config.admin.api_key);
        assert_eq!(state.pool.get_stats().max_size, 1);
    }
}
