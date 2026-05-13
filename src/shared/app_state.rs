//! Application State
//!
//! Shared state passed to all request handlers.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::application::services::{
    AdminService, BitMapService, BloomService, ClusterService, FunctionService, GeoService,
    HashService, JsonService, KeyService, ListService, ProbabilisticService, PubSubService,
    ScriptingService, SearchService, SetService, SortedSetService, StreamService, StringService,
    TimeSeriesService, TransactionService, VectorService,
};
use crate::infrastructure::config::Settings;
use crate::infrastructure::redis::capabilities::RedisCapabilities;
use crate::infrastructure::redis::cluster_connection::ClusterPool;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::pubsub_manager::PubSubManager;
use metrics_exporter_prometheus::PrometheusHandle;

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

    /// Pub/Sub operations service
    pub pubsub_service: Arc<PubSubService>,

    /// Transaction operations service
    pub transaction_service: Arc<TransactionService>,

    /// Lua scripting operations service
    pub scripting_service: Arc<ScriptingService>,

    /// Redis functions operations service
    pub function_service: Arc<FunctionService>,

    /// RedisTimeSeries operations service
    pub timeseries_service: Arc<TimeSeriesService>,

    /// Cluster operations service
    pub cluster_service: Arc<ClusterService>,

    /// Vector Sets operations service
    pub vector_service: Arc<VectorService>,

    /// Cluster connection pool (only set in cluster mode)
    pub cluster_pool: Option<Arc<ClusterPool>>,

    /// Prometheus metrics handle for rendering /metrics endpoint
    pub metrics_handle: Option<Arc<PrometheusHandle>>,
}

impl AppState {
    /// Create new application state
    ///
    /// Note: This creates a PubSubManager using the redis URL from config.
    /// If PubSubManager creation fails, this will panic. For production use,
    /// consider using `try_new` or `new_with_services`.
    pub fn new(
        pool: Arc<InstrumentedPool>,
        config: Arc<Settings>,
        capabilities: Arc<RedisCapabilities>,
    ) -> Self {
        let max_blocking_timeout = Duration::from_secs(config.blocking.max_timeout_seconds as u64);
        let sse_semaphore = Arc::new(Semaphore::new(config.blocking.max_sse_connections));
        let string_service = Arc::new(StringService::new(pool.clone()));
        let hash_service = Arc::new(HashService::new(pool.clone()));
        let list_service = Arc::new(
            ListService::new(pool.clone()).with_max_blocking_timeout(max_blocking_timeout),
        );
        let set_service = Arc::new(SetService::new(pool.clone()));
        let sorted_set_service = Arc::new(
            SortedSetService::new(pool.clone()).with_max_blocking_timeout(max_blocking_timeout),
        );
        let bitmap_service = Arc::new(BitMapService::new(pool.clone()));
        let key_service = Arc::new(KeyService::new(pool.clone()));
        let admin_service = Arc::new(
            AdminService::new(pool.clone()).with_max_blocking_timeout(max_blocking_timeout),
        );
        let stream_service = Arc::new(
            StreamService::new(pool.clone()).with_max_blocking_timeout(max_blocking_timeout),
        );
        let json_service = Arc::new(JsonService::new(pool.clone()));
        let search_service = Arc::new(SearchService::new(pool.clone()));
        let bloom_service = Arc::new(BloomService::new(pool.clone()));
        let probabilistic_service = Arc::new(ProbabilisticService::new(pool.clone()));
        let geo_service = Arc::new(GeoService::new(pool.clone()));

        // Create PubSubManager backed by the pool so it reads resolved_url()
        // on each new subscription — sentinel failover propagates automatically.
        let pubsub_manager = Arc::new(PubSubManager::new_with_pool(
            pool.clone(),
            config.pubsub.clone(),
        ));
        let pubsub_service = Arc::new(PubSubService::new(pool.clone(), pubsub_manager));
        let transaction_service = Arc::new(TransactionService::new(pool.clone()));
        let scripting_service = Arc::new(ScriptingService::new(pool.clone()));
        let function_service = Arc::new(FunctionService::new(pool.clone()));
        let timeseries_service = Arc::new(TimeSeriesService::new(pool.clone()));
        let cluster_repo = Arc::new(
            crate::infrastructure::redis::repositories::RedisClusterRepository::new(pool.clone()),
        );
        let cluster_service = Arc::new(ClusterService::new(cluster_repo));

        let vector_repo = Arc::new(
            crate::infrastructure::redis::repositories::RedisVectorRepository::new(pool.clone()),
        );
        let vector_service = Arc::new(VectorService::new(vector_repo));

        // Install Prometheus recorder (None if already installed, e.g. in tests)
        let metrics_handle = crate::infrastructure::metrics::install_prometheus_recorder()
            .ok()
            .map(Arc::new);

        Self::new_with_services(
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
            pubsub_service,
            transaction_service,
            scripting_service,
            function_service,
            timeseries_service,
            cluster_service,
            vector_service,
            None,
            metrics_handle,
        )
    }

