//! Application Settings
//!
//! Configuration loaded from environment variables.

use serde::Deserialize;
use config::{Config, Environment};

/// Root settings structure
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub redis: RedisConfig,
    pub pool: PoolConfig,
    pub pubsub: PubSubConfig,
    pub blocking: BlockingConfig,
    pub admin: AdminConfig,
    pub log: LogConfig,
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

    /// Idle timeout for subscriptions in milliseconds (default: 300000 = 5 minutes)
    #[serde(default = "default_pubsub_idle_timeout")]
    pub idle_timeout_ms: u64,
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
}

/// Admin API configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AdminConfig {
    /// API key required for admin endpoints
    pub api_key: String,
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

fn default_pubsub_idle_timeout() -> u64 {
    300_000
}

fn default_max_blocking_timeout() -> u32 {
    30
}

fn default_blocking_timeout() -> u32 {
    5
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
        }
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
            idle_timeout_ms: default_pubsub_idle_timeout(),
        }
    }
}

impl Default for BlockingConfig {
    fn default() -> Self {
        Self {
            max_timeout_seconds: default_max_blocking_timeout(),
            default_timeout_seconds: default_blocking_timeout(),
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

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            redis: RedisConfig::default(),
            pool: PoolConfig::default(),
            pubsub: PubSubConfig::default(),
            blocking: BlockingConfig::default(),
            admin: AdminConfig::default(),
            log: LogConfig::default(),
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
            .set_default("redis.url", "redis://localhost:6379")?
            .set_default("redis.database", 0)?
            .set_default("redis.tls_enabled", false)?
            .set_default("redis.tls_skip_verify", false)?
            .set_default("pool.min_size", 2)?
            .set_default("pool.max_size", 10)?
            .set_default("pool.connect_timeout_ms", 5000)?
            .set_default("pool.command_timeout_ms", 5000)?
            .set_default("pool.idle_timeout_ms", 600_000)?
            .set_default("pubsub.max_subscriptions", 100)?
            .set_default("pubsub.connection_timeout_ms", 30000)?
            .set_default("pubsub.idle_timeout_ms", 300_000)?
            .set_default("blocking.max_timeout_seconds", 30)?
            .set_default("blocking.default_timeout_seconds", 5)?
            .set_default("admin.api_key", "changeme-admin-key")?
            .set_default("log.level", "info")?
            .set_default("log.format", "json")?
            // Load from environment with double underscore separator for nested config
            // e.g., ADMIN__API_KEY maps to admin.api_key
            .add_source(
                Environment::default()
                    .separator("__")
                    .try_parsing(true)
            )
            .build()?;

        let settings: Settings = config.try_deserialize()?;
        Ok(settings)
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
        assert_eq!(config.idle_timeout_ms, 300_000);
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
    fn test_settings_load_defaults() {
        let settings = Settings::load().expect("settings load");
        assert!(settings.server.port > 0);
        assert!(!settings.admin.api_key.is_empty());
    }
}
