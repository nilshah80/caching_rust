//! Redis Connection Pool
//!
//! Instrumented connection pool with metrics tracking and TLS support.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use deadpool_redis::{Config, Pool, Runtime, Connection};
use serde::Serialize;
use tracing::{debug, info, warn};
use utoipa::ToSchema;

use crate::domain::errors::CacheError;
use crate::infrastructure::config::{PoolConfig, RedisConfig};
use crate::infrastructure::redis::capabilities::{
    FeatureCapabilities, ModuleCapabilities, RedisCapabilities,
};
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
    pub async fn new(redis_config: &RedisConfig, pool_config: &PoolConfig) -> Result<Self, CacheError> {
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
                warn!("TLS certificate verification is disabled - not recommended for production");
            }

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
        if let Some(at_pos) = url.find('@') {
            if let Some(scheme_end) = url.find("://") {
                let prefix = &url[..scheme_end + 3];
                let suffix = &url[at_pos..];
                return format!("{prefix}***{suffix}");
            }
        }
        url.to_string()
    }

    /// Get a connection from the pool with instrumentation
    pub async fn get(&self) -> Result<Connection, CacheError> {
        self.metrics.current_waiting.fetch_add(1, Ordering::Relaxed);
        self.metrics.total_wait_count.fetch_add(1, Ordering::Relaxed);

        let start = Instant::now();
        let result = self.inner.get().await;
        let wait_ms = start.elapsed().as_millis() as u64;

        self.metrics.total_wait_duration_ms.fetch_add(wait_ms, Ordering::Relaxed);
        self.metrics.current_waiting.fetch_sub(1, Ordering::Relaxed);

        match result {
            Ok(conn) => {
                debug!(wait_ms, "Connection acquired from pool");
                Ok(conn)
            }
            Err(e) => {
                self.metrics.failed_checkouts.fetch_add(1, Ordering::Relaxed);
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
            total_connections_created: self.metrics.total_connections_created.load(Ordering::Relaxed),
            total_wait_count: total_wait,
            avg_wait_ms,
            current_waiting: self.metrics.current_waiting.load(Ordering::Relaxed),
            failed_checkouts: self.metrics.failed_checkouts.load(Ordering::Relaxed),
        }
    }

    /// Detect Redis capabilities
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

        // Get loaded modules
        let modules_result: Result<Vec<Vec<String>>, _> = redis::cmd("MODULE")
            .arg("LIST")
            .query_async(&mut conn)
            .await;

        let modules = modules_result.unwrap_or_default();

        let module_capabilities = ModuleCapabilities {
            json: RedisCapabilities::detect_module(&modules, "rejson")
                || RedisCapabilities::detect_module(&modules, "redisjson"),
            search: RedisCapabilities::detect_module(&modules, "search")
                || RedisCapabilities::detect_module(&modules, "ft"),
            bloom: RedisCapabilities::detect_module(&modules, "bf")
                || RedisCapabilities::detect_module(&modules, "bloom"),
            timeseries: RedisCapabilities::detect_module(&modules, "timeseries"),
            graph: RedisCapabilities::detect_module(&modules, "graph"),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_stats_default() {
        let metrics = PoolMetrics::default();
        assert_eq!(metrics.total_wait_count.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.failed_checkouts.load(Ordering::Relaxed), 0);
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
}