    /// Create new application state with an optional cluster pool.
    /// Used by `main.rs` when booting in cluster mode.
    pub fn new_with_cluster(
        pool: Arc<InstrumentedPool>,
        config: Arc<Settings>,
        capabilities: Arc<RedisCapabilities>,
        cluster_pool: Option<Arc<ClusterPool>>,
    ) -> Self {
        let mut state = Self::new(pool, config, capabilities);
        state.cluster_pool = cluster_pool;
        state
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
        pubsub_service: Arc<PubSubService>,
        transaction_service: Arc<TransactionService>,
        scripting_service: Arc<ScriptingService>,
        function_service: Arc<FunctionService>,
        timeseries_service: Arc<TimeSeriesService>,
        cluster_service: Arc<ClusterService>,
        vector_service: Arc<VectorService>,
        cluster_pool: Option<Arc<ClusterPool>>,
        metrics_handle: Option<Arc<PrometheusHandle>>,
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
            pubsub_service,
            transaction_service,
            scripting_service,
            function_service,
            timeseries_service,
            cluster_service,
            vector_service,
            cluster_pool,
            metrics_handle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        VectorAddResult, VectorInfo, VectorRangeResult, VectorSimResult,
    };
    use crate::domain::errors::CacheError;
    use crate::domain::repositories::VectorRepository;
    use crate::test_support::{
        MockAdminRepository, MockBitMapRepository, MockBloomRepository, MockClusterRepository,
        MockFunctionRepository, MockGeoRepository, MockHashRepository, MockJsonRepository,
        MockKeyRepository, MockListRepository, MockProbabilisticRepository, MockSearchRepository,
        MockSetRepository, MockSortedSetRepository, MockStreamRepository, MockStringRepository,
        MockTimeSeriesRepository,
    };
    use async_trait::async_trait;

    struct MockVectorRepository;

    #[async_trait]
    impl VectorRepository for MockVectorRepository {
        async fn vadd(
            &self,
            key: &str,
            items: Vec<(String, Vec<f32>)>,
        ) -> Result<VectorAddResult, CacheError> {
            Ok(VectorAddResult {
                key: key.to_string(),
                added_count: items.len() as u64,
            })
        }
        async fn vrem(&self, _key: &str, items: Vec<String>) -> Result<u64, CacheError> {
            Ok(items.len() as u64)
        }
        async fn vsim(
            &self,
            _key: &str,
            _vector: Vec<f32>,
            _k: u64,
        ) -> Result<VectorSimResult, CacheError> {
            Ok(VectorSimResult { items: vec![] })
        }
        async fn vcard(&self, _key: &str) -> Result<u64, CacheError> {
            Ok(42)
        }
        async fn vdim(&self, _key: &str) -> Result<u64, CacheError> {
            Ok(128)
        }
        async fn vemb(
            &self,
            _key: &str,
            items: Vec<String>,
        ) -> Result<Vec<Option<Vec<f32>>>, CacheError> {
            Ok(items.into_iter().map(|_| Some(vec![1.0, 2.0])).collect())
        }
        async fn vismember(&self, _key: &str, items: Vec<String>) -> Result<Vec<bool>, CacheError> {
            Ok(items.into_iter().map(|_| true).collect())
        }
        async fn vlinks(&self, _key: &str, _item: &str) -> Result<Vec<Vec<String>>, CacheError> {
            Ok(vec![vec!["neighbor".to_string()]])
        }
        async fn vrandmember(&self, _key: &str, _count: i64) -> Result<Vec<String>, CacheError> {
            Ok(vec!["member1".to_string()])
        }
        async fn vrange(
            &self,
            _key: &str,
            _start: &str,
            _end: &str,
            _count: Option<i64>,
        ) -> Result<VectorRangeResult, CacheError> {
            Ok(VectorRangeResult { items: vec![] })
        }
        async fn vinfo(&self, _key: &str) -> Result<VectorInfo, CacheError> {
            Ok(VectorInfo {
                dimension: 128,
                distance_metric: "L2".to_string(),
                data_type: "FLOAT32".to_string(),
                count: 10,
            })
        }
        async fn vgetattr(&self, _key: &str, _item: &str) -> Result<Option<String>, CacheError> {
            Ok(Some("{}".to_string()))
        }
        async fn vsetattr(
            &self,
            _key: &str,
            _item: &str,
            _attributes: &str,
        ) -> Result<bool, CacheError> {
            Ok(true)
        }
    }

