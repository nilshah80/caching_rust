//! Common Schemas
//!
//! Shared request/response schemas used across endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Pagination parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u32,

    /// Items per page
    #[serde(default = "default_limit")]
    pub limit: u32,
}

const fn default_page() -> u32 {
    1
}

const fn default_limit() -> u32 {
    100
}

/// Basic scan/cursor parameters (use keys::ScanParams for extended key scanning)
#[derive(Debug, Deserialize, ToSchema)]
pub struct BasicScanParams {
    /// Cursor position (use "0" to start)
    #[serde(default = "default_cursor")]
    pub cursor: String,

    /// Pattern to match
    pub pattern: Option<String>,

    /// Maximum items to return
    #[serde(default = "default_count")]
    pub count: u32,
}

fn default_cursor() -> String {
    "0".to_string()
}

const fn default_count() -> u32 {
    10
}

/// TTL information response
#[derive(Debug, Serialize, ToSchema)]
pub struct TtlInfo {
    /// TTL in seconds (-1 if no expiry, -2 if key doesn't exist)
    pub ttl: i64,

    /// TTL in milliseconds
    pub pttl: i64,

    /// Whether key has expiry
    pub has_expiry: bool,
}

/// Key info response
#[derive(Debug, Serialize, ToSchema)]
pub struct KeyInfo {
    /// Key name
    pub key: String,

    /// Redis data type
    #[serde(rename = "type")]
    pub data_type: String,

    /// TTL in seconds (None if no expiry)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i64>,

    /// Internal encoding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_defaults() {
        assert_eq!(default_page(), 1);
        assert_eq!(default_limit(), 100);
        assert_eq!(default_cursor(), "0");
        assert_eq!(default_count(), 10);
    }
}
