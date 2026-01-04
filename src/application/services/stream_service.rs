//! Stream Service
//!
//! Business logic for Redis Stream operations.
//! Follows Architecture Decision 3: Blocking commands enforce max 30s timeout.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::domain::entities::{
    AutoClaimResult, ClaimResult, ConsumerGroupInfo, ConsumerInfo, PendingEntry, PendingSummary,
    StreamEntry, StreamInfo, StreamReadResult, XAddOptions, XAutoClaimOptions, XClaimOptions,
    XGroupCreateOptions, XPendingOptions, XReadGroupOptions, XReadOptions, XTrimStrategy,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::StreamRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisStreamRepository;

/// Maximum allowed timeout for blocking operations (30 seconds) - Architecture Decision 3
const MAX_BLOCKING_TIMEOUT_SECONDS: u64 = 30;

/// Default blocking timeout for SSE streaming iterations
const DEFAULT_SSE_BLOCK_MS: i64 = 5000;

/// Service for stream operations
pub struct StreamService {
    repository: Arc<dyn StreamRepository>,
    max_blocking_timeout: Duration,
}

impl StreamService {
    /// Create a new StreamService with default Redis repository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisStreamRepository::new(pool)))
    }

    /// Create a new StreamService with custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn StreamRepository>) -> Self {
        Self {
            repository,
            max_blocking_timeout: Duration::from_secs(MAX_BLOCKING_TIMEOUT_SECONDS),
        }
    }

    /// Set custom max blocking timeout (for testing or configuration)
    #[allow(dead_code)]
    pub fn with_max_blocking_timeout(mut self, timeout: Duration) -> Self {
        self.max_blocking_timeout = timeout;
        self
    }

    /// Enforce maximum timeout for blocking operations (Architecture Decision 3)
    fn enforce_timeout(&self, requested: Duration) -> Duration {
        if requested > self.max_blocking_timeout {
            self.max_blocking_timeout
        } else {
            requested
        }
    }

    /// Enforce maximum block_ms for XREAD/XREADGROUP options
    /// Clamps negative values to 0 and applies max_blocking_timeout
    fn enforce_block_ms(&self, block_ms: Option<i64>) -> Option<i64> {
        block_ms.map(|ms| {
            let max_ms = self.max_blocking_timeout.as_millis() as i64;
            // Clamp negative values to 0, then apply max
            ms.max(0).min(max_ms)
        })
    }

    // ========== Basic Stream Operations ==========

    /// XADD - Add entry to stream
    /// Returns the ID of the added entry
    pub async fn xadd(
        &self,
        key: &str,
        fields: HashMap<String, String>,
        options: XAddOptions,
    ) -> Result<String, CacheError> {
        if fields.is_empty() {
            return Err(CacheError::InvalidInput(
                "Fields cannot be empty".to_string(),
            ));
        }
        self.repository.xadd(key, &fields, options).await
    }

    /// XLEN - Get the number of entries in the stream
    pub async fn xlen(&self, key: &str) -> Result<i64, CacheError> {
        self.repository.xlen(key).await
    }

    /// XRANGE - Get entries in a range (inclusive)
    /// Use "-" for start and "+" for end to get all entries
    pub async fn xrange(
        &self,
        key: &str,
        start: &str,
        end: &str,
        count: Option<i64>,
    ) -> Result<Vec<StreamEntry>, CacheError> {
        self.repository.xrange(key, start, end, count).await
    }

    /// XREVRANGE - Get entries in reverse order
    pub async fn xrevrange(
        &self,
        key: &str,
        end: &str,
        start: &str,
        count: Option<i64>,
    ) -> Result<Vec<StreamEntry>, CacheError> {
        self.repository.xrevrange(key, end, start, count).await
    }

    /// XDEL - Delete entries from stream
    /// Returns the number of entries deleted
    pub async fn xdel(&self, key: &str, ids: Vec<String>) -> Result<i64, CacheError> {
        if ids.is_empty() {
            return Err(CacheError::InvalidInput("IDs cannot be empty".to_string()));
        }
        self.repository.xdel(key, &ids).await
    }

    /// XTRIM - Trim stream to specified length or ID
    /// Returns the number of entries removed
    pub async fn xtrim(&self, key: &str, strategy: XTrimStrategy) -> Result<i64, CacheError> {
        self.repository.xtrim(key, strategy).await
    }

    /// XINFO STREAM - Get information about a stream
    pub async fn xinfo_stream(&self, key: &str, full: bool) -> Result<StreamInfo, CacheError> {
        self.repository.xinfo_stream(key, full).await
    }

    // ========== Read Operations ==========

    /// XREAD - Read entries from one or more streams (non-blocking or with enforced timeout)
    pub async fn xread(
        &self,
        streams: Vec<(String, String)>,
        options: XReadOptions,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
        if streams.is_empty() {
            return Err(CacheError::InvalidInput(
                "Streams cannot be empty".to_string(),
            ));
        }

        // Enforce max blocking timeout
        let mut enforced_options = options;
        enforced_options.block_ms = self.enforce_block_ms(enforced_options.block_ms);

        self.repository.xread(&streams, enforced_options).await
    }

    /// XREAD with blocking and enforced maximum timeout (Architecture Decision 3)
    /// Returns None if timeout is reached with no data
    pub async fn xread_blocking(
        &self,
        streams: Vec<(String, String)>,
        count: Option<i64>,
        timeout_seconds: u32,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
        if streams.is_empty() {
            return Err(CacheError::InvalidInput(
                "Streams cannot be empty".to_string(),
            ));
        }

        let timeout = self.enforce_timeout(Duration::from_secs(timeout_seconds as u64));
        self.repository
            .xread_blocking(&streams, count, timeout)
            .await
    }

    /// Get default block time for SSE streaming iterations
    pub fn default_sse_block_ms() -> i64 {
        DEFAULT_SSE_BLOCK_MS
    }

    // ========== Consumer Group Operations (Admin Protected) ==========

    /// XGROUP CREATE - Create a consumer group
    pub async fn xgroup_create(
        &self,
        key: &str,
        group: &str,
        id: &str,
        options: XGroupCreateOptions,
    ) -> Result<(), CacheError> {
        if group.is_empty() {
            return Err(CacheError::InvalidInput(
                "Group name cannot be empty".to_string(),
            ));
        }
        self.repository.xgroup_create(key, group, id, options).await
    }

    /// XGROUP DESTROY - Delete a consumer group
    /// Returns true if the group was destroyed
    pub async fn xgroup_destroy(&self, key: &str, group: &str) -> Result<bool, CacheError> {
        self.repository.xgroup_destroy(key, group).await
    }

    /// XGROUP SETID - Set the last delivered ID of a consumer group
    pub async fn xgroup_setid(
        &self,
        key: &str,
        group: &str,
        id: &str,
        entries_read: Option<i64>,
    ) -> Result<(), CacheError> {
        self.repository
            .xgroup_setid(key, group, id, entries_read)
            .await
    }

    /// XGROUP CREATECONSUMER - Create a consumer in a group
    /// Returns true if the consumer was created (false if already exists)
    pub async fn xgroup_createconsumer(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
    ) -> Result<bool, CacheError> {
        if consumer.is_empty() {
            return Err(CacheError::InvalidInput(
                "Consumer name cannot be empty".to_string(),
            ));
        }
        self.repository
            .xgroup_createconsumer(key, group, consumer)
            .await
    }

    /// XGROUP DELCONSUMER - Delete a consumer from a group
    /// Returns the number of pending messages that were deleted
    pub async fn xgroup_delconsumer(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
    ) -> Result<i64, CacheError> {
        self.repository
            .xgroup_delconsumer(key, group, consumer)
            .await
    }

    /// XINFO GROUPS - Get information about consumer groups
    pub async fn xinfo_groups(&self, key: &str) -> Result<Vec<ConsumerGroupInfo>, CacheError> {
        self.repository.xinfo_groups(key).await
    }

    /// XINFO CONSUMERS - Get information about consumers in a group
    pub async fn xinfo_consumers(
        &self,
        key: &str,
        group: &str,
    ) -> Result<Vec<ConsumerInfo>, CacheError> {
        self.repository.xinfo_consumers(key, group).await
    }

    // ========== Consumer Group Read Operations ==========

    /// XREADGROUP - Read entries as a consumer in a group (non-blocking or with enforced timeout)
    /// Use ">" for id to get only new entries never delivered to any consumer
    pub async fn xreadgroup(
        &self,
        group: &str,
        consumer: &str,
        streams: Vec<(String, String)>,
        options: XReadGroupOptions,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
        if streams.is_empty() {
            return Err(CacheError::InvalidInput(
                "Streams cannot be empty".to_string(),
            ));
        }
        if group.is_empty() || consumer.is_empty() {
            return Err(CacheError::InvalidInput(
                "Group and consumer names cannot be empty".to_string(),
            ));
        }

        // Enforce max blocking timeout
        let mut enforced_options = options;
        enforced_options.block_ms = self.enforce_block_ms(enforced_options.block_ms);

        self.repository
            .xreadgroup(group, consumer, &streams, enforced_options)
            .await
    }

    /// XREADGROUP with blocking and enforced maximum timeout (Architecture Decision 3)
    /// Returns None if timeout is reached with no data
    pub async fn xreadgroup_blocking(
        &self,
        group: &str,
        consumer: &str,
        streams: Vec<(String, String)>,
        count: Option<i64>,
        no_ack: bool,
        timeout_seconds: u32,
    ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
        if streams.is_empty() {
            return Err(CacheError::InvalidInput(
                "Streams cannot be empty".to_string(),
            ));
        }
        if group.is_empty() || consumer.is_empty() {
            return Err(CacheError::InvalidInput(
                "Group and consumer names cannot be empty".to_string(),
            ));
        }

        let timeout = self.enforce_timeout(Duration::from_secs(timeout_seconds as u64));
        self.repository
            .xreadgroup_blocking(group, consumer, &streams, count, no_ack, timeout)
            .await
    }

    /// XACK - Acknowledge one or more entries
    /// Returns the number of entries acknowledged
    pub async fn xack(&self, key: &str, group: &str, ids: Vec<String>) -> Result<i64, CacheError> {
        if ids.is_empty() {
            return Err(CacheError::InvalidInput("IDs cannot be empty".to_string()));
        }
        self.repository.xack(key, group, &ids).await
    }

    // ========== Pending Entry Operations ==========

    /// XPENDING - Get pending entries summary
    pub async fn xpending_summary(
        &self,
        key: &str,
        group: &str,
    ) -> Result<PendingSummary, CacheError> {
        self.repository.xpending_summary(key, group).await
    }

    /// XPENDING - Get pending entries with details
    pub async fn xpending(
        &self,
        key: &str,
        group: &str,
        options: XPendingOptions,
    ) -> Result<Vec<PendingEntry>, CacheError> {
        self.repository.xpending(key, group, options).await
    }

    /// XCLAIM - Claim pending entries
    pub async fn xclaim(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
        ids: Vec<String>,
        options: XClaimOptions,
    ) -> Result<ClaimResult, CacheError> {
        if ids.is_empty() {
            return Err(CacheError::InvalidInput("IDs cannot be empty".to_string()));
        }
        if consumer.is_empty() {
            return Err(CacheError::InvalidInput(
                "Consumer name cannot be empty".to_string(),
            ));
        }
        self.repository
            .xclaim(key, group, consumer, &ids, options)
            .await
    }

    /// XAUTOCLAIM - Automatically claim pending entries older than min_idle_time
    pub async fn xautoclaim(
        &self,
        key: &str,
        group: &str,
        consumer: &str,
        min_idle_time_ms: i64,
        start: &str,
        options: XAutoClaimOptions,
    ) -> Result<AutoClaimResult, CacheError> {
        if consumer.is_empty() {
            return Err(CacheError::InvalidInput(
                "Consumer name cannot be empty".to_string(),
            ));
        }
        self.repository
            .xautoclaim(key, group, consumer, min_idle_time_ms, start, options)
            .await
    }

    // ========== Stream Management ==========

    /// XSETID - Set the last ID of a stream (admin operation)
    pub async fn xsetid(
        &self,
        key: &str,
        last_id: &str,
        entries_added: Option<i64>,
        max_deleted_id: Option<&str>,
    ) -> Result<(), CacheError> {
        self.repository
            .xsetid(key, last_id, entries_added, max_deleted_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock StreamRepository for testing
    struct MockStreamRepository;

    #[async_trait::async_trait]
    impl StreamRepository for MockStreamRepository {
        async fn xadd(
            &self,
            _key: &str,
            _fields: &HashMap<String, String>,
            _options: XAddOptions,
        ) -> Result<String, CacheError> {
            Ok("1704000001234-0".to_string())
        }

        async fn xlen(&self, _key: &str) -> Result<i64, CacheError> {
            Ok(10)
        }

        async fn xrange(
            &self,
            _key: &str,
            _start: &str,
            _end: &str,
            _count: Option<i64>,
        ) -> Result<Vec<StreamEntry>, CacheError> {
            Ok(vec![])
        }

        async fn xrevrange(
            &self,
            _key: &str,
            _end: &str,
            _start: &str,
            _count: Option<i64>,
        ) -> Result<Vec<StreamEntry>, CacheError> {
            Ok(vec![])
        }

        async fn xdel(&self, _key: &str, ids: &[String]) -> Result<i64, CacheError> {
            Ok(ids.len() as i64)
        }

        async fn xtrim(&self, _key: &str, _strategy: XTrimStrategy) -> Result<i64, CacheError> {
            Ok(5)
        }

        async fn xinfo_stream(&self, _key: &str, _full: bool) -> Result<StreamInfo, CacheError> {
            Ok(StreamInfo {
                length: 10,
                groups: 2,
                first_entry_id: Some("1704000001234-0".to_string()),
                last_entry_id: Some("1704000001235-0".to_string()),
                first_entry: None,
                last_entry: None,
                max_deleted_entry_id: None,
                entries_added: Some(100),
                radix_tree_keys: None,
                radix_tree_nodes: None,
            })
        }

        async fn xread(
            &self,
            _streams: &[(String, String)],
            _options: XReadOptions,
        ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
            Ok(Some(vec![]))
        }

        async fn xread_blocking(
            &self,
            _streams: &[(String, String)],
            _count: Option<i64>,
            _timeout: Duration,
        ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
            Ok(None)
        }

        async fn xgroup_create(
            &self,
            _key: &str,
            _group: &str,
            _id: &str,
            _options: XGroupCreateOptions,
        ) -> Result<(), CacheError> {
            Ok(())
        }

        async fn xgroup_destroy(&self, _key: &str, _group: &str) -> Result<bool, CacheError> {
            Ok(true)
        }

        async fn xgroup_setid(
            &self,
            _key: &str,
            _group: &str,
            _id: &str,
            _entries_read: Option<i64>,
        ) -> Result<(), CacheError> {
            Ok(())
        }

        async fn xgroup_createconsumer(
            &self,
            _key: &str,
            _group: &str,
            _consumer: &str,
        ) -> Result<bool, CacheError> {
            Ok(true)
        }

        async fn xgroup_delconsumer(
            &self,
            _key: &str,
            _group: &str,
            _consumer: &str,
        ) -> Result<i64, CacheError> {
            Ok(5)
        }

        async fn xinfo_groups(&self, _key: &str) -> Result<Vec<ConsumerGroupInfo>, CacheError> {
            Ok(vec![])
        }

        async fn xinfo_consumers(
            &self,
            _key: &str,
            _group: &str,
        ) -> Result<Vec<ConsumerInfo>, CacheError> {
            Ok(vec![])
        }

        async fn xreadgroup(
            &self,
            _group: &str,
            _consumer: &str,
            _streams: &[(String, String)],
            _options: XReadGroupOptions,
        ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
            Ok(Some(vec![]))
        }

        async fn xreadgroup_blocking(
            &self,
            _group: &str,
            _consumer: &str,
            _streams: &[(String, String)],
            _count: Option<i64>,
            _no_ack: bool,
            _timeout: Duration,
        ) -> Result<Option<Vec<StreamReadResult>>, CacheError> {
            Ok(None)
        }

        async fn xack(
            &self,
            _key: &str,
            _group: &str,
            ids: &[String],
        ) -> Result<i64, CacheError> {
            Ok(ids.len() as i64)
        }

        async fn xpending_summary(
            &self,
            _key: &str,
            _group: &str,
        ) -> Result<PendingSummary, CacheError> {
            Ok(PendingSummary {
                count: 0,
                min_id: None,
                max_id: None,
                consumers: HashMap::new(),
            })
        }

        async fn xpending(
            &self,
            _key: &str,
            _group: &str,
            _options: XPendingOptions,
        ) -> Result<Vec<PendingEntry>, CacheError> {
            Ok(vec![])
        }

        async fn xclaim(
            &self,
            _key: &str,
            _group: &str,
            _consumer: &str,
            _ids: &[String],
            _options: XClaimOptions,
        ) -> Result<ClaimResult, CacheError> {
            Ok(ClaimResult { entries: vec![] })
        }

        async fn xautoclaim(
            &self,
            _key: &str,
            _group: &str,
            _consumer: &str,
            _min_idle_time_ms: i64,
            _start: &str,
            _options: XAutoClaimOptions,
        ) -> Result<AutoClaimResult, CacheError> {
            Ok(AutoClaimResult {
                next_id: "0-0".to_string(),
                entries: vec![],
                deleted_ids: vec![],
            })
        }

        async fn xsetid(
            &self,
            _key: &str,
            _last_id: &str,
            _entries_added: Option<i64>,
            _max_deleted_id: Option<&str>,
        ) -> Result<(), CacheError> {
            Ok(())
        }
    }

    fn create_test_service() -> StreamService {
        StreamService::new_with_repository(Arc::new(MockStreamRepository))
    }

    #[tokio::test]
    async fn test_xadd_validation() {
        let service = create_test_service();

        // Empty fields should fail
        let result = service.xadd("stream", HashMap::new(), XAddOptions::default()).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));

        // Valid fields should succeed
        let mut fields = HashMap::new();
        fields.insert("key".to_string(), "value".to_string());
        let result = service.xadd("stream", fields, XAddOptions::default()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_xread_validation() {
        let service = create_test_service();

        // Empty streams should fail
        let result = service.xread(vec![], XReadOptions::default()).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));

        // Valid streams should succeed
        let result = service
            .xread(
                vec![("stream".to_string(), "0".to_string())],
                XReadOptions::default(),
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_xreadgroup_validation() {
        let service = create_test_service();

        // Empty streams should fail
        let result = service
            .xreadgroup("group", "consumer", vec![], XReadGroupOptions::default())
            .await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));

        // Empty group/consumer should fail
        let result = service
            .xreadgroup(
                "",
                "consumer",
                vec![("stream".to_string(), ">".to_string())],
                XReadGroupOptions::default(),
            )
            .await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_timeout_enforcement() {
        let service = create_test_service();

        // Test that timeout is enforced
        let timeout = service.enforce_timeout(Duration::from_secs(60));
        assert_eq!(timeout, Duration::from_secs(30));

        // Test that smaller timeout is not modified
        let timeout = service.enforce_timeout(Duration::from_secs(10));
        assert_eq!(timeout, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_block_ms_enforcement() {
        let service = create_test_service();

        // Test that block_ms is enforced
        let enforced = service.enforce_block_ms(Some(60000));
        assert_eq!(enforced, Some(30000));

        // Test that smaller block_ms is not modified
        let enforced = service.enforce_block_ms(Some(5000));
        assert_eq!(enforced, Some(5000));

        // Test that None remains None
        let enforced = service.enforce_block_ms(None);
        assert!(enforced.is_none());
    }

    #[tokio::test]
    async fn test_xgroup_create_validation() {
        let service = create_test_service();

        // Empty group name should fail
        let result = service
            .xgroup_create("stream", "", "0", XGroupCreateOptions::default())
            .await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_xack_validation() {
        let service = create_test_service();

        // Empty IDs should fail
        let result = service.xack("stream", "group", vec![]).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_xclaim_validation() {
        let service = create_test_service();

        // Empty IDs should fail
        let result = service
            .xclaim("stream", "group", "consumer", vec![], XClaimOptions::default())
            .await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));

        // Empty consumer should fail
        let result = service
            .xclaim(
                "stream",
                "group",
                "",
                vec!["id".to_string()],
                XClaimOptions::default(),
            )
            .await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_with_max_blocking_timeout_applies() {
        let service =
            create_test_service().with_max_blocking_timeout(Duration::from_secs(5));
        let enforced = service.enforce_timeout(Duration::from_secs(10));
        assert_eq!(enforced, Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_basic_stream_operations() {
        let service = create_test_service();

        let length = service.xlen("stream").await.unwrap();
        assert_eq!(length, 10);

        let range = service.xrange("stream", "-", "+", None).await.unwrap();
        assert!(range.is_empty());

        let revrange = service.xrevrange("stream", "+", "-", None).await.unwrap();
        assert!(revrange.is_empty());

        let deleted = service
            .xdel("stream", vec!["1-0".to_string()])
            .await
            .unwrap();
        assert_eq!(deleted, 1);

        let trimmed = service
            .xtrim("stream", XTrimStrategy::MaxLen {
                count: 10,
                approximate: true,
            })
            .await
            .unwrap();
        assert_eq!(trimmed, 5);

        let info = service.xinfo_stream("stream", false).await.unwrap();
        assert_eq!(info.groups, 2);
    }

    #[tokio::test]
    async fn test_xdel_validation() {
        let service = create_test_service();
        let result = service.xdel("stream", vec![]).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_xread_blocking_validation_and_success() {
        let service = create_test_service();

        let result = service.xread_blocking(vec![], None, 1).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));

        let result = service
            .xread_blocking(vec![("stream".to_string(), "0".to_string())], None, 1)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_group_management_and_info() {
        let service = create_test_service();

        let destroyed = service.xgroup_destroy("stream", "group").await.unwrap();
        assert!(destroyed);

        service
            .xgroup_setid("stream", "group", "0", Some(1))
            .await
            .unwrap();

        let created = service
            .xgroup_createconsumer("stream", "group", "consumer")
            .await
            .unwrap();
        assert!(created);

        let pending = service
            .xgroup_delconsumer("stream", "group", "consumer")
            .await
            .unwrap();
        assert_eq!(pending, 5);

        let groups = service.xinfo_groups("stream").await.unwrap();
        assert!(groups.is_empty());

        let consumers = service.xinfo_consumers("stream", "group").await.unwrap();
        assert!(consumers.is_empty());
    }

    #[tokio::test]
    async fn test_xgroup_createconsumer_validation() {
        let service = create_test_service();
        let result = service.xgroup_createconsumer("stream", "group", "").await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_xreadgroup_success_and_validation() {
        let service = create_test_service();

        let result = service
            .xreadgroup(
                "group",
                "consumer",
                vec![("stream".to_string(), ">".to_string())],
                XReadGroupOptions::default(),
            )
            .await;
        assert!(result.is_ok());

        let result = service
            .xreadgroup(
                "group",
                "",
                vec![("stream".to_string(), ">".to_string())],
                XReadGroupOptions::default(),
            )
            .await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_xreadgroup_blocking_validation_and_success() {
        let service = create_test_service();

        let result = service
            .xreadgroup_blocking("group", "consumer", vec![], None, false, 1)
            .await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));

        let result = service
            .xreadgroup_blocking("", "consumer", vec![("stream".to_string(), ">".to_string())], None, false, 1)
            .await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));

        let result = service
            .xreadgroup_blocking(
                "group",
                "consumer",
                vec![("stream".to_string(), ">".to_string())],
                None,
                false,
                1,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pending_and_claim_operations() {
        let service = create_test_service();

        let summary = service.xpending_summary("stream", "group").await.unwrap();
        assert_eq!(summary.count, 0);

        let entries = service
            .xpending(
                "stream",
                "group",
                XPendingOptions {
                    start: None,
                    end: None,
                    count: None,
                    consumer: None,
                    idle_ms: None,
                },
            )
            .await
            .unwrap();
        assert!(entries.is_empty());

        let claim = service
            .xclaim(
                "stream",
                "group",
                "consumer",
                vec!["1-0".to_string()],
                XClaimOptions::default(),
            )
            .await
            .unwrap();
        assert!(claim.entries.is_empty());

        let autoclaim = service
            .xautoclaim(
                "stream",
                "group",
                "consumer",
                0,
                "0-0",
                XAutoClaimOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(autoclaim.next_id, "0-0");
    }

    #[tokio::test]
    async fn test_xautoclaim_validation() {
        let service = create_test_service();
        let result = service
            .xautoclaim(
                "stream",
                "group",
                "",
                0,
                "0-0",
                XAutoClaimOptions::default(),
            )
            .await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_xsetid_success() {
        let service = create_test_service();
        let result = service
            .xsetid("stream", "1-0", Some(1), Some("0-0"))
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_sse_block_ms() {
        assert_eq!(StreamService::default_sse_block_ms(), 5000);
    }
}
