//! Redis Connection Pool
//!
//! Instrumented connection pool with metrics tracking and TLS support.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
#[cfg(not(test))]
use std::time::Instant;
#[cfg(test)]
use std::time::Instant;

use deadpool_redis::{Config, Connection, Pool, Runtime};
use serde::Serialize;
#[cfg(not(test))]
use tracing::{debug, info, warn};
#[cfg(test)]
use tracing::{debug, warn};
use utoipa::ToSchema;

use crate::domain::errors::CacheError;
use crate::infrastructure::config::{PoolConfig, RedisConfig};
use crate::infrastructure::redis::capabilities::RedisCapabilities;
#[cfg(not(test))]
use crate::infrastructure::redis::capabilities::{FeatureCapabilities, ModuleCapabilities};
#[cfg(not(test))]
use chrono::Utc;

/// Pool metrics for monitoring
#[derive(Debug, Default)]
pub struct PoolMetrics {
    /// Total connections created
    pub total_connections_created: AtomicU64,

    /// Total connection checkout requests
    pub total_wait_count: AtomicU64,

    /// Total wait duration in milliseconds
    pub total_wait_duration_ms: AtomicU64,

    /// Currently waiting for connection
    pub current_waiting: AtomicUsize,

    /// Failed connection checkouts
    pub failed_checkouts: AtomicU64,
}

/// Pool statistics response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PoolStats {
    /// Current pool size
    pub size: usize,

    /// Available connections
    pub available: usize,

    /// Maximum pool size
    pub max_size: usize,

    /// Total connections created
    pub total_connections_created: u64,

    /// Total checkout requests
    pub total_wait_count: u64,

    /// Average wait time in milliseconds
    pub avg_wait_ms: f64,

    /// Currently waiting for connection
    pub current_waiting: usize,

    /// Failed checkouts
    pub failed_checkouts: u64,
}

/// Instrumented Redis connection pool with custom metrics
pub struct InstrumentedPool {
    inner: Pool,
    metrics: Arc<PoolMetrics>,
    max_size: usize,
    #[cfg(test)]
    allow_get: bool,
}

impl InstrumentedPool {
    /// Create a new instrumented pool
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to build the connection pool configuration
    /// - Failed to connect to Redis
    /// - TLS configuration is invalid
    #[cfg(not(test))]
    pub async fn new(
        redis_config: &RedisConfig,
        pool_config: &PoolConfig,
    ) -> Result<Self, CacheError> {
        // Build connection URL with TLS if enabled
        let connection_url = Self::build_connection_url(redis_config)?;

        info!(
            url = %Self::mask_password(&connection_url),
            tls_enabled = redis_config.tls_enabled,
            min_size = pool_config.min_size,
            max_size = pool_config.max_size,
            "Creating Redis connection pool"
        );

        let cfg = Config::from_url(&connection_url);
        let pool = cfg
            .builder()
            .map_err(|e| CacheError::ConnectionFailed(e.to_string()))?
            .max_size(pool_config.max_size as usize)
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| CacheError::ConnectionFailed(e.to_string()))?;

        // Test connection
        let mut conn = pool.get().await.map_err(|e| {
            CacheError::ConnectionFailed(format!("Failed to connect to Redis: {}", e))
        })?;

