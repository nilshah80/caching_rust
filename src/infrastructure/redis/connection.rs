//! Redis Connection Pool
//!
//! Instrumented connection pool with metrics tracking and TLS support.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
#[cfg(not(test))]
use std::time::Duration;
use std::time::Instant;

#[cfg(not(test))]
use deadpool_redis::Hook;
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
use crate::infrastructure::redis::pool_connection::PoolConnection;
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
/// Pool + URL bundled together so they can be swapped atomically under one lock.
struct PoolState {
    pool: Pool,
    url: String,
    /// Capabilities detected against the current pool/master.
    /// Refreshed after sentinel failover pool swap.
    capabilities: Option<Arc<RedisCapabilities>>,
}

/// Simple circuit breaker state for fast-failing when Redis is unreachable.
/// After `CIRCUIT_BREAKER_THRESHOLD` consecutive failures, requests are rejected
/// immediately until a cooldown period elapses and a probe succeeds.
struct CircuitBreaker {
    /// Consecutive connection failures
    consecutive_failures: AtomicU64,
    /// Timestamp (epoch secs) when the circuit was opened
    opened_at: AtomicU64,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self {
            consecutive_failures: AtomicU64::new(0),
            opened_at: AtomicU64::new(0),
        }
    }
}

/// Number of consecutive failures before the circuit opens
const CIRCUIT_BREAKER_THRESHOLD: u64 = 5;

/// How long (seconds) the circuit stays open before allowing a probe request
const CIRCUIT_BREAKER_COOLDOWN_SECS: u64 = 5;

pub struct InstrumentedPool {
    /// Pool and resolved URL behind a single RwLock so sentinel failover swaps both atomically.
    /// The read path clones the Pool handle (cheap — internally Arc'd) and drops the lock
    /// before any async work.
    state: std::sync::RwLock<PoolState>,
    metrics: Arc<PoolMetrics>,
    max_size: usize,
    allow_get: bool,
    /// Optional cluster pool. When set, `get()` returns cluster connections
    /// that route commands based on key hash slot (MOVED/ASK handling).
    cluster_pool: Option<crate::infrastructure::redis::cluster_connection::ClusterPool>,
    /// Set to true when sentinel failover detects capability drift.
    /// Readiness probe returns 503 until the process is restarted.
    capability_drift: std::sync::atomic::AtomicBool,
    /// Circuit breaker for fast-failing when Redis is unreachable
    circuit_breaker: CircuitBreaker,
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
        // Resolve connection URL: sentinel mode discovers the master, standalone uses config directly
        let connection_url = if redis_config.sentinel_enabled {
            Self::resolve_sentinel_master(redis_config).await?
        } else {
            Self::build_connection_url(redis_config)?
        };

        let mode = if redis_config.sentinel_enabled {
            "sentinel"
        } else {
            "standalone"
        };

        info!(
            url = %Self::mask_password(&connection_url),
            mode,
            tls_enabled = redis_config.tls_enabled,
            min_size = pool_config.min_size,
            max_size = pool_config.max_size,
            "Creating Redis connection pool"
        );

        if pool_config.idle_timeout_ms > 0 {
            warn!(
                idle_timeout_ms = pool_config.idle_timeout_ms,
                "deadpool-redis does not support idle eviction; idle timeout is documented but not enforced by the pool"
            );
        }

        let metrics = Arc::new(PoolMetrics::default());
        let pool = Self::build_pool(&connection_url, pool_config, metrics.clone())?;

        // Test connection
        Self::verify_connection(&pool).await?;
        Self::prewarm_pool(&pool, pool_config.min_size as usize).await?;

        info!(
            mode,
            tls_enabled = redis_config.tls_enabled,
            min_size = pool_config.min_size,
            max_size = pool_config.max_size,
            "Redis connection pool created successfully"
        );

