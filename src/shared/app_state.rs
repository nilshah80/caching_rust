//! Application State
//!
//! Shared state passed to all request handlers.

use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::application::services::{AdminService, BitMapService, BloomService, GeoService, HashService, JsonService, KeyService, ListService, ProbabilisticService, SearchService, SetService, SortedSetService, StreamService, StringService};
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

    /// Semaphore to limit concurrent SSE/streaming connections
    /// Prevents pool exhaustion from long-lived blocking connections
    pub sse_semaphore: Arc<Semaphore>,

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

    /// Bitmap operations service
    pub bitmap_service: Arc<BitMapService>,

    /// Key management service
    pub key_service: Arc<KeyService>,

    /// Admin operations service
    pub admin_service: Arc<AdminService>,

    /// Stream operations service
    pub stream_service: Arc<StreamService>,

    /// JSON operations service (RedisJSON module)
    pub json_service: Arc<JsonService>,

    /// Search operations service (RediSearch module)
    pub search_service: Arc<SearchService>,

    /// Bloom filter operations service (RedisBloom module)
    pub bloom_service: Arc<BloomService>,

    /// Probabilistic data structures service (CMS, Top-K, HyperLogLog)
    pub probabilistic_service: Arc<ProbabilisticService>,

    /// Geospatial operations service
    pub geo_service: Arc<GeoService>,
}

impl AppState {
    /// Create new application state
    pub fn new(
        pool: Arc<InstrumentedPool>,
        config: Arc<Settings>,
        capabilities: Arc<RedisCapabilities>,
    ) -> Self {
        let sse_semaphore = Arc::new(Semaphore::new(config.blocking.max_sse_connections));
        let string_service = Arc::new(StringService::new(pool.clone()));
        let hash_service = Arc::new(HashService::new(pool.clone()));
        let list_service = Arc::new(ListService::new(pool.clone()));
        let set_service = Arc::new(SetService::new(pool.clone()));
        let sorted_set_service = Arc::new(SortedSetService::new(pool.clone()));
        let bitmap_service = Arc::new(BitMapService::new(pool.clone()));
        let key_service = Arc::new(KeyService::new(pool.clone()));
        let admin_service = Arc::new(AdminService::new(pool.clone()));
        let stream_service = Arc::new(StreamService::new(pool.clone()));
        let json_service = Arc::new(JsonService::new(pool.clone()));
        let search_service = Arc::new(SearchService::new(pool.clone()));
        let bloom_service = Arc::new(BloomService::new(pool.clone()));
        let probabilistic_service = Arc::new(ProbabilisticService::new(pool.clone()));
        let geo_service = Arc::new(GeoService::new(pool.clone()));

        Self::new_with_services(pool, config, capabilities, sse_semaphore, string_service, hash_service, list_service, set_service, sorted_set_service, bitmap_service, key_service, admin_service, stream_service, json_service, search_service, bloom_service, probabilistic_service, geo_service)
    }

    /// Create new application state with custom services (useful for testing)
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_services(
        pool: Arc<InstrumentedPool>,
        config: Arc<Settings>,
        capabilities: Arc<RedisCapabilities>,
        sse_semaphore: Arc<Semaphore>,
        string_service: Arc<StringService>,
        hash_service: Arc<HashService>,
        list_service: Arc<ListService>,
        set_service: Arc<SetService>,
        sorted_set_service: Arc<SortedSetService>,
        bitmap_service: Arc<BitMapService>,
        key_service: Arc<KeyService>,
        admin_service: Arc<AdminService>,
        stream_service: Arc<StreamService>,
        json_service: Arc<JsonService>,
        search_service: Arc<SearchService>,
        bloom_service: Arc<BloomService>,
        probabilistic_service: Arc<ProbabilisticService>,
        geo_service: Arc<GeoService>,
    ) -> Self {
        Self {
            pool,
            config,
            capabilities,
            sse_semaphore,
            string_service,
            hash_service,
            list_service,
            set_service,
            sorted_set_service,
            bitmap_service,
            key_service,
            admin_service,
            stream_service,
            json_service,
            search_service,
            bloom_service,
            probabilistic_service,
            geo_service,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockAdminRepository, MockBitMapRepository, MockBloomRepository, MockGeoRepository, MockHashRepository, MockJsonRepository, MockKeyRepository, MockListRepository, MockProbabilisticRepository, MockSearchRepository, MockSetRepository, MockSortedSetRepository, MockStreamRepository, MockStringRepository};

    #[test]
    fn test_new_with_services() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let config = Arc::new(Settings::default());
        let capabilities = Arc::new(RedisCapabilities::default_capabilities());
        let sse_semaphore = Arc::new(Semaphore::new(config.blocking.max_sse_connections));
        let string_service = Arc::new(StringService::new_with_repository(Arc::new(MockStringRepository::new())));
        let hash_service = Arc::new(HashService::new_with_repository(Arc::new(MockHashRepository::new())));
        let list_service = Arc::new(ListService::new_with_repository(Arc::new(MockListRepository::new())));
        let set_service = Arc::new(SetService::new_with_repository(Arc::new(MockSetRepository::new())));
        let sorted_set_service = Arc::new(SortedSetService::new_with_repository(Arc::new(MockSortedSetRepository::new())));
        let bitmap_service = Arc::new(BitMapService::new_with_repository(Arc::new(MockBitMapRepository::new())));
        let key_service = Arc::new(KeyService::new_with_repository(Arc::new(MockKeyRepository::new())));
        let admin_service = Arc::new(AdminService::new_with_repository(Arc::new(MockAdminRepository::default())));
        let stream_service = Arc::new(StreamService::new_with_repository(Arc::new(MockStreamRepository::new())));
        let json_service = Arc::new(JsonService::new_with_repository(Arc::new(MockJsonRepository::new())));
        let search_service = Arc::new(SearchService::new_with_repository(Arc::new(MockSearchRepository::new())));
        let bloom_service = Arc::new(BloomService::new_with_repository(Arc::new(MockBloomRepository::new())));
        let probabilistic_service = Arc::new(ProbabilisticService::new_with_repository(Arc::new(MockProbabilisticRepository::new())));
        let geo_service = Arc::new(GeoService::new_with_repository(Arc::new(MockGeoRepository::new())));

        let state = AppState::new_with_services(
            pool.clone(),
            config.clone(),
            capabilities.clone(),
            sse_semaphore,
            string_service.clone(),
            hash_service.clone(),
            list_service.clone(),
            set_service.clone(),
            sorted_set_service.clone(),
            bitmap_service.clone(),
            key_service.clone(),
            admin_service.clone(),
            stream_service.clone(),
            json_service.clone(),
            search_service.clone(),
            bloom_service.clone(),
            probabilistic_service.clone(),
            geo_service.clone(),
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