        // Verify connection with PING
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| CacheError::ConnectionFailed(format!("Redis PING failed: {}", e)))?;

        info!(
            tls_enabled = redis_config.tls_enabled,
            "Redis connection pool created successfully"
        );

        Ok(Self {
            inner: pool,
            metrics: Arc::new(PoolMetrics::default()),
            max_size: pool_config.max_size as usize,
            #[cfg(test)]
            allow_get: true,
        })
    }

    #[cfg(test)]
    pub async fn new(
        _redis_config: &RedisConfig,
        _pool_config: &PoolConfig,
    ) -> Result<Self, CacheError> {
        Ok(Self::new_for_tests())
    }

    #[cfg(test)]
    pub fn new_for_tests() -> Self {
        let mut cfg = Config::from_url("redis://127.0.0.1:0");
        let mut pool_cfg = deadpool_redis::PoolConfig::new(1);
        pool_cfg.timeouts.create = Some(std::time::Duration::from_millis(1));
        cfg.pool = Some(pool_cfg);

        let pool = cfg
            .builder()
            .expect("failed to build test pool config")
            .runtime(Runtime::Tokio1)
            .build()
            .expect("failed to build test pool");

        Self {
            inner: pool,
            metrics: Arc::new(PoolMetrics::default()),
            max_size: 1,
            allow_get: false,
        }
    }

    #[cfg(test)]
    pub fn new_for_tests_with_url(redis_url: &str) -> Result<Self, CacheError> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .builder()
            .map_err(|e| CacheError::ConnectionFailed(e.to_string()))?
            .max_size(4)
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| CacheError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            inner: pool,
            metrics: Arc::new(PoolMetrics::default()),
            max_size: 4,
            allow_get: true,
        })
    }

    /// Build the Redis connection URL with TLS support
    fn build_connection_url(config: &RedisConfig) -> Result<String, CacheError> {
        let mut url = config.url.clone();

        // If TLS is enabled, ensure we use rediss:// scheme
        if config.tls_enabled {
            if url.starts_with("redis://") {
                url = url.replacen("redis://", "rediss://", 1);
            } else if !url.starts_with("rediss://") {
                url = format!("rediss://{url}");
            }

            // Add TLS parameters to the URL if skip_verify is enabled
            if config.tls_skip_verify {
                // The redis crate with tls-rustls-insecure feature allows insecure connections
                // when using rediss:// scheme with #insecure flag
                if !url.contains('#') {
                    url.push_str("#insecure");
                }
                #[cfg(not(test))]
                warn!("TLS certificate verification is disabled - not recommended for production");
            }

            #[cfg(not(test))]
            info!(
                tls_skip_verify = config.tls_skip_verify,
                tls_cert_path = ?config.tls_cert_path,
                tls_ca_path = ?config.tls_ca_path,
                "TLS configuration applied"
            );
        }

        // Handle password if provided separately
        if let Some(ref password) = config.password {
            // Insert password into URL if not already present
            if !url.contains('@') {
                url = Self::insert_password_into_url(&url, password)?;
            }
        }

        // Handle database selection
        if config.database > 0 {
            // Check if URL already has a database path (after host:port)
            // URL format: redis://host:port or redis://host:port/db
            let has_db_path = url
                .strip_prefix("redis://")
                .or_else(|| url.strip_prefix("rediss://"))
                .map(|rest| {
                    // Remove any fragment (#insecure)
                    let rest = rest.split('#').next().unwrap_or(rest);
                    // Check if there's a path after host:port
                    rest.contains('/')
                })
                .unwrap_or(false);

            if !has_db_path {
                // Remove any fragment, add database, then re-add fragment
                if let Some(hash_pos) = url.find('#') {
                    let (base, fragment) = url.split_at(hash_pos);
                    url = format!("{}/{}{}", base, config.database, fragment);
                } else {
                    url = format!("{}/{}", url, config.database);
                }
            }
        }

        Ok(url)
    }

    /// Insert password into Redis URL
    fn insert_password_into_url(url: &str, password: &str) -> Result<String, CacheError> {
        // Parse URL scheme (redis:// or rediss://)
        let (scheme, rest) = if let Some(rest) = url.strip_prefix("rediss://") {
            ("rediss://", rest)
        } else if let Some(rest) = url.strip_prefix("redis://") {
            ("redis://", rest)
        } else {
            return Err(CacheError::InvalidInput(format!(
                "Invalid Redis URL scheme: {url}"
            )));
        };

        // Insert password: redis://password@host:port
        Ok(format!("{scheme}:{password}@{rest}"))
    }

    /// Mask password in URL for logging
    fn mask_password(url: &str) -> String {
        // Simple regex-free password masking
        if let Some(at_pos) = url.find('@')
            && let Some(scheme_end) = url.find("://")
        {
            let prefix = &url[..scheme_end + 3];
            let suffix = &url[at_pos..];
            return format!("{prefix}***{suffix}");
        }
        url.to_string()
    }

    /// Get a connection from the pool with instrumentation
    #[cfg(not(test))]
    pub async fn get(&self) -> Result<Connection, CacheError> {
        self.metrics.current_waiting.fetch_add(1, Ordering::Relaxed);
        self.metrics
            .total_wait_count
            .fetch_add(1, Ordering::Relaxed);

        let start = Instant::now();
        let result = self.inner.get().await;
        let wait_ms = start.elapsed().as_millis() as u64;

        self.metrics
            .total_wait_duration_ms
            .fetch_add(wait_ms, Ordering::Relaxed);
        self.metrics.current_waiting.fetch_sub(1, Ordering::Relaxed);

        match result {
            Ok(conn) => {
                debug!(wait_ms, "Connection acquired from pool");
                Ok(conn)
            }
            Err(e) => {
                self.metrics
                    .failed_checkouts
                    .fetch_add(1, Ordering::Relaxed);
                warn!(error = %e, wait_ms, "Failed to get connection from pool");
                Err(CacheError::PoolError(e.to_string()))
            }
        }
    }

    #[cfg(test)]
    pub async fn get(&self) -> Result<Connection, CacheError> {
        if !self.allow_get {
            return Err(CacheError::PoolError(
                "pool get disabled in tests".to_string(),
            ));
        }

        self.metrics.current_waiting.fetch_add(1, Ordering::Relaxed);
        self.metrics
            .total_wait_count
            .fetch_add(1, Ordering::Relaxed);

        let start = Instant::now();
        let result = self.inner.get().await;
        let wait_ms = start.elapsed().as_millis() as u64;

        self.metrics
            .total_wait_duration_ms
            .fetch_add(wait_ms, Ordering::Relaxed);
        self.metrics.current_waiting.fetch_sub(1, Ordering::Relaxed);

        match result {
            Ok(conn) => {
                debug!(wait_ms, "Connection acquired from pool");
                Ok(conn)
            }
            Err(e) => {
                self.metrics
                    .failed_checkouts
                    .fetch_add(1, Ordering::Relaxed);
                warn!(error = %e, wait_ms, "Failed to get connection from pool");
                Err(CacheError::PoolError(e.to_string()))
            }
        }
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStats {
        let status = self.inner.status();
        let total_wait = self.metrics.total_wait_count.load(Ordering::Relaxed);
        let total_duration = self.metrics.total_wait_duration_ms.load(Ordering::Relaxed);

        let avg_wait_ms = if total_wait > 0 {
            total_duration as f64 / total_wait as f64
        } else {
            0.0
        };

        PoolStats {
            size: status.size,
            available: status.available,
            max_size: self.max_size,
            total_connections_created: self
                .metrics
                .total_connections_created
                .load(Ordering::Relaxed),
            total_wait_count: total_wait,
            avg_wait_ms,
            current_waiting: self.metrics.current_waiting.load(Ordering::Relaxed),
            failed_checkouts: self.metrics.failed_checkouts.load(Ordering::Relaxed),
        }
    }

    /// Detect Redis capabilities
    #[cfg(not(test))]
    pub async fn detect_capabilities(&self) -> Result<RedisCapabilities, CacheError> {
        let mut conn = self.get().await?;

        // Get Redis version from INFO
        let info: String = redis::cmd("INFO")
            .arg("server")
            .query_async(&mut conn)
            .await
            .map_err(CacheError::RedisError)?;

        let redis_version = RedisCapabilities::parse_version(&info);
        info!(version = %redis_version, "Detected Redis version");

        // Get loaded modules using redis::Value for flexible parsing
        // Redis 8 returns: [["name", "module1", "ver", "123", ...], ["name", "module2", ...]]
        let modules_result: Result<redis::Value, _> = redis::cmd("MODULE")
            .arg("LIST")
            .query_async(&mut conn)
            .await;

        let module_names = extract_module_names(modules_result.ok());
        debug!(?module_names, "Detected Redis modules");

        let module_capabilities = ModuleCapabilities {
            json: module_names
                .iter()
                .any(|n| n.contains("rejson") || n.contains("redisjson")),
            search: module_names
                .iter()
                .any(|n| n.contains("search") || n == "ft"),
            bloom: module_names
                .iter()
                .any(|n| n == "bf" || n.contains("bloom")),
            timeseries: module_names.iter().any(|n| n.contains("timeseries")),
            graph: module_names.iter().any(|n| n.contains("graph")),
        };

        // Check cluster mode
        let cluster_result: Result<String, _> = redis::cmd("CLUSTER")
            .arg("INFO")
            .query_async(&mut conn)
            .await;

        let cluster_enabled = cluster_result
            .map(|info| info.contains("cluster_enabled:1"))
            .unwrap_or(false);

        let feature_capabilities = FeatureCapabilities {
            streams: RedisCapabilities::version_gte(&redis_version, "5.0.0"),
            acl: RedisCapabilities::version_gte(&redis_version, "6.0.0"),
            functions: RedisCapabilities::version_gte(&redis_version, "7.0.0"),
            lcs: RedisCapabilities::version_gte(&redis_version, "7.0.0"),
            command_docs: RedisCapabilities::version_gte(&redis_version, "7.0.0"),
            hash_field_expiration: RedisCapabilities::version_gte(&redis_version, "7.4.0"),
            // Redis 8.0 pre-releases report as 7.9.x, so gate at 7.9.0
            hash_8_commands: RedisCapabilities::version_gte(&redis_version, "7.9.0"),
            cluster: cluster_enabled,
        };

        info!(
            ?module_capabilities,
            ?feature_capabilities,
            "Redis capabilities detected"
        );

        Ok(RedisCapabilities {
            redis_version,
            modules: module_capabilities,
            features: feature_capabilities,
            detected_at: Utc::now(),
        })
    }

    #[cfg(test)]
    pub async fn detect_capabilities(&self) -> Result<RedisCapabilities, CacheError> {
        let _ = self;
        Err(CacheError::ConnectionFailed(
            "capability detection disabled in tests".to_string(),
        ))
    }
}

