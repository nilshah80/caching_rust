//! Redis Capabilities Detection
//!
//! Detects available Redis features and modules at startup.

use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// Redis server capabilities detected at startup
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RedisCapabilities {
    /// Redis server version
    pub redis_version: String,

    /// Available Redis modules
    pub modules: ModuleCapabilities,

    /// Available Redis features based on version
    pub features: FeatureCapabilities,

    /// When capabilities were detected
    pub detected_at: DateTime<Utc>,
}

/// Available Redis modules
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct ModuleCapabilities {
    /// ReJSON / RedisJSON module
    pub json: bool,

    /// RediSearch module
    pub search: bool,

    /// RedisBloom module (BF, CF, CMS, TopK)
    pub bloom: bool,

    /// RedisTimeSeries module
    pub timeseries: bool,

    /// RedisGraph module (deprecated but may exist)
    pub graph: bool,
}

/// Available Redis features based on version
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FeatureCapabilities {
    /// Streams support (Redis 5.0+)
    pub streams: bool,

    /// ACL support (Redis 6.0+)
    pub acl: bool,

    /// Functions support (Redis 7.0+)
    pub functions: bool,

    /// Cluster mode enabled
    pub cluster: bool,
}

impl RedisCapabilities {
    /// Create default capabilities (used when detection fails)
    pub fn default_capabilities() -> Self {
        Self {
            redis_version: "unknown".to_string(),
            modules: ModuleCapabilities {
                json: false,
                search: false,
                bloom: false,
                timeseries: false,
                graph: false,
            },
            features: FeatureCapabilities {
                streams: true, // Assume modern Redis
                acl: true,
                functions: false,
                cluster: false,
            },
            detected_at: Utc::now(),
        }
    }

    /// Parse Redis version from INFO output
    pub fn parse_version(info: &str) -> String {
        for line in info.lines() {
            if line.starts_with("redis_version:") {
                return line
                    .trim_start_matches("redis_version:")
                    .trim()
                    .to_string();
            }
        }
        "unknown".to_string()
    }

    /// Check if version is greater than or equal to target
    pub fn version_gte(version: &str, target: &str) -> bool {
        let parse_version = |v: &str| -> Vec<u32> {
            v.split('.')
                .filter_map(|s| s.parse().ok())
                .collect()
        };

        let current = parse_version(version);
        let target = parse_version(target);

        for (c, t) in current.iter().zip(target.iter()) {
            match c.cmp(t) {
                std::cmp::Ordering::Greater => return true,
                std::cmp::Ordering::Less => return false,
                std::cmp::Ordering::Equal => continue,
            }
        }
        current.len() >= target.len()
    }

    /// Detect module from MODULE LIST output
    pub fn detect_module(modules: &[Vec<String>], name_pattern: &str) -> bool {
        modules.iter().any(|module| {
            module.iter().any(|field| {
                field.to_lowercase().contains(name_pattern)
            })
        })
    }
}

impl Default for FeatureCapabilities {
    fn default() -> Self {
        Self {
            streams: true,
            acl: true,
            functions: false,
            cluster: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let info = "# Server\nredis_version:7.2.4\nredis_git_sha1:00000000";
        assert_eq!(RedisCapabilities::parse_version(info), "7.2.4");
    }

    #[test]
    fn test_version_comparison() {
        assert!(RedisCapabilities::version_gte("7.2.4", "7.0.0"));
        assert!(RedisCapabilities::version_gte("7.2.4", "7.2.4"));
        assert!(!RedisCapabilities::version_gte("6.2.0", "7.0.0"));
        assert!(RedisCapabilities::version_gte("7.0.0", "6.0.0"));
    }
}
