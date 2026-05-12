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

    /// LCS command support (Redis 7.0+)
    pub lcs: bool,

    /// COMMAND DOCS/LIST support (Redis 7.0+)
    pub command_docs: bool,

    /// Hash field expiration support (Redis 7.4+)
    pub hash_field_expiration: bool,

    /// Redis 8.0 hash commands (HGETEX, HSETEX, HGETDEL)
    pub hash_8_commands: bool,

    /// Redis 8.4 string commands (MSETEX, DELEX, DIGEST)
    pub string_8_4_commands: bool,

    /// LATENCY HISTOGRAM support (Redis 7.0+)
    pub latency_histogram: bool,

    /// CLUSTER SLOT-STATS support (Redis 8.2+, requires cluster mode)
    pub cluster_slot_stats: bool,

    /// XACKDEL stream command (Redis 8.2+) — atomic acknowledge + delete
    pub xackdel: bool,

    /// XDELEX stream command (Redis 8.2+) — XDEL with reference policy
    pub xdelex: bool,

    /// XADD/XTRIM `KEEPREF | DELREF | ACKED` options (Redis 8.2+).
    pub stream_reference_policy: bool,

    /// XADD idempotent options + XCFGSET (Redis 8.6+).
    pub stream_idmp: bool,

    /// HOTKEYS START/STOP/GET/RESET hot-key sampling (Redis 8.6+).
    pub hotkeys: bool,

    /// Redis 8.0 vector sets commands (VADD, VSIM, etc.)
    pub vectors: bool,

    /// VRANGE command support (may be absent on early 8.x builds)
    pub vector_range: bool,

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
                lcs: true,
                command_docs: true,
                hash_field_expiration: true,
                hash_8_commands: true,
                string_8_4_commands: false, // Only enabled when Redis ≥ 8.4
                latency_histogram: false,   // Only enabled when Redis ≥ 7.0
                cluster_slot_stats: false,  // Only enabled in cluster mode on Redis ≥ 8.2
                xackdel: false,             // Only enabled when Redis ≥ 8.2
                xdelex: false,              // Only enabled when Redis ≥ 8.2
                stream_reference_policy: false, // Only enabled when Redis ≥ 8.2
                stream_idmp: false,         // Only enabled when Redis ≥ 8.6
                hotkeys: false,             // Only enabled when Redis ≥ 8.6
                vectors: false,             // Only enabled after positive COMMAND INFO VADD probe
                vector_range: false,        // Only enabled after positive COMMAND INFO VRANGE probe
                cluster: false,
            },
            detected_at: Utc::now(),
        }
    }

    /// Parse Redis version from INFO output
    pub fn parse_version(info: &str) -> String {
        for line in info.lines() {
            if line.starts_with("redis_version:") {
                return line.trim_start_matches("redis_version:").trim().to_string();
            }
        }
        "unknown".to_string()
    }

    /// Check if version is greater than or equal to target
    pub fn version_gte(version: &str, target: &str) -> bool {
        let parse_version =
            |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };

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
            module
                .iter()
                .any(|field| field.to_lowercase().contains(name_pattern))
        })
    }
}

