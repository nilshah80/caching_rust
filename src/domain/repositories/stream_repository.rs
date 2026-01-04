//! Stream Repository Trait
//!
//! Abstract interface for Redis Stream operations.

use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;

use crate::domain::entities::{
    AutoClaimResult, ClaimResult, ConsumerGroupInfo, ConsumerInfo, PendingEntry, PendingSummary,
    StreamEntry, StreamInfo, StreamReadResult, XAddOptions, XAutoClaimOptions, XClaimOptions,
    XGroupCreateOptions, XPendingOptions, XReadGroupOptions, XReadOptions, XTrimStrategy,
};
use crate::domain::errors::CacheError;

/// Repository trait for Redis Stream operations
#[async_trait]
pub trait StreamRepository: Send + Sync {
    // ========== Basic Stream Operations ==========

    /// XADD - Add entry to stream
    /// Returns the ID of the added entry
    async fn xadd(
        &self,
        key: &str,
        fields: &HashMap<String, String>,
        options: XAddOptions,
    ) -> Result<String, CacheError>;

    /// XLEN - Get the number of entries in the stream
    async fn xlen(&self, key: &str) -> Result<i64, CacheError>;

    /// XRANGE - Get entries in a range (inclusive)
    /// Use "-" for start and "+" for end to get all entries
    async fn xrange(
        &self,
        key: &str,
        start: &str,
        end: &str,
        count: Option<i64>,
    ) -> Result<Vec<StreamEntry>, CacheError>;

    /// XREVRANGE - Get entries in reverse order
    async fn xrevrange(
        &self,
        key: &str,
        end: &str,
        start: &str,
        count: Option<i64>,
    ) -> Result<Vec<StreamEntry>, CacheError>;

    /// XDEL - Delete entries from stream
    /// Returns the number of entries deleted
    async fn xdel(&self, key: &str, ids: &[String]) -> Result<i64, CacheError>;

    /// XTRIM - Trim stream to specified length or ID
    /// Returns the number of entries removed
    async fn xtrim(&self, key: &str, strategy: XTrimStrategy) -> Result<i64, CacheError>;

    /// XINFO STREAM - Get information about a stream
    async fn xinfo_stream(&self, key: &str, full: bool) -> Result<StreamInfo, CacheError>;

    // ========== Read Operations ==========

    /// XREAD - Read entries from one or more streams
    /// Returns None if no entries are available and blocking times out
    async fn xread(
        &self,
        streams: &[(String, String)], // (key, last_id) pairs
        options: XReadOptions,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError>;

    /// XREAD with blocking and enforced maximum timeout
    /// Returns None if timeout is reached with no data
    async fn xread_blocking(
        &self,
        streams: &[(String, String)],
        count: Option<i64>,
        timeout: Duration,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError>;

    // ========== Consumer Group Operations ==========

    /// XGROUP CREATE - Create a consumer group
    async fn xgroup_create(
        &self,
        key: &str,
        group: &str,
        id: &str,
        options: XGroupCreateOptions,
    ) -> Result<(), CacheError>;

    /// XGROUP DESTROY - Delete a consumer group
    /// Returns true if the group was destroyed
    async fn xgroup_destroy(&self, key: &str, group: &str) -> Result<bool, CacheError>;

    /// XGROUP SETID - Set the last delivered ID of a consumer group
    async fn xgroup_setid(
        &self,
        key: &str,
        group: &str,
        id: &str,
        entries_read: Option<i64>,
    ) -> Result<(), CacheError>;

    /// XGROUP CREATECONSUMER - Create a consumer in a group
    /// Returns true if the consumer was created (false if already exists)
    async fn xgroup_createconsumer(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
    ) -> Result<bool, CacheError>;

    /// XGROUP DELCONSUMER - Delete a consumer from a group
    /// Returns the number of pending messages that were deleted
    async fn xgroup_delconsumer(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
    ) -> Result<i64, CacheError>;

    /// XINFO GROUPS - Get information about consumer groups
    async fn xinfo_groups(&self, key: &str) -> Result<Vec<ConsumerGroupInfo>, CacheError>;

    /// XINFO CONSUMERS - Get information about consumers in a group
    async fn xinfo_consumers(
        &self,
        key: &str,
        group: &str,
    ) -> Result<Vec<ConsumerInfo>, CacheError>;

    // ========== Consumer Group Read Operations ==========

    /// XREADGROUP - Read entries as a consumer in a group
    /// Use ">" for id to get only new entries never delivered to any consumer
    /// Returns None if no entries are available and blocking times out
    async fn xreadgroup(
        &self,
        group: &str,
        consumer: &str,
        streams: &[(String, String)], // (key, id) pairs
        options: XReadGroupOptions,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError>;

    /// XREADGROUP with blocking and enforced maximum timeout
    async fn xreadgroup_blocking(
        &self,
        group: &str,
        consumer: &str,
        streams: &[(String, String)],
        count: Option<i64>,
        no_ack: bool,
        timeout: Duration,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError>;

    /// XACK - Acknowledge one or more entries
    /// Returns the number of entries acknowledged
    async fn xack(&self, key: &str, group: &str, ids: &[String]) -> Result<i64, CacheError>;

    // ========== Pending Entry Operations ==========

    /// XPENDING - Get pending entries summary
    async fn xpending_summary(&self, key: &str, group: &str) -> Result<PendingSummary, CacheError>;

    /// XPENDING - Get pending entries with details
    async fn xpending(
        &self,
        key: &str,
        group: &str,
        options: XPendingOptions,
    ) -> Result<Vec<PendingEntry>, CacheError>;

    /// XCLAIM - Claim pending entries
    async fn xclaim(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
        ids: &[String],
        options: XClaimOptions,
    ) -> Result<ClaimResult, CacheError>;

    /// XAUTOCLAIM - Automatically claim pending entries older than min_idle_time
    async fn xautoclaim(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
        min_idle_time_ms: i64,
        start: &str,
        options: XAutoClaimOptions,
    ) -> Result<AutoClaimResult, CacheError>;

    // ========== Stream Management ==========

    /// XSETID - Set the last ID of a stream (admin operation)
    async fn xsetid(
        &self,
        key: &str,
        last_id: &str,
        entries_added: Option<i64>,
        max_deleted_id: Option<&str>,
    ) -> Result<(), CacheError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xread_options_default() {
        let opts = XReadOptions::default();
        assert!(opts.count.is_none());
        assert!(opts.block_ms.is_none());
    }

    #[test]
    fn test_xreadgroup_options_default() {
        let opts = XReadGroupOptions::default();
        assert!(opts.count.is_none());
        assert!(opts.block_ms.is_none());
        assert!(!opts.no_ack);
    }

    #[test]
    fn test_xclaim_options_default() {
        let opts = XClaimOptions::default();
        assert_eq!(opts.min_idle_time_ms, 0);
        assert!(!opts.force);
        assert!(!opts.just_id);
    }
}
