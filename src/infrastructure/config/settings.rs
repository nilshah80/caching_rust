//! Application Settings
//!
//! Configuration loaded from environment variables.

use config::{Config, Environment};
use serde::Deserialize;

/// Root settings structure
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Settings {
    pub server: ServerConfig,
    pub redis: RedisConfig,
    pub pool: PoolConfig,
    pub pubsub: PubSubConfig,
    pub blocking: BlockingConfig,
    pub admin: AdminConfig,
    pub log: LogConfig,
    pub rate_limit: RateLimitConfig,
}

/// HTTP Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to (default: "0.0.0.0")
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to listen on (default: 8080)
    #[serde(default = "default_port")]
    pub port: u16,

    /// Request timeout in milliseconds (default: 30000)
    #[serde(default = "default_request_timeout")]
    pub request_timeout_ms: u64,

    /// Maximum request body size in bytes (default: 10MB)
    #[serde(default = "default_max_body_size")]
    pub max_body_size_bytes: usize,

    /// Maximum batch size for bulk operations like MSET, MGET (default: 1000)
    #[serde(default = "default_max_batch_size")]
    pub max_batch_size: usize,

    /// Maximum string value size in bytes (default: 512KB)
    #[serde(default = "default_max_value_size")]
    pub max_value_size_bytes: usize,

    /// Allowed CORS origins (comma-separated). Use "*" to allow any origin.
    /// Default: "*" (permissive, suitable for development only).
    /// Example: "https://app.example.com,https://admin.example.com"
    #[serde(default = "default_cors_origins")]
    pub cors_origins: String,
}

/// Redis connection configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    /// Redis URL (default: "redis://localhost:6379")
    #[serde(default = "default_redis_url")]
    pub url: String,

    /// Redis password (optional)
    pub password: Option<String>,

    /// Redis database number (default: 0)
    #[serde(default)]
    pub database: u8,

    /// Enable TLS for Redis connection
    #[serde(default)]
    pub tls_enabled: bool,

    /// Path to TLS certificate file
    pub tls_cert_path: Option<String>,

    /// Path to TLS key file
    pub tls_key_path: Option<String>,

    /// Path to TLS CA certificate file
    pub tls_ca_path: Option<String>,

    /// Skip TLS certificate verification (not recommended for production)
    #[serde(default)]
    pub tls_skip_verify: bool,

    /// Enable Redis Cluster mode (mutually exclusive with sentinel)
    #[serde(default)]
    pub cluster_enabled: bool,

    /// Comma-separated cluster seed node URLs
    /// Example: "redis://node1:7001,redis://node2:7002,redis://node3:7003"
    #[serde(default)]
    pub cluster_nodes: String,

    /// Read from cluster replicas (default: false)
    #[serde(default)]
    pub cluster_read_from_replicas: bool,

    /// Enable Redis Sentinel mode (mutually exclusive with cluster)
    #[serde(default)]
    pub sentinel_enabled: bool,

    /// Comma-separated sentinel node URLs
    /// Example: "redis://sentinel1:26379,redis://sentinel2:26379"
    #[serde(default)]
    pub sentinel_nodes: String,

    /// Sentinel master group name (default: "mymaster")
    #[serde(default = "default_sentinel_master_name")]
    pub sentinel_master_name: String,

    /// Sentinel password (separate from Redis password)
    pub sentinel_password: Option<String>,
}

/// Connection pool configuration
#[derive(Debug, Clone, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of connections (default: 2)
    #[serde(default = "default_pool_min")]
    pub min_size: u32,

    /// Maximum number of connections (default: 10)
    #[serde(default = "default_pool_max")]
    pub max_size: u32,

    /// Connection timeout in milliseconds (default: 5000)
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,

    /// Command timeout in milliseconds (default: 5000)
    #[serde(default = "default_command_timeout")]
    pub command_timeout_ms: u64,

    /// Idle connection timeout in milliseconds (default: 600000 = 10 minutes)
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_ms: u64,
}