impl Default for FeatureCapabilities {
    fn default() -> Self {
        Self {
            streams: true,
            acl: true,
            functions: false,
            lcs: true,
            command_docs: true,
            hash_field_expiration: true,
            hash_8_commands: true,
            string_8_4_commands: false, // Only enabled when Redis ≥ 8.4
            latency_histogram: false,   // Only enabled when Redis ≥ 7.0
            cluster_slot_stats: false,  // Only enabled in cluster mode on Redis ≥ 8.2
            xackdel: false,             // Only enabled when Redis ≥ 8.2
            xdelex: false,              // Only enabled when Redis ≥ 8.2
            stream_reference_policy: false, // Only enabled when Redis ≥ 8.2
            stream_idmp: false,         // Only enabled when Redis ≥ 8.6
            hotkeys: false,             // Only enabled when Redis ≥ 8.6
            vectors: false,             // Only enabled after positive COMMAND INFO VADD probe
            vector_range: false,        // Only enabled after positive COMMAND INFO VRANGE probe
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

    #[test]
    fn test_detect_module() {
        let modules = vec![
            vec!["name".to_string(), "ReJSON".to_string()],
            vec!["name".to_string(), "search".to_string()],
        ];
        assert!(RedisCapabilities::detect_module(&modules, "rejson"));
        assert!(RedisCapabilities::detect_module(&modules, "search"));
        assert!(!RedisCapabilities::detect_module(&modules, "timeseries"));
    }

    #[test]
    fn test_default_capabilities() {
        let caps = RedisCapabilities::default_capabilities();
        assert_eq!(caps.redis_version, "unknown");
        assert!(!caps.modules.json);
        assert!(caps.features.streams);
    }

    #[test]
    fn test_feature_capabilities_default() {
        let caps = FeatureCapabilities::default();
        assert!(caps.streams);
        assert!(caps.acl);
        assert!(!caps.functions);
        assert!(caps.hash_8_commands);
        assert!(!caps.vectors); // Only true after positive COMMAND INFO VADD probe
        assert!(!caps.cluster);
    }

    #[test]
    fn test_parse_version_missing() {
        let info = "# Server\nredis_git_sha1:00000000";
        assert_eq!(RedisCapabilities::parse_version(info), "unknown");
    }

    #[test]
    fn test_version_comparison_shorter_current() {
        assert!(!RedisCapabilities::version_gte("7.0", "7.0.1"));
    }

    // --- Version parsing edge cases ---

    #[test]
    fn test_parse_version_with_prerelease_suffix() {
        let info = "# Server\nredis_version:7.2.4-rc1\nredis_git_sha1:00000000";
        assert_eq!(RedisCapabilities::parse_version(info), "7.2.4-rc1");
    }

    #[test]
    fn test_parse_version_empty_string() {
        assert_eq!(RedisCapabilities::parse_version(""), "unknown");
    }

    #[test]
    fn test_parse_version_multiline_finds_version() {
        let info = "# Server\r\nos:Linux\r\nredis_version:6.2.7\r\nredis_mode:standalone\r\n";
        assert_eq!(RedisCapabilities::parse_version(info), "6.2.7");
    }

    // --- Version comparison edge cases ---

    #[test]
    fn test_version_gte_equal_versions() {
        assert!(RedisCapabilities::version_gte("7.0.0", "7.0.0"));
    }

    #[test]
    fn test_version_gte_single_digit_current() {
        // "7" parses as [7], target "7.0.0" parses as [7, 0, 0].
        // After zip comparison all equal, but current.len() (1) < target.len() (3) => false.
        assert!(!RedisCapabilities::version_gte("7", "7.0.0"));
    }

    #[test]
    fn test_version_gte_major_dominates() {
        assert!(RedisCapabilities::version_gte("8.0.0", "7.9.9"));
    }

    #[test]
    fn test_version_gte_patch_matters() {
        assert!(!RedisCapabilities::version_gte("7.0.1", "7.0.2"));
    }

    #[test]
    fn test_version_gte_prerelease_ignored_in_parse() {
        // "7.2.4-rc1" — the "4-rc1" segment fails u32 parse, so version becomes [7, 2].
        // Target "7.2.4" becomes [7, 2, 4]. After zip: equal, but len 2 < 3 => false.
        assert!(!RedisCapabilities::version_gte("7.2.4-rc1", "7.2.4"));
    }

    // --- Module detection edge cases ---

    #[test]
    fn test_detect_module_empty_list() {
        let modules: Vec<Vec<String>> = vec![];
        assert!(!RedisCapabilities::detect_module(&modules, "rejson"));
    }

    #[test]
    fn test_detect_module_case_insensitive() {
        let modules = vec![vec!["name".to_string(), "ReJSON".to_string()]];
        assert!(RedisCapabilities::detect_module(&modules, "rejson"));
    }

    #[test]
    fn test_detect_module_name_at_different_position() {
        // Module info where the name field is not at index 1
        let modules = vec![vec![
            "ver".to_string(),
            "2".to_string(),
            "name".to_string(),
            "search".to_string(),
        ]];
        // detect_module checks any field, so "search" will be found
        assert!(RedisCapabilities::detect_module(&modules, "search"));
    }

    // --- Feature capabilities from version ---

    #[test]
    fn test_features_redis_4x() {
        let v = "4.0.14";
        assert!(!RedisCapabilities::version_gte(v, "5.0.0")); // streams
        assert!(!RedisCapabilities::version_gte(v, "6.0.0")); // acl
        assert!(!RedisCapabilities::version_gte(v, "7.0.0")); // functions
    }

    #[test]
    fn test_features_redis_5() {
        let v = "5.0.0";
        assert!(RedisCapabilities::version_gte(v, "5.0.0")); // streams
        assert!(!RedisCapabilities::version_gte(v, "6.0.0")); // acl
        assert!(!RedisCapabilities::version_gte(v, "7.0.0")); // functions
    }

    #[test]
    fn test_features_redis_6() {
        let v = "6.0.0";
        assert!(RedisCapabilities::version_gte(v, "5.0.0")); // streams
        assert!(RedisCapabilities::version_gte(v, "6.0.0")); // acl
        assert!(!RedisCapabilities::version_gte(v, "7.0.0")); // functions
    }

    #[test]
    fn test_features_redis_7() {
        let v = "7.0.0";
        assert!(RedisCapabilities::version_gte(v, "5.0.0")); // streams
        assert!(RedisCapabilities::version_gte(v, "6.0.0")); // acl
        assert!(RedisCapabilities::version_gte(v, "7.0.0")); // functions
        assert!(!RedisCapabilities::version_gte(v, "8.0.0")); // hash_8_commands
    }

    #[test]
    fn test_features_redis_8() {
        let v = "8.0.0";
        assert!(RedisCapabilities::version_gte(v, "5.0.0")); // streams
        assert!(RedisCapabilities::version_gte(v, "6.0.0")); // acl
        assert!(RedisCapabilities::version_gte(v, "7.0.0")); // functions
        assert!(RedisCapabilities::version_gte(v, "7.4.0")); // hash_field_expiration
        assert!(RedisCapabilities::version_gte(v, "8.0.0")); // hash_8_commands
    }
}