/// Extract module names from Redis MODULE LIST response
/// Redis 8 returns: Array([Array([BulkString("name"), BulkString("timeseries"), ...])])
/// We look for "name" keys and extract the following value as the module name
#[cfg(not(test))]
fn extract_module_names(value: Option<redis::Value>) -> Vec<String> {
    let mut names = Vec::new();

    if let Some(redis::Value::Array(modules)) = value {
        for module in modules {
            if let redis::Value::Array(fields) = module {
                // Look for "name" key and get the next value
                let mut iter = fields.iter();
                while let Some(field) = iter.next() {
                    if let redis::Value::BulkString(key) = field
                        && key == b"name"
                    {
                        if let Some(redis::Value::BulkString(name_bytes)) = iter.next()
                            && let Ok(name) = String::from_utf8(name_bytes.clone())
                        {
                            names.push(name.to_lowercase());
                        }
                        break;
                    }
                }
            }
        }
    }

    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::config::PoolConfig;
    use crate::infrastructure::config::RedisConfig;

    #[test]
    fn test_pool_stats_default() {
        let metrics = PoolMetrics::default();
        assert_eq!(metrics.total_wait_count.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.failed_checkouts.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_new_stub() {
        let pool = InstrumentedPool::new(&RedisConfig::default(), &PoolConfig::default())
            .await
            .unwrap();
        let stats = pool.get_stats();
        assert_eq!(stats.max_size, 1);
    }

    #[test]
    fn test_build_connection_url_no_tls() {
        let config = RedisConfig {
            url: "redis://localhost:6379".to_string(),
            password: None,
            database: 0,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: false,
        };
        let url = InstrumentedPool::build_connection_url(&config).unwrap();
        assert_eq!(url, "redis://localhost:6379");
    }

    #[test]
    fn test_build_connection_url_with_tls() {
        let config = RedisConfig {
            url: "redis://localhost:6379".to_string(),
            password: None,
            database: 0,
            tls_enabled: true,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: false,
        };
        let url = InstrumentedPool::build_connection_url(&config).unwrap();
        assert_eq!(url, "rediss://localhost:6379");
    }

    #[test]
    fn test_build_connection_url_with_tls_insecure() {
        let config = RedisConfig {
            url: "redis://localhost:6379".to_string(),
            password: None,
            database: 0,
            tls_enabled: true,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: true,
        };
        let url = InstrumentedPool::build_connection_url(&config).unwrap();
        assert_eq!(url, "rediss://localhost:6379#insecure");
    }

    #[test]
    fn test_build_connection_url_with_password() {
        let config = RedisConfig {
            url: "redis://localhost:6379".to_string(),
            password: Some("secret123".to_string()),
            database: 0,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: false,
        };
        let url = InstrumentedPool::build_connection_url(&config).unwrap();
        assert_eq!(url, "redis://:secret123@localhost:6379");
    }

    #[test]
    fn test_build_connection_url_with_database() {
        let config = RedisConfig {
            url: "redis://localhost:6379".to_string(),
            password: None,
            database: 5,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: false,
        };
        let url = InstrumentedPool::build_connection_url(&config).unwrap();
        assert_eq!(url, "redis://localhost:6379/5");
    }

    #[test]
    fn test_build_connection_url_tls_prefix_for_plain_host() {
        let config = RedisConfig {
            url: "localhost:6379".to_string(),
            password: None,
            database: 0,
            tls_enabled: true,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: false,
        };
        let url = InstrumentedPool::build_connection_url(&config).unwrap();
        assert_eq!(url, "rediss://localhost:6379");
    }

    #[test]
    fn test_build_connection_url_with_existing_password() {
        let config = RedisConfig {
            url: "redis://:old@localhost:6379".to_string(),
            password: Some("new".to_string()),
            database: 0,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: false,
        };
        let url = InstrumentedPool::build_connection_url(&config).unwrap();
        assert_eq!(url, "redis://:old@localhost:6379");
    }

    #[test]
    fn test_build_connection_url_tls_with_fragment_and_db() {
        let config = RedisConfig {
            url: "rediss://localhost:6379#insecure".to_string(),
            password: None,
            database: 2,
            tls_enabled: true,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: true,
        };
        let url = InstrumentedPool::build_connection_url(&config).unwrap();
        assert_eq!(url, "rediss://localhost:6379/2#insecure");
    }

    #[test]
    fn test_build_connection_url_does_not_override_db_path() {
        let config = RedisConfig {
            url: "redis://localhost:6379/1".to_string(),
            password: None,
            database: 5,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: false,
        };
        let url = InstrumentedPool::build_connection_url(&config).unwrap();
        assert_eq!(url, "redis://localhost:6379/1");
    }

    #[test]
    fn test_build_connection_url_tls_already_rediss() {
        let config = RedisConfig {
            url: "rediss://localhost:6379".to_string(),
            password: None,
            database: 0,
            tls_enabled: true,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: false,
        };
        let url = InstrumentedPool::build_connection_url(&config).unwrap();
        assert_eq!(url, "rediss://localhost:6379");
    }

    #[test]
    fn test_mask_password() {
        assert_eq!(
            InstrumentedPool::mask_password("redis://:secret@localhost:6379"),
            "redis://***@localhost:6379"
        );
        assert_eq!(
            InstrumentedPool::mask_password("rediss://:password123@redis.example.com:6380"),
            "rediss://***@redis.example.com:6380"
        );
        assert_eq!(
            InstrumentedPool::mask_password("redis://localhost:6379"),
            "redis://localhost:6379"
        );
    }

    #[test]
    fn test_build_connection_url_invalid_scheme() {
        let config = RedisConfig {
            url: "http://localhost:6379".to_string(),
            password: Some("secret".to_string()),
            database: 0,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: false,
        };
        let err = InstrumentedPool::build_connection_url(&config).unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[test]
    fn test_get_stats_defaults() {
        let pool = InstrumentedPool::new_for_tests();
        let stats = pool.get_stats();
        assert_eq!(stats.max_size, 1);
        assert_eq!(stats.total_wait_count, 0);
    }

    #[test]
    fn test_get_stats_with_waits() {
        let pool = InstrumentedPool::new_for_tests();
        pool.metrics.total_wait_count.store(2, Ordering::Relaxed);
        pool.metrics
            .total_wait_duration_ms
            .store(10, Ordering::Relaxed);
        let stats = pool.get_stats();
        assert_eq!(stats.avg_wait_ms, 5.0);
    }

    #[tokio::test]
    async fn test_get_stub() {
        let pool = InstrumentedPool::new_for_tests();
        let err = pool.get().await.err().expect("pool error");
        assert!(matches!(err, CacheError::PoolError(_)));
    }

    #[tokio::test]
    async fn test_detect_capabilities_stub() {
        let pool = InstrumentedPool::new_for_tests();
        let err = pool.detect_capabilities().await.unwrap_err();
        assert!(matches!(err, CacheError::ConnectionFailed(_)));
    }
}