/// Pub/Sub configuration (separate from command pool)
#[derive(Debug, Clone, Deserialize)]
pub struct PubSubConfig {
    /// Maximum concurrent subscriptions (default: 100)
    #[serde(default = "default_max_subscriptions")]
    pub max_subscriptions: usize,

    /// Connection timeout for subscriptions in milliseconds (default: 30000)
    #[serde(default = "default_pubsub_connect_timeout")]
    pub connection_timeout_ms: u64,
}

/// Blocking commands configuration
#[derive(Debug, Clone, Deserialize)]
pub struct BlockingConfig {
    /// Maximum blocking timeout in seconds (default: 30, hard limit)
    #[serde(default = "default_max_blocking_timeout")]
    pub max_timeout_seconds: u32,

    /// Default blocking timeout in seconds (default: 5)
    #[serde(default = "default_blocking_timeout")]
    pub default_timeout_seconds: u32,

    /// Maximum concurrent SSE/streaming connections (default: 5)
    /// Limits how many SSE connections can hold pool connections simultaneously
    /// to prevent pool exhaustion from long-lived streaming requests.
    /// Default is half of pool.max_size (10) to leave room for regular requests.
    #[serde(default = "default_max_sse_connections")]
    pub max_sse_connections: usize,

    /// Default count for unbounded XREAD operations (default: 100)
    /// Prevents OOM when reading large streams without explicit count
    #[serde(default = "default_stream_read_count")]
    pub default_stream_read_count: usize,
}

/// Admin API configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AdminConfig {
    /// API key required for admin endpoints
    pub api_key: String,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting (default: true)
    #[serde(default = "default_rate_limit_enabled")]
    pub enabled: bool,

    /// Maximum requests per second (global, shared across all clients) (default: 100)
    #[serde(default = "default_rate_limit_rps")]
    pub requests_per_second: u64,

    /// Burst size (default: 50)
    #[serde(default = "default_rate_limit_burst")]
    pub burst_size: u32,
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    /// Log level (default: "info")
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log format: "json" or "pretty" (default: "json")
    #[serde(default = "default_log_format")]
    pub format: String,
}

// Default value functions
fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_request_timeout() -> u64 {
    30000
}

fn default_max_body_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

fn default_max_batch_size() -> usize {
    1000
}

fn default_max_value_size() -> usize {
    512 * 1024 // 512KB
}

fn default_sentinel_master_name() -> String {
    "mymaster".to_string()
}

fn default_cors_origins() -> String {
    "*".to_string()
}

fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}

fn default_pool_min() -> u32 {
    2
}

fn default_pool_max() -> u32 {
    10
}

fn default_connect_timeout() -> u64 {
    5000
}

fn default_command_timeout() -> u64 {
    5000
}

fn default_idle_timeout() -> u64 {
    600_000
}

fn default_max_subscriptions() -> usize {
    100
}

fn default_pubsub_connect_timeout() -> u64 {
    30000
}

fn default_max_blocking_timeout() -> u32 {
    30
}

fn default_blocking_timeout() -> u32 {
    5
}

fn default_max_sse_connections() -> usize {
    // Default to half of pool max (5) to leave room for regular requests
    // Users should configure this based on their pool.max_size
    5
}

fn default_stream_read_count() -> usize {
    100
}

fn default_rate_limit_enabled() -> bool {
    false
}

fn default_rate_limit_rps() -> u64 {
    100
}

fn default_rate_limit_burst() -> u32 {
    50
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            request_timeout_ms: default_request_timeout(),
            max_body_size_bytes: default_max_body_size(),
            max_batch_size: default_max_batch_size(),
            max_value_size_bytes: default_max_value_size(),
            cors_origins: default_cors_origins(),
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: default_redis_url(),
            password: None,
            database: 0,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_skip_verify: false,
            cluster_enabled: false,
            cluster_nodes: String::new(),
            cluster_read_from_replicas: false,
            sentinel_enabled: false,
            sentinel_nodes: String::new(),
            sentinel_master_name: default_sentinel_master_name(),
            sentinel_password: None,
        }
    }
}