        Ok(Self {
            state: std::sync::RwLock::new(PoolState {
                pool,
                url: connection_url,
                capabilities: None,
            }),
            metrics,
            max_size: pool_config.max_size as usize,
            allow_get: true,
            cluster_pool: None,
            capability_drift: std::sync::atomic::AtomicBool::new(false),
            circuit_breaker: CircuitBreaker::default(),
        })
    }

    #[cfg(not(test))]
    fn build_pool(
        connection_url: &str,
        pool_config: &PoolConfig,
        metrics: Arc<PoolMetrics>,
    ) -> Result<Pool, CacheError> {
        let mut cfg = Config::from_url(connection_url);
        let mut managed_pool = deadpool_redis::PoolConfig::new(pool_config.max_size as usize);
        managed_pool.timeouts.wait = Some(Duration::from_millis(pool_config.connect_timeout_ms));
        managed_pool.timeouts.create = Some(Duration::from_millis(pool_config.connect_timeout_ms));
        managed_pool.timeouts.recycle = Some(Duration::from_millis(pool_config.command_timeout_ms));
        cfg.pool = Some(managed_pool);

        let command_timeout = Duration::from_millis(pool_config.command_timeout_ms);
        cfg.builder()
            .map_err(|e| CacheError::ConnectionFailed(e.to_string()))?
            .runtime(Runtime::Tokio1)
            .post_create(Hook::sync_fn(move |conn, _metrics| {
                metrics
                    .total_connections_created
                    .fetch_add(1, Ordering::Relaxed);
                conn.set_response_timeout(command_timeout);
                Ok(())
            }))
            .build()
            .map_err(|e| CacheError::ConnectionFailed(e.to_string()))
    }

    #[cfg(not(test))]
    async fn verify_connection(pool: &Pool) -> Result<(), CacheError> {
        let mut conn = pool.get().await.map_err(|e| {
            CacheError::ConnectionFailed(format!("Failed to connect to Redis: {}", e))
        })?;

        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| CacheError::ConnectionFailed(format!("Redis PING failed: {}", e)))?;

        Ok(())
    }

    #[cfg(not(test))]
    async fn prewarm_pool(pool: &Pool, min_size: usize) -> Result<(), CacheError> {
        if min_size == 0 {
            return Ok(());
        }

        let mut connections = Vec::with_capacity(min_size);
        for _ in 0..min_size {
            let conn = pool.get().await.map_err(|e| {
                CacheError::ConnectionFailed(format!("Failed to prewarm Redis pool: {}", e))
            })?;
            connections.push(conn);
        }

        drop(connections);
        info!(prewarmed_connections = min_size, "Redis pool prewarmed");
        Ok(())
    }

    /// Resolve the master URL from Sentinel nodes.
    ///
    /// Queries the sentinel SENTINEL GET-MASTER-ADDR-BY-NAME command
    /// to discover the current master, then builds a redis:// URL for the pool.
    #[cfg(not(test))]
    async fn resolve_sentinel_master(config: &RedisConfig) -> Result<String, CacheError> {
        let sentinel_urls = config.sentinel_node_urls();
        let master_name = &config.sentinel_master_name;

        info!(
            sentinels = ?sentinel_urls,
            master_name,
            "Resolving master from Sentinel"
        );

        // Try each sentinel until one responds.
        // If sentinel_password is set, authenticate to the sentinel itself.
        for url in &sentinel_urls {
            let mut sentinel_info: redis::ConnectionInfo = match url.parse() {
                Ok(info) => info,
                Err(_) => continue,
            };

            // Authenticate to the sentinel node (separate from Redis master password)
            if let Some(ref sentinel_pw) = config.sentinel_password {
                let redis_settings = sentinel_info
                    .redis_settings()
                    .clone()
                    .set_password(sentinel_pw);
                sentinel_info = sentinel_info.set_redis_settings(redis_settings);
            }

            let client = match redis::Client::open(sentinel_info) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut conn = match client.get_multiplexed_async_connection().await {
                Ok(c) => c,
                Err(e) => {
                    warn!(sentinel = %url, error = %e, "Sentinel unreachable, trying next");
                    continue;
                }
            };

            let master_addr: Result<Vec<String>, _> = redis::cmd("SENTINEL")
                .arg("get-master-addr-by-name")
                .arg(master_name)
                .query_async(&mut conn)
                .await;

            match master_addr {
                Ok(addr) if addr.len() >= 2 => {
                    // Build the master URL preserving TLS scheme and database from config
                    let scheme = if config.tls_enabled {
                        "rediss"
                    } else {
                        "redis"
                    };
                    let auth = config
                        .password
                        .as_ref()
                        .map_or(String::new(), |pw| format!(":{pw}@"));
                    let db = if config.database > 0 {
                        format!("/{}", config.database)
                    } else {
                        String::new()
                    };
                    let insecure = if config.tls_enabled && config.tls_skip_verify {
                        "#insecure"
                    } else {
                        ""
                    };
                    let master_url =
                        format!("{scheme}://{auth}{}:{}{db}{insecure}", addr[0], addr[1]);

                    info!(
                        master = %Self::mask_password(&master_url),
                        sentinel = %url,
                        "Sentinel resolved master address"
                    );

                    return Ok(master_url);
                }
                Ok(_) => {
                    warn!(sentinel = %url, "Sentinel returned invalid master address");
                }
                Err(e) => {
                    warn!(sentinel = %url, error = %e, "SENTINEL get-master-addr-by-name failed");
                }
            }
        }

        Err(CacheError::ConnectionFailed(format!(
            "No sentinel could resolve master '{master_name}'"
        )))
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
            state: std::sync::RwLock::new(PoolState {
                pool,
                url: "redis://127.0.0.1:0".to_string(),
                capabilities: None,
            }),
            metrics: Arc::new(PoolMetrics::default()),
            max_size: 1,
            allow_get: false,
            cluster_pool: None,
            capability_drift: std::sync::atomic::AtomicBool::new(false),
            circuit_breaker: CircuitBreaker::default(),
        }
    }

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
            state: std::sync::RwLock::new(PoolState {
                pool,
                url: redis_url.to_string(),
                capabilities: None,
            }),
            metrics: Arc::new(PoolMetrics::default()),
            max_size: 4,
            allow_get: true,
            cluster_pool: None,
            capability_drift: std::sync::atomic::AtomicBool::new(false),
            circuit_breaker: CircuitBreaker::default(),
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

    /// Set the cluster pool for cluster-aware command routing.
    /// When set, `get()` returns cluster connections instead of standalone ones.
    pub fn set_cluster_pool(
        &mut self,
        cluster: crate::infrastructure::redis::cluster_connection::ClusterPool,
    ) {
        self.cluster_pool = Some(cluster);
    }

    /// Check if the circuit breaker is open (Redis is considered down).
    /// Returns Ok(()) if the request should proceed, Err if it should be fast-failed.
    fn check_circuit_breaker(&self) -> Result<(), CacheError> {
        let failures = self
            .circuit_breaker
            .consecutive_failures
            .load(Ordering::Relaxed);
        if failures >= CIRCUIT_BREAKER_THRESHOLD {
            let opened_at = self.circuit_breaker.opened_at.load(Ordering::Relaxed);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            if now.saturating_sub(opened_at) < CIRCUIT_BREAKER_COOLDOWN_SECS {
                return Err(CacheError::ConnectionFailed(
                    "circuit breaker open — Redis unreachable".to_string(),
                ));
            }
            // Cooldown elapsed — allow this request as a probe
        }
        Ok(())
    }

    /// Record a successful connection checkout — resets the circuit breaker.
    fn record_success(&self) {
        self.circuit_breaker
            .consecutive_failures
            .store(0, Ordering::Relaxed);
    }

    /// Record a failed connection checkout — may trip the circuit breaker.
    fn record_failure(&self) {
        let prev = self
            .circuit_breaker
            .consecutive_failures
            .fetch_add(1, Ordering::Relaxed);
        if prev + 1 >= CIRCUIT_BREAKER_THRESHOLD {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            self.circuit_breaker.opened_at.store(now, Ordering::Relaxed);
        }
    }

    /// Get a connection from the pool with instrumentation.
    /// Returns a `PoolConnection` which may be standalone or cluster-routed.
    /// In cluster mode, commands are automatically routed to the correct node.
    /// Includes circuit breaker: fast-fails after consecutive connection failures.
    pub async fn get(&self) -> Result<PoolConnection, CacheError> {
        if !self.allow_get {
            return Err(CacheError::PoolError(
                "pool get disabled in tests".to_string(),
            ));
        }

        self.check_circuit_breaker()?;

        // If we have a cluster pool, prefer it for data command routing
        if let Some(ref cluster) = self.cluster_pool {
            self.metrics.current_waiting.fetch_add(1, Ordering::Relaxed);
            self.metrics
                .total_wait_count
                .fetch_add(1, Ordering::Relaxed);

            let start = Instant::now();
            let result = cluster.get().await;
            let wait_ms = start.elapsed().as_millis() as u64;

            self.metrics
                .total_wait_duration_ms
                .fetch_add(wait_ms, Ordering::Relaxed);
            self.metrics.current_waiting.fetch_sub(1, Ordering::Relaxed);

            return match result {
                Ok(conn) => {
                    self.record_success();
                    debug!(wait_ms, "Cluster connection acquired");
                    Ok(PoolConnection::Cluster(conn))
                }
                Err(e) => {
                    self.record_failure();
                    self.metrics
                        .failed_checkouts
                        .fetch_add(1, Ordering::Relaxed);
                    warn!(error = %e, wait_ms, "Failed to get cluster connection");
                    Err(CacheError::PoolError(e.to_string()))
                }
            };
        }

        self.metrics.current_waiting.fetch_add(1, Ordering::Relaxed);
        self.metrics
            .total_wait_count
            .fetch_add(1, Ordering::Relaxed);

        let start = Instant::now();
        // Clone the pool handle (cheap — Pool is internally Arc'd) to release the read lock
        // before the async .get() call. This avoids holding the lock across an await point.
        let pool = self
            .state
            .read()
            .map_err(|_| CacheError::PoolError("pool lock poisoned".to_string()))?
            .pool
            .clone();
        let result = pool.get().await;
        let wait_ms = start.elapsed().as_millis() as u64;

        self.metrics
            .total_wait_duration_ms
            .fetch_add(wait_ms, Ordering::Relaxed);
        self.metrics.current_waiting.fetch_sub(1, Ordering::Relaxed);

        match result {
            Ok(conn) => {
                self.record_success();
                debug!(wait_ms, "Connection acquired from pool");
                Ok(PoolConnection::Standalone(conn))
            }
            Err(e) => {
                self.record_failure();
                self.metrics
                    .failed_checkouts
                    .fetch_add(1, Ordering::Relaxed);
                warn!(error = %e, wait_ms, "Failed to get connection from pool");
                Err(CacheError::PoolError(e.to_string()))
            }
        }
    }

    /// Get a standalone connection (bypasses cluster pool).
    /// Used by health checks and admin commands that must always hit a known node.
    pub async fn get_standalone(&self) -> Result<Connection, CacheError> {
        if !self.allow_get {
            return Err(CacheError::PoolError(
                "pool get disabled in tests".to_string(),
            ));
        }

        let pool = self
            .state
            .read()
            .map_err(|_| CacheError::PoolError("pool lock poisoned".to_string()))?
            .pool
            .clone();
        pool.get()
            .await
            .map_err(|e| CacheError::PoolError(e.to_string()))
    }

    /// Get the resolved Redis URL used for this pool's connections.
    /// In sentinel mode this is the master address, not the sentinel address.
    pub fn resolved_url(&self) -> String {
        self.state.read().map(|s| s.url.clone()).unwrap_or_default()
    }

    /// Swap the inner pool and resolved URL atomically under a single write lock.
    /// Used by the sentinel watcher to point the pool at a newly promoted master.
    /// Clears cached capabilities so they are re-detected against the new master.
    pub fn swap_pool(&self, new_pool: Pool, new_url: String) {
        if let Ok(mut state) = self.state.write() {
            state.pool = new_pool;
            state.url = new_url;
            state.capabilities = None;
        }
    }

    /// Store detected capabilities in the pool state.
    pub fn store_capabilities(&self, caps: Arc<RedisCapabilities>) {
        if let Ok(mut state) = self.state.write() {
            state.capabilities = Some(caps);
        }
    }

    /// Get the most recently detected capabilities.
    /// Returns None if capabilities have not been detected or were cleared by a pool swap.
    pub fn get_capabilities(&self) -> Option<Arc<RedisCapabilities>> {
        self.state.read().ok().and_then(|s| s.capabilities.clone())
    }

    /// Mark the pool as having drifted capabilities after sentinel failover.
    /// The readiness probe will return 503 until the process is restarted.
    pub fn set_capability_drift(&self) {
        self.capability_drift
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Check if capability drift has been detected after a sentinel failover.
    pub fn has_capability_drift(&self) -> bool {
        self.capability_drift
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Build a fully-configured replacement pool for a new URL.
    ///
    /// Applies the same timeouts, metrics hooks, and prewarming as the initial
    /// pool creation so that sentinel-failover pools behave identically to the
    /// pool created at startup.
    #[cfg(not(test))]
    pub async fn build_replacement_pool(
        &self,
        url: &str,
        pool_config: &PoolConfig,
    ) -> Result<Pool, CacheError> {
        let pool = Self::build_pool(url, pool_config, self.metrics.clone())?;
        Self::verify_connection(&pool).await?;
        Self::prewarm_pool(&pool, pool_config.min_size as usize).await?;
        Ok(pool)
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStats {
        let status = self
            .state
            .read()
            .map(|s| s.pool.status())
            .unwrap_or(deadpool_redis::Status {
                max_size: 0,
                size: 0,
                available: 0,
                waiting: 0,
            });
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

    /// Detect Redis capabilities (always uses standalone connection for node-local INFO)
    #[cfg(not(test))]
    pub async fn detect_capabilities(&self) -> Result<RedisCapabilities, CacheError> {
        let mut conn = self.get_standalone().await?;

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

        // Check cluster mode via CLUSTER INFO (returns cluster_state:ok in cluster mode)
        // or INFO server (contains cluster_enabled:1)
        let cluster_enabled = if let Ok(cluster_info) = redis::cmd("CLUSTER")
            .arg("INFO")
            .query_async::<String>(&mut conn)
            .await
        {
            cluster_info.contains("cluster_state:ok")
        } else {
            // Fallback: check INFO server for cluster_enabled:1
            info.contains("cluster_enabled:1")
        };

        // Probe for vector set commands by checking COMMAND INFO for every
        // exposed command. Version check alone is insufficient — some 8.x builds
        // may not include all vector commands.
        let is_8x = RedisCapabilities::version_gte(&redis_version, "7.9.0");

        // Batch-probe all vector commands we expose in a single COMMAND INFO call.
        // COMMAND INFO returns one entry per command name; Nil means unknown.
        let (vectors_available, vector_range_available) = if is_8x {
            // Core commands: VADD VREM VSIM VCARD VDIM VEMB VISMEMBER VLINKS
            //                VRANDMEMBER VINFO VGETATTR VSETATTR
            // Extended:      VRANGE (may be absent on early 8.x)
            let core_cmds = [
                "VADD",
                "VREM",
                "VSIM",
                "VCARD",
                "VDIM",
                "VEMB",
                "VISMEMBER",
                "VLINKS",
                "VRANDMEMBER",
                "VINFO",
                "VGETATTR",
                "VSETATTR",
            ];
            let mut cmd = redis::cmd("COMMAND");
            cmd.arg("INFO");
            for c in &core_cmds {
                cmd.arg(*c);
            }
            cmd.arg("VRANGE");

            let all_present = |arr: &[redis::Value], count: usize| -> bool {
                arr.len() >= count
                    && arr
                        .iter()
                        .take(count)
                        .all(|v| !matches!(v, redis::Value::Nil))
            };

            match cmd.query_async::<redis::Value>(&mut conn).await {
                Ok(redis::Value::Array(ref arr)) => {
                    let core_ok = all_present(arr, core_cmds.len());
                    let range_ok = core_ok
                        && arr.len() > core_cmds.len()
                        && !matches!(arr.get(core_cmds.len()), Some(redis::Value::Nil));
                    (core_ok, range_ok)
                }
                Err(e) => {
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("noperm") || msg.contains("acl") || msg.contains("denied") {
                        // ACL restricts COMMAND INFO — we cannot verify which
                        // vector commands exist, so fail closed: disable vectors.
                        // Operators must grant COMMAND INFO permission or the
                        // service will not expose vector routes.
                        warn!(
                            "COMMAND INFO blocked by ACL — vector routes disabled. \
                             Grant COMMAND INFO permission to enable vector support."
                        );
                    } else {
                        warn!(?e, "Vector capability probe failed — vectors disabled");
                    }
                    (false, false)
                }
                _ => (false, false),
            }
        } else {
            (false, false)
        };

        let feature_capabilities = FeatureCapabilities {
            streams: RedisCapabilities::version_gte(&redis_version, "5.0.0"),
            acl: RedisCapabilities::version_gte(&redis_version, "6.0.0"),
            functions: RedisCapabilities::version_gte(&redis_version, "7.0.0"),
            lcs: RedisCapabilities::version_gte(&redis_version, "7.0.0"),
            command_docs: RedisCapabilities::version_gte(&redis_version, "7.0.0"),
            hash_field_expiration: RedisCapabilities::version_gte(&redis_version, "7.4.0"),
            // Redis 8.0 pre-releases report as 7.9.x, so gate at 7.9.0
            hash_8_commands: RedisCapabilities::version_gte(&redis_version, "7.9.0"),
            // MSETEX/DELEX/DIGEST landed in Redis 8.4 GA
            string_8_4_commands: RedisCapabilities::version_gte(&redis_version, "8.4.0"),
            // LATENCY HISTOGRAM exists since Redis 7.0
            latency_histogram: RedisCapabilities::version_gte(&redis_version, "7.0.0"),
            // CLUSTER SLOT-STATS landed in Redis 8.2 and only makes sense in cluster mode
            cluster_slot_stats: cluster_enabled
                && RedisCapabilities::version_gte(&redis_version, "8.2.0"),
            // XACKDEL — atomic ack+delete on streams, Redis 8.2+
            xackdel: RedisCapabilities::version_gte(&redis_version, "8.2.0"),
            vectors: vectors_available,
            vector_range: vector_range_available,
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
            ..RedisConfig::default()
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
            ..RedisConfig::default()
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
            ..RedisConfig::default()
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
            ..RedisConfig::default()
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
            ..RedisConfig::default()
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
            ..RedisConfig::default()
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
            ..RedisConfig::default()
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
            ..RedisConfig::default()
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
            ..RedisConfig::default()
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
            ..RedisConfig::default()
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
            ..RedisConfig::default()
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

    #[test]
    fn test_resolved_url() {
        let pool = InstrumentedPool::new_for_tests();
        assert_eq!(pool.resolved_url(), "redis://127.0.0.1:0");
    }

    #[test]
    fn test_swap_pool_updates_url() {
        let pool = InstrumentedPool::new_for_tests();
        assert_eq!(pool.resolved_url(), "redis://127.0.0.1:0");

        // Create a new pool and swap
        let cfg = Config::from_url("redis://127.0.0.1:9999");
        let new_pool = cfg
            .builder()
            .expect("builder")
            .max_size(1)
            .runtime(Runtime::Tokio1)
            .build()
            .expect("build");

        pool.swap_pool(new_pool, "redis://127.0.0.1:9999".to_string());
        assert_eq!(pool.resolved_url(), "redis://127.0.0.1:9999");
    }

    #[tokio::test]
    async fn test_get_standalone_disabled_in_tests() {
        let pool = InstrumentedPool::new_for_tests();
        let err = pool.get_standalone().await.err().expect("should error");
        assert!(matches!(err, CacheError::PoolError(_)));
    }

    #[test]
    fn test_new_for_tests_with_url() {
        let pool = InstrumentedPool::new_for_tests_with_url("redis://127.0.0.1:6379").unwrap();
        assert_eq!(pool.resolved_url(), "redis://127.0.0.1:6379");
        assert_eq!(pool.get_stats().max_size, 4);
    }
}
