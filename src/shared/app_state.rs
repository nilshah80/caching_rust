//! Application State
//!
//! Shared state passed to all request handlers.

use std::sync::Arc;

use crate::application::services::{AdminService, HashService, KeyService, ListService, SetService, SortedSetService, StringService};
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

    /// Hash operations service
    pub hash_service: Arc<HashService>,

    /// List operations service
    pub list_service: Arc<ListService>,

    /// Set operations service
    pub set_service: Arc<SetService>,

    /// Sorted Set operations service
    pub sorted_set_service: Arc<SortedSetService>,

    /// Key management service
    pub key_service: Arc<KeyService>,

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
        let hash_service = Arc::new(HashService::new(pool.clone()));
        let list_service = Arc::new(ListService::new(pool.clone()));
        let set_service = Arc::new(SetService::new(pool.clone()));
        let sorted_set_service = Arc::new(SortedSetService::new(pool.clone()));
        let key_service = Arc::new(KeyService::new(pool.clone()));
        let admin_service = Arc::new(AdminService::new(pool.clone()));

        Self::new_with_services(pool, config, capabilities, string_service, hash_service, list_service, set_service, sorted_set_service, key_service, admin_service)
    }

    /// Create new application state with custom services (useful for testing)
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_services(
        pool: Arc<InstrumentedPool>,
        config: Arc<Settings>,
        capabilities: Arc<RedisCapabilities>,
        string_service: Arc<StringService>,
        hash_service: Arc<HashService>,
        list_service: Arc<ListService>,
        set_service: Arc<SetService>,
        sorted_set_service: Arc<SortedSetService>,
        key_service: Arc<KeyService>,
        admin_service: Arc<AdminService>,
    ) -> Self {
        Self {
            pool,
            config,
            capabilities,
            string_service,
            hash_service,
            list_service,
            set_service,
            sorted_set_service,
            key_service,
            admin_service,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockAdminRepository, MockHashRepository, MockKeyRepository, MockListRepository, MockSetRepository, MockSortedSetRepository, MockStringRepository};

    #[test]
    fn test_new_with_services() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let config = Arc::new(Settings::default());
        let capabilities = Arc::new(RedisCapabilities::default_capabilities());
        let string_service = Arc::new(StringService::new_with_repository(Arc::new(MockStringRepository::new())));
        let hash_service = Arc::new(HashService::new_with_repository(Arc::new(MockHashRepository::new())));
        let list_service = Arc::new(ListService::new_with_repository(Arc::new(MockListRepository::new())));
        let set_service = Arc::new(SetService::new_with_repository(Arc::new(MockSetRepository::new())));
        let sorted_set_service = Arc::new(SortedSetService::new_with_repository(Arc::new(MockSortedSetRepository::new())));
        let key_service = Arc::new(KeyService::new_with_repository(Arc::new(MockKeyRepository::new())));
        let admin_service = Arc::new(AdminService::new_with_repository(Arc::new(MockAdminRepository::default())));

        let state = AppState::new_with_services(
            pool.clone(),
            config.clone(),
            capabilities.clone(),
            string_service.clone(),
            hash_service.clone(),
            list_service.clone(),
            set_service.clone(),
            sorted_set_service.clone(),
            key_service.clone(),
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