impl RedisConfig {
    /// Parse cluster_nodes string into a Vec of URLs.
    pub fn cluster_node_urls(&self) -> Vec<String> {
        self.cluster_nodes
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Parse sentinel_nodes string into a Vec of URLs.
    pub fn sentinel_node_urls(&self) -> Vec<String> {
        self.sentinel_nodes
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_size: default_pool_min(),
            max_size: default_pool_max(),
            connect_timeout_ms: default_connect_timeout(),
            command_timeout_ms: default_command_timeout(),
            idle_timeout_ms: default_idle_timeout(),
        }
    }
}

impl Default for PubSubConfig {
    fn default() -> Self {
        Self {
            max_subscriptions: default_max_subscriptions(),
            connection_timeout_ms: default_pubsub_connect_timeout(),
        }
    }
}

impl Default for BlockingConfig {
    fn default() -> Self {
        Self {
            max_timeout_seconds: default_max_blocking_timeout(),
            default_timeout_seconds: default_blocking_timeout(),
            max_sse_connections: default_max_sse_connections(),
            default_stream_read_count: default_stream_read_count(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: default_rate_limit_enabled(),
            requests_per_second: default_rate_limit_rps(),
            burst_size: default_rate_limit_burst(),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            api_key: "changeme-admin-key".to_string(),
        }
    }
}

impl Settings {
    /// Load settings from environment variables
    pub fn load() -> anyhow::Result<Self> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        let config = Config::builder()
            // Set defaults
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("server.request_timeout_ms", 30000)?
            .set_default("server.max_body_size_bytes", 10 * 1024 * 1024)?
            .set_default("server.max_batch_size", 1000)?
            .set_default("server.max_value_size_bytes", 512 * 1024)?
            .set_default("server.cors_origins", "*")?
            .set_default("redis.url", "redis://localhost:6379")?
            .set_default("redis.database", 0)?
            .set_default("redis.tls_enabled", false)?
            .set_default("redis.tls_skip_verify", false)?
            .set_default("redis.cluster_enabled", false)?
            .set_default("redis.cluster_nodes", "")?
            .set_default("redis.cluster_read_from_replicas", false)?
            .set_default("redis.sentinel_enabled", false)?
            .set_default("redis.sentinel_nodes", "")?
            .set_default("redis.sentinel_master_name", "mymaster")?
            .set_default("pool.min_size", 2)?
            .set_default("pool.max_size", 10)?
            .set_default("pool.connect_timeout_ms", 5000)?
            .set_default("pool.command_timeout_ms", 5000)?
            .set_default("pool.idle_timeout_ms", 600_000)?
            .set_default("pubsub.max_subscriptions", 100)?
            .set_default("pubsub.connection_timeout_ms", 30000)?
            .set_default("blocking.max_timeout_seconds", 30)?
            .set_default("blocking.default_timeout_seconds", 5)?
            .set_default("blocking.max_sse_connections", 5)?
            .set_default("blocking.default_stream_read_count", 100)?
            .set_default("admin.api_key", "changeme-admin-key")?
            .set_default("rate_limit.enabled", false)?
            .set_default("rate_limit.requests_per_second", 100)?
            .set_default("rate_limit.burst_size", 50)?
            .set_default("log.level", "info")?
            .set_default("log.format", "json")?
            // Load from environment with double underscore separator for nested config
            // e.g., ADMIN__API_KEY maps to admin.api_key
            .add_source(Environment::default().separator("__").try_parsing(true))
            .build()?;

        let settings: Settings = config.try_deserialize()?;
        settings.validate()?;
        Ok(settings)
    }

    fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.blocking.max_timeout_seconds > 0,
            "blocking.max_timeout_seconds must be greater than 0"
        );
        anyhow::ensure!(
            self.blocking.default_timeout_seconds > 0,
            "blocking.default_timeout_seconds must be greater than 0"
        );
        anyhow::ensure!(
            self.blocking.default_timeout_seconds <= self.blocking.max_timeout_seconds,
            "blocking.default_timeout_seconds must be less than or equal to blocking.max_timeout_seconds"
        );
        anyhow::ensure!(
            self.blocking.max_sse_connections > 0,
            "blocking.max_sse_connections must be greater than 0"
        );

        if self.rate_limit.enabled {
            anyhow::ensure!(
                self.rate_limit.requests_per_second > 0,
                "rate_limit.requests_per_second must be greater than 0 when rate limiting is enabled"
            );
            anyhow::ensure!(
                self.rate_limit.burst_size > 0,
                "rate_limit.burst_size must be greater than 0 when rate limiting is enabled"
            );
        }

        // Cluster and sentinel are mutually exclusive
        anyhow::ensure!(
            !(self.redis.cluster_enabled && self.redis.sentinel_enabled),
            "cluster and sentinel modes are mutually exclusive"
        );

        if self.redis.cluster_enabled {
            anyhow::ensure!(
                !self.redis.cluster_node_urls().is_empty(),
                "redis.cluster_nodes must be non-empty when cluster is enabled"
            );
        }

        if self.redis.sentinel_enabled {
            anyhow::ensure!(
                !self.redis.sentinel_node_urls().is_empty(),
                "redis.sentinel_nodes must be non-empty when sentinel is enabled"
            );
            anyhow::ensure!(
                !self.redis.sentinel_master_name.is_empty(),
                "redis.sentinel_master_name must be non-empty when sentinel is enabled"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_server_config() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.request_timeout_ms, 30000);
    }

    #[test]
    fn test_default_redis_config() {
        let config = RedisConfig::default();
        assert_eq!(config.url, "redis://localhost:6379");
        assert_eq!(config.database, 0);
        assert!(!config.tls_enabled);
        assert!(!config.tls_skip_verify);
    }

    #[test]
    fn test_default_pool_config() {
        let config = PoolConfig::default();
        assert_eq!(config.min_size, 2);
        assert_eq!(config.max_size, 10);
        assert_eq!(config.connect_timeout_ms, 5000);
        assert_eq!(config.command_timeout_ms, 5000);
        assert_eq!(config.idle_timeout_ms, 600_000);
    }

    #[test]
    fn test_default_blocking_config() {
        let config = BlockingConfig::default();
        assert_eq!(config.max_timeout_seconds, 30);
        assert_eq!(config.default_timeout_seconds, 5);
    }

    #[test]
    fn test_default_pubsub_config() {
        let config = PubSubConfig::default();
        assert_eq!(config.max_subscriptions, 100);
        assert_eq!(config.connection_timeout_ms, 30000);
    }

    #[test]
    fn test_validate_rejects_zero_rate_limit_rps_when_enabled() {
        let settings = Settings {
            rate_limit: RateLimitConfig {
                enabled: true,
                requests_per_second: 0,
                burst_size: 50,
            },
            ..Settings::default()
        };

        let err = settings.validate().expect_err("validation should fail");
        assert!(
            err.to_string()
                .contains("rate_limit.requests_per_second must be greater than 0")
        );
    }

    #[test]
    fn test_validate_rejects_zero_rate_limit_burst_when_enabled() {
        let settings = Settings {
            rate_limit: RateLimitConfig {
                enabled: true,
                requests_per_second: 100,
                burst_size: 0,
            },
            ..Settings::default()
        };

        let err = settings.validate().expect_err("validation should fail");
        assert!(
            err.to_string()
                .contains("rate_limit.burst_size must be greater than 0")
        );
    }

    #[test]
    fn test_validate_allows_zero_values_when_rate_limit_disabled() {
        let settings = Settings {
            rate_limit: RateLimitConfig {
                enabled: false,
                requests_per_second: 0,
                burst_size: 0,
            },
            ..Settings::default()
        };

        settings.validate().expect("validation should pass");
    }

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.server.host, "0.0.0.0");
        assert_eq!(settings.redis.url, "redis://localhost:6379");
        assert_eq!(settings.admin.api_key, "changeme-admin-key");
        assert_eq!(settings.log.level, "info");
    }

    #[test]
    fn test_validate_rejects_cluster_and_sentinel_both_enabled() {
        let mut settings = Settings::default();
        settings.redis.cluster_enabled = true;
        settings.redis.cluster_nodes = "redis://n1:7001".to_string();
        settings.redis.sentinel_enabled = true;
        settings.redis.sentinel_nodes = "redis://s1:26379".to_string();

        let err = settings.validate().expect_err("validation should fail");
        assert!(err.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn test_validate_rejects_cluster_with_empty_nodes() {
        let mut settings = Settings::default();
        settings.redis.cluster_enabled = true;
        settings.redis.cluster_nodes = String::new();

        let err = settings.validate().expect_err("validation should fail");
        assert!(err.to_string().contains("cluster_nodes must be non-empty"));
    }

    #[test]
    fn test_validate_rejects_sentinel_with_empty_nodes() {
        let mut settings = Settings::default();
        settings.redis.sentinel_enabled = true;
        settings.redis.sentinel_nodes = String::new();

        let err = settings.validate().expect_err("validation should fail");
        assert!(err.to_string().contains("sentinel_nodes must be non-empty"));
    }

    #[test]
    fn test_validate_rejects_sentinel_with_empty_master_name() {
        let mut settings = Settings::default();
        settings.redis.sentinel_enabled = true;
        settings.redis.sentinel_nodes = "redis://s1:26379".to_string();
        settings.redis.sentinel_master_name = String::new();

        let err = settings.validate().expect_err("validation should fail");
        assert!(
            err.to_string()
                .contains("sentinel_master_name must be non-empty")
        );
    }

    #[test]
    fn test_validate_accepts_valid_cluster_config() {
        let mut settings = Settings::default();
        settings.redis.cluster_enabled = true;
        settings.redis.cluster_nodes =
            "redis://n1:7001,redis://n2:7002,redis://n3:7003".to_string();

        settings.validate().expect("validation should pass");
    }

    #[test]
    fn test_validate_accepts_valid_sentinel_config() {
        let mut settings = Settings::default();
        settings.redis.sentinel_enabled = true;
        settings.redis.sentinel_nodes = "redis://s1:26379,redis://s2:26380".to_string();
        settings.redis.sentinel_master_name = "mymaster".to_string();

        settings.validate().expect("validation should pass");
    }

    #[test]
    fn test_cluster_node_urls_parsing() {
        let config = RedisConfig {
            cluster_nodes: "redis://n1:7001, redis://n2:7002 , redis://n3:7003".to_string(),
            ..RedisConfig::default()
        };
        let urls = config.cluster_node_urls();
        assert_eq!(urls.len(), 3);
        assert_eq!(urls[0], "redis://n1:7001");
        assert_eq!(urls[1], "redis://n2:7002");
        assert_eq!(urls[2], "redis://n3:7003");
    }

    #[test]
    fn test_sentinel_node_urls_parsing() {
        let config = RedisConfig {
            sentinel_nodes: "redis://s1:26379,redis://s2:26380".to_string(),
            ..RedisConfig::default()
        };
        let urls = config.sentinel_node_urls();
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_default_redis_config_has_cluster_sentinel_disabled() {
        let config = RedisConfig::default();
        assert!(!config.cluster_enabled);
        assert!(!config.sentinel_enabled);
        assert!(config.cluster_nodes.is_empty());
        assert!(config.sentinel_nodes.is_empty());
        assert_eq!(config.sentinel_master_name, "mymaster");
    }

    #[test]
    fn test_settings_load_defaults() {
        let settings = Settings::load().expect("settings load");
        assert!(settings.server.port > 0);
        assert!(!settings.admin.api_key.is_empty());
    }
}