    #[test]
    fn test_new_with_services() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let config = Arc::new(Settings::default());
        let capabilities = Arc::new(RedisCapabilities::default_capabilities());
        let sse_semaphore = Arc::new(Semaphore::new(config.blocking.max_sse_connections));
        let string_service = Arc::new(StringService::new_with_repository(Arc::new(
            MockStringRepository::new(),
        )));
        let hash_service = Arc::new(HashService::new_with_repository(Arc::new(
            MockHashRepository::new(),
        )));
        let list_service = Arc::new(ListService::new_with_repository(Arc::new(
            MockListRepository::new(),
        )));
        let set_service = Arc::new(SetService::new_with_repository(Arc::new(
            MockSetRepository::new(),
        )));
        let sorted_set_service = Arc::new(SortedSetService::new_with_repository(Arc::new(
            MockSortedSetRepository::new(),
        )));
        let bitmap_service = Arc::new(BitMapService::new_with_repository(Arc::new(
            MockBitMapRepository::new(),
        )));
        let key_service = Arc::new(KeyService::new_with_repository(Arc::new(
            MockKeyRepository::new(),
        )));
        let admin_service = Arc::new(AdminService::new_with_repository(Arc::new(
            MockAdminRepository,
        )));
        let stream_service = Arc::new(StreamService::new_with_repository(Arc::new(
            MockStreamRepository::new(),
        )));
        let json_service = Arc::new(JsonService::new_with_repository(Arc::new(
            MockJsonRepository::new(),
        )));
        let search_service = Arc::new(SearchService::new_with_repository(Arc::new(
            MockSearchRepository::new(),
        )));
        let bloom_service = Arc::new(BloomService::new_with_repository(Arc::new(
            MockBloomRepository::new(),
        )));
        let probabilistic_service = Arc::new(ProbabilisticService::new_with_repository(Arc::new(
            MockProbabilisticRepository::new(),
        )));
        let geo_service = Arc::new(GeoService::new_with_repository(Arc::new(
            MockGeoRepository::new(),
        )));
        let pubsub_manager = Arc::new(
            PubSubManager::new(&config.redis.url, config.pubsub.clone())
                .expect("Failed to create PubSubManager for tests"),
        );
        let pubsub_service = Arc::new(PubSubService::new(pool.clone(), pubsub_manager));
        let transaction_service = Arc::new(TransactionService::new(pool.clone()));
        let scripting_service = Arc::new(ScriptingService::new(pool.clone()));
        let function_service = Arc::new(FunctionService::new_with_repository(Arc::new(
            MockFunctionRepository,
        )));
        let timeseries_service = Arc::new(TimeSeriesService::new_with_repository(Arc::new(
            MockTimeSeriesRepository::new(),
        )));
        let cluster_service = Arc::new(ClusterService::new(Arc::new(MockClusterRepository)));
        let vector_service = Arc::new(VectorService::new(Arc::new(MockVectorRepository)));

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
            pubsub_service.clone(),
            transaction_service.clone(),
            scripting_service.clone(),
            function_service,
            timeseries_service,
            cluster_service,
            vector_service,
            None,
            None,
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
        assert!(state.cluster_pool.is_none());
    }

    #[test]
    fn test_new_with_cluster_none() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let config = Arc::new(Settings::default());
        let capabilities = Arc::new(RedisCapabilities::default_capabilities());

        let state = AppState::new_with_cluster(pool, config, capabilities, None);
        assert!(state.cluster_pool.is_none());
    }

    #[test]
    fn test_new_with_cluster_some() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let mut config = Settings::default();
        config.redis.cluster_enabled = true;
        config.redis.cluster_nodes = "redis://127.0.0.1:7001".to_string();
        let config = Arc::new(config);
        let capabilities = Arc::new(RedisCapabilities::default_capabilities());

        let cp = crate::infrastructure::redis::cluster_connection::ClusterPool::new(&config.redis)
            .unwrap();
        let state = AppState::new_with_cluster(pool, config, capabilities, Some(Arc::new(cp)));
        assert!(state.cluster_pool.is_some());
    }
}
