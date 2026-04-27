#![allow(dead_code, clippy::needless_range_loop)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use testcontainers::ContainerAsync;
use testcontainers::GenericImage;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::redis::{REDIS_PORT, Redis};

use crate::application::services::{
    AdminService, BitMapService, BloomService, ClusterService, FunctionService, GeoService,
    HashService, JsonService, KeyService, ListService, ProbabilisticService, PubSubService,
    SearchService, SetService, SortedSetService, StreamService, StringService, TimeSeriesService,
};
use crate::domain::entities::{
    AclDryrunResult,
    AclLogEntry,
    // Search entities
    AggregateOptions,
    AggregateResult,
    AliasResult,
    AppendResult,
    AutoClaimResult,
    BgRewriteAofResult,
    BgSaveResult,
    // Bloom entities
    BloomAddResult,
    BloomCardResult,
    BloomExistsResult,
    BloomInfo,
    BloomInsertOptions,
    BloomInsertResult,
    BloomLoadChunkResult,
    BloomReserveOptions,
    BloomReserveResult,
    BloomScanDumpResult,
    ClaimResult,
    ClientInfo,
    ClientKillOptions,
    ClientPauseOptions,
    // Probabilistic entities
    CmsIncrByResult,
    CmsInfo,
    CmsInitResult,
    CmsMergeResult,
    CmsQueryResult,
    ConsumerGroupInfo,
    ConsumerInfo,
    CopyKeyOptions,
    CopyOptions,
    CopyResult,
    CuckooAddResult,
    CuckooCountResult,
    CuckooDelResult,
    CuckooExistsResult,
    CuckooInfo,
    CuckooInsertOptions,
    CuckooInsertResult,
    CuckooLoadChunkResult,
    CuckooReserveOptions,
    CuckooReserveResult,
    CuckooScanDumpResult,
    DeleteResult,
    DictDumpResult,
    DictResult,
    DumpResult,
    ExistsResult,
    ExpireOptions,
    ExpireResult,
    ExplainResult,
    FlushOptions,
    FlushResult,
    GetExOptions,
    IndexAlterResult,
    IndexCreateOptions,
    IndexCreateResult,
    IndexDropResult,
    IndexInfo,
    JsonArrAppendResult,
    JsonArrIndexResult,
    JsonArrInsertResult,
    JsonArrLenResult,
    JsonArrPopResult,
    JsonArrTrimResult,
    JsonClearResult,
    JsonDebugMemoryResult,
    JsonDelResult,
    JsonGetResult,
    JsonMGetItem,
    JsonMGetResult,
    JsonMSetItem,
    JsonNumResult,
    JsonObjKeysResult,
    JsonObjLenResult,
    JsonRespResult,
    JsonSetOptions,
    JsonSetResult,
    JsonStrAppendResult,
    JsonStrLenResult,
    JsonToggleResult,
    JsonTypeResult,
    // Admin entity for COMMAND GETKEYSANDFLAGS (10.7)
    KeyAndFlags,
    KeyInfo,
    LatencyEvent,
    MGetResult,
    MemoryStats,
    MemoryUsage,
    MoveKeyOptions,
    PendingEntry,
    PendingSummary,
    PersistResult,
    PfAddResult,
    PfCountResult,
    PfMergeResult,
    ProfileResult,
    ProfileType,
    RandomKeyResult,
    RangeResult,
    RenameResult,
    ScanResult,
    SearchFieldSchema,
    SearchOptions,
    SearchResult,
    ServerInfo,
    ServerTime,
    SetOptions,
    SetRangeResult,
    SetResult,
    SlowlogEntry,
    SpellcheckOptions,
    SpellcheckResult,
    StreamEntry,
    StreamInfo,
    StreamReadResult,
    StringValue,
    SugAddOptions,
    SugAddResult,
    SugDelResult,
    SugGetOptions,
    SugLenResult,
    Suggestion,
    SynonymGroup,
    SynonymUpdateResult,
    TDigestAckResult,
    TDigestInfo,
    TDigestRanksResult,
    TDigestScalarResult,
    TDigestValuesResult,
    TopKAddResult,
    TopKCountResult,
    TopKIncrByResult,
    TopKInfo,
    TopKItem,
    TopKListResult,
    TopKQueryResult,
    TopKReserveResult,
    TouchResult,
    XAddOptions,
    XAutoClaimOptions,
    XClaimOptions,
    XGroupCreateOptions,
    XPendingOptions,
    XReadGroupOptions,
    XReadOptions,
    XTrimOptions,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    AdminRepository, BitMapRepository, BitOperation, BitfieldCommand, BitfieldResult,
    BlockingPopResult, BloomRepository, ClusterInfo, ClusterNode, ClusterRepository,
    ClusterSlotRange, DelExCondition, ExpireCondition, FunctionFlushMode, FunctionRepository,
    FunctionRestorePolicy, HSetExCondition, HashExpiration, HashRepository, InsertPosition,
    JsonRepository, KeyRepository, LMPopResult, LPosOptions, LcsMatch, LcsMatchResult, LcsOptions,
    LcsResult, LexRange, ListDirection, ListRepository, MSetExExistence, MSetExOptions,
    NumSubResult, ProbabilisticRepository, PubSubRepository, PublishResult, ScoreRange,
    ScoredMember, SearchRepository, SetRepository, SetScanResult, SortOptions, SortedSetRepository,
    StreamRepository, StringRepository, TimeSeriesAddOptions, TimeSeriesCreateOptions,
    TimeSeriesMGetResult, TimeSeriesRangeOptions, TimeSeriesRangeResult, TimeSeriesRepository,
    TimeSeriesSample, TsAggregation, ZAddOptions, ZAddResult, ZPopDirection, ZPopResult,
    ZRangeOptions, ZScanResult, ZSetAlgebraOptions,
};
use crate::infrastructure::config::Settings;
use crate::infrastructure::redis::capabilities::RedisCapabilities;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::pubsub_manager::PubSubManager;
use crate::shared::app_state::AppState;
use serde_json::Value;

/// Start a Redis container for integration testing.
///
/// Returns `None` when Docker is unavailable **unless** `REQUIRE_DOCKER=1` is
/// set, in which case the test panics so CI pipelines don't silently skip
/// Redis integration tests.
pub async fn start_redis_container() -> Option<(ContainerAsync<Redis>, String)> {
    let container = match Redis::default().start().await {
        Ok(container) => container,
        Err(err) => {
            return handle_docker_unavailable("Redis", &err);
        }
    };
    let host = match container.get_host().await {
        Ok(host) => host,
        Err(err) => {
            return handle_docker_unavailable("Redis", &err);
        }
    };
    let port = match container.get_host_port_ipv4(REDIS_PORT).await {
        Ok(port) => port,
        Err(err) => {
            return handle_docker_unavailable("Redis", &err);
        }
    };
    let url = format!("redis://{host}:{port}");

    // Wait for Redis to actually accept commands (port mapping != ready).
    // Without this, blocking command tests can hit a timeout during startup.
    for _ in 0..20 {
        if let Ok(client) = redis::Client::open(url.as_str())
            && let Ok(mut conn) = client.get_multiplexed_async_connection().await
            && redis::cmd("PING")
                .query_async::<String>(&mut conn)
                .await
                .is_ok()
        {
            return Some((container, url));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Some((container, url))
}

/// Start a generic Redis image for integration testing.
///
/// Same Docker-availability semantics as [`start_redis_container`].
pub async fn start_generic_redis_image(
    image: GenericImage,
    port: u16,
    ready_delay: Duration,
    label: &str,
) -> Option<(ContainerAsync<GenericImage>, String)> {
    let container = match image.start().await {
        Ok(container) => container,
        Err(err) => {
            return handle_docker_unavailable(label, &err);
        }
    };
    tokio::time::sleep(ready_delay).await;
    let host = match container.get_host().await {
        Ok(host) => host,
        Err(err) => {
            return handle_docker_unavailable(label, &err);
        }
    };
    let mapped_port = match container.get_host_port_ipv4(port).await {
        Ok(port) => port,
        Err(err) => {
            return handle_docker_unavailable(label, &err);
        }
    };
    Some((container, format!("redis://{host}:{mapped_port}")))
}

/// Central handler for Docker unavailability in test helpers.
///
/// - When `REQUIRE_DOCKER=1`: panics so the test fails visibly (use in CI).
/// - Otherwise: prints a warning to stderr and returns `None` to skip the test.
fn handle_docker_unavailable<T>(label: &str, err: &dyn std::fmt::Display) -> Option<T> {
    if std::env::var("REQUIRE_DOCKER").as_deref() == Ok("1") {
        panic!("Docker-dependent {label} test failed and REQUIRE_DOCKER=1 is set: {err}");
    }
    // Write directly to the stderr fd via std::io::Write, bypassing the test
    // harness's macro-level capture so the warning is visible even for passing
    // tests (eprintln! is swallowed by default).
    use std::io::Write;
    let _ = writeln!(
        std::io::stderr(),
        "WARNING: Skipping Docker-dependent {label} test (set REQUIRE_DOCKER=1 to fail instead): {err}"
    );
    None
}

#[derive(Default)]
pub struct MockStringRepository {
    store: Mutex<HashMap<String, String>>,
}

impl MockStringRepository {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, key: &str, value: &str) {
        self.store
            .lock()
            .expect("store lock")
            .insert(key.to_string(), value.to_string());
    }

    /// Deterministic stand-in for the XXH3 digest used by Redis 8.4 DIGEST/DELEX.
    /// Matches across `digest()` and the IFDEQ/IFDNE arms of `delex()`.
    pub fn mock_digest(value: &str) -> String {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

#[async_trait]
impl StringRepository for MockStringRepository {
    async fn get(&self, key: &str) -> Result<Option<StringValue>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store
            .get(key)
            .map(|value| StringValue::new(key.to_string(), value.clone())))
    }

    async fn set(
        &self,
        key: &str,
        value: &str,
        options: SetOptions,
    ) -> Result<SetResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let exists = store.contains_key(key);
        if options.nx && exists {
            return Ok(SetResult {
                key: key.to_string(),
                success: false,
                previous_value: None,
            });
        }
        if options.xx && !exists {
            return Ok(SetResult {
                key: key.to_string(),
                success: false,
                previous_value: None,
            });
        }

        let previous = if options.get {
            store.get(key).cloned()
        } else {
            None
        };
        store.insert(key.to_string(), value.to_string());

        Ok(SetResult {
            key: key.to_string(),
            success: true,
            previous_value: previous,
        })
    }

    async fn set_nx(
        &self,
        key: &str,
        value: &str,
        _ttl: Option<Duration>,
    ) -> Result<bool, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if store.contains_key(key) {
            return Ok(false);
        }
        store.insert(key.to_string(), value.to_string());
        Ok(true)
    }

    async fn set_ex(&self, key: &str, value: &str, _ttl: Duration) -> Result<(), CacheError> {
        let mut store = self.store.lock().expect("store lock");
        store.insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn mget(&self, keys: &[String]) -> Result<MGetResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        let mut found = HashMap::new();
        let mut missing = Vec::new();

        for key in keys {
            match store.get(key) {
                Some(value) => {
                    found.insert(key.clone(), value.clone());
                }
                None => missing.push(key.clone()),
            }
        }

        Ok(MGetResult { found, missing })
    }

    async fn mset(&self, pairs: &[(String, String)]) -> Result<(), CacheError> {
        let mut store = self.store.lock().expect("store lock");
        for (key, value) in pairs {
            store.insert(key.clone(), value.clone());
        }
        Ok(())
    }

    async fn mset_nx(&self, pairs: &[(String, String)]) -> Result<bool, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if pairs.iter().any(|(key, _)| store.contains_key(key)) {
            return Ok(false);
        }
        for (key, value) in pairs {
            store.insert(key.clone(), value.clone());
        }
        Ok(true)
    }

    async fn incr(&self, key: &str) -> Result<i64, CacheError> {
        self.incr_by(key, 1).await
    }

    async fn incr_by(&self, key: &str, delta: i64) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let current = store
            .get(key)
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(0);
        let next = current + delta;
        store.insert(key.to_string(), next.to_string());
        Ok(next)
    }

    async fn incr_by_float(&self, key: &str, delta: f64) -> Result<f64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let current = store
            .get(key)
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.0);
        let next = current + delta;
        store.insert(key.to_string(), next.to_string());
        Ok(next)
    }

    async fn decr(&self, key: &str) -> Result<i64, CacheError> {
        self.decr_by(key, 1).await
    }

    async fn decr_by(&self, key: &str, delta: i64) -> Result<i64, CacheError> {
        self.incr_by(key, -delta).await
    }

    async fn append(&self, key: &str, value: &str) -> Result<AppendResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        entry.push_str(value);
        Ok(AppendResult {
            key: key.to_string(),
            new_length: entry.len() as i64,
        })
    }

    async fn str_len(&self, key: &str) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).map(|value| value.len() as i64).unwrap_or(0))
    }

    async fn get_range(&self, key: &str, start: i64, end: i64) -> Result<RangeResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        let value = store.get(key).cloned().unwrap_or_default();
        let len = value.len() as i64;
        let start = if start < 0 {
            (len + start).max(0)
        } else {
            start
        };
        let end = if end < 0 { len + end } else { end };
        let end = end.min(len.saturating_sub(1)).max(start - 1);
        let start_usize = start as usize;
        let end_usize = (end + 1) as usize;
        let slice = if start <= end {
            value.get(start_usize..end_usize).unwrap_or("").to_string()
        } else {
            String::new()
        };

        Ok(RangeResult {
            key: key.to_string(),
            value: slice,
            start,
            end,
        })
    }

    async fn set_range(
        &self,
        key: &str,
        offset: i64,
        value: &str,
    ) -> Result<SetRangeResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let mut current = store.get(key).cloned().unwrap_or_default();
        let offset_usize = offset.max(0) as usize;
        if offset_usize > current.len() {
            current.push_str(&"\0".repeat(offset_usize - current.len()));
        }
        if offset_usize + value.len() > current.len() {
            current.replace_range(offset_usize.., value);
        } else {
            current.replace_range(offset_usize..offset_usize + value.len(), value);
        }
        let new_length = current.len() as i64;
        store.insert(key.to_string(), current);
        Ok(SetRangeResult {
            key: key.to_string(),
            new_length,
        })
    }

    async fn get_ex(
        &self,
        key: &str,
        _options: GetExOptions,
    ) -> Result<Option<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).cloned())
    }

    async fn get_del(&self, key: &str) -> Result<Option<String>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        Ok(store.remove(key))
    }

    async fn lcs(
        &self,
        key1: &str,
        key2: &str,
        options: LcsOptions,
    ) -> Result<LcsResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        let s1 = store.get(key1).cloned().unwrap_or_default();
        let s2 = store.get(key2).cloned().unwrap_or_default();

        // Compute actual LCS using dynamic programming
        let b1 = s1.as_bytes();
        let b2 = s2.as_bytes();
        let m = b1.len();
        let n = b2.len();
        let mut dp = vec![vec![0usize; n + 1]; m + 1];

        for i in 1..=m {
            for j in 1..=n {
                if b1[i - 1] == b2[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        let lcs_len = dp[m][n] as i64;

        if options.len {
            return Ok(LcsResult::Length(lcs_len));
        }

        // Backtrack to find the LCS string
        let mut lcs_chars = Vec::new();
        let mut i = m;
        let mut j = n;
        while i > 0 && j > 0 {
            if b1[i - 1] == b2[j - 1] {
                lcs_chars.push(b1[i - 1]);
                i -= 1;
                j -= 1;
            } else if dp[i - 1][j] > dp[i][j - 1] {
                i -= 1;
            } else {
                j -= 1;
            }
        }
        lcs_chars.reverse();
        let lcs_string = String::from_utf8_lossy(&lcs_chars).to_string();

        if options.idx {
            // Find match positions by scanning the LCS against both strings
            let mut matches = Vec::new();
            let mut li = 0usize; // position in lcs
            let mut i = 0usize; // position in s1
            let mut j = 0usize; // position in s2

            while li < lcs_chars.len() {
                // Find start of next match run
                while i < m && b1[i] != lcs_chars[li] {
                    i += 1;
                }
                while j < n && b2[j] != lcs_chars[li] {
                    j += 1;
                }

                let start1 = i;
                let start2 = j;
                let mut run_len = 0;

                // Extend consecutive match
                while li < lcs_chars.len()
                    && i < m
                    && j < n
                    && b1[i] == lcs_chars[li]
                    && b2[j] == lcs_chars[li]
                {
                    i += 1;
                    j += 1;
                    li += 1;
                    run_len += 1;
                }

                let min_match = options.min_match_len.unwrap_or(0) as usize;
                if run_len >= min_match.max(1) {
                    let match_len = if options.with_match_len {
                        Some(run_len as i64)
                    } else {
                        None
                    };
                    matches.push(LcsMatch {
                        key1_range: (start1 as i64, (start1 + run_len - 1) as i64),
                        key2_range: (start2 as i64, (start2 + run_len - 1) as i64),
                        match_len,
                    });
                }
            }

            Ok(LcsResult::Matches(LcsMatchResult {
                matches,
                len: lcs_len,
            }))
        } else {
            Ok(LcsResult::String(lcs_string))
        }
    }

    async fn msetex(
        &self,
        pairs: &[(String, String)],
        options: MSetExOptions,
    ) -> Result<bool, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        match options.existence {
            Some(MSetExExistence::Nx) if pairs.iter().any(|(k, _)| store.contains_key(k)) => {
                return Ok(false);
            }
            Some(MSetExExistence::Xx) if !pairs.iter().all(|(k, _)| store.contains_key(k)) => {
                return Ok(false);
            }
            _ => {}
        }
        for (key, value) in pairs {
            store.insert(key.clone(), value.clone());
        }
        Ok(true)
    }

    async fn delex(
        &self,
        key: &str,
        condition: Option<DelExCondition>,
    ) -> Result<bool, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let current = match store.get(key) {
            Some(v) => v.clone(),
            None => return Ok(false),
        };
        let allowed = match condition {
            None => true,
            Some(DelExCondition::IfEq(v)) => current == v,
            Some(DelExCondition::IfNe(v)) => current != v,
            Some(DelExCondition::IfDeq(d)) => MockStringRepository::mock_digest(&current) == d,
            Some(DelExCondition::IfDne(d)) => MockStringRepository::mock_digest(&current) != d,
        };
        if allowed {
            store.remove(key);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn digest(&self, key: &str) -> Result<Option<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).map(|v| MockStringRepository::mock_digest(v)))
    }
}

#[derive(Default)]
pub struct MockAdminRepository;

#[async_trait]
impl AdminRepository for MockAdminRepository {
    async fn get_server_info(&self) -> Result<ServerInfo, CacheError> {
        Ok(ServerInfo::default())
    }

    async fn get_server_time(&self) -> Result<ServerTime, CacheError> {
        Ok(ServerTime {
            timestamp: 1,
            microseconds: 2,
        })
    }

    async fn get_db_size(&self) -> Result<i64, CacheError> {
        Ok(42)
    }

    async fn get_last_save(&self) -> Result<i64, CacheError> {
        Ok(123)
    }

    async fn debug_object(&self, key: &str) -> Result<String, CacheError> {
        Ok(format!("debug:{key}"))
    }

    async fn shutdown(&self, _save: bool, _now: bool) -> Result<(), CacheError> {
        Ok(())
    }

    async fn get_memory_stats(&self) -> Result<MemoryStats, CacheError> {
        Ok(MemoryStats::default())
    }

    async fn get_memory_usage(&self, key: &str, _samples: u32) -> Result<MemoryUsage, CacheError> {
        Ok(MemoryUsage {
            key: key.to_string(),
            bytes: Some(64),
        })
    }

    async fn memory_doctor(&self) -> Result<String, CacheError> {
        Ok("OK".to_string())
    }

    async fn memory_purge(&self) -> Result<(), CacheError> {
        Ok(())
    }

    async fn flush_db(&self, options: FlushOptions) -> Result<FlushResult, CacheError> {
        Ok(FlushResult {
            success: true,
            mode: if options.async_mode { "ASYNC" } else { "SYNC" }.to_string(),
        })
    }

    async fn flush_all(&self, options: FlushOptions) -> Result<FlushResult, CacheError> {
        Ok(FlushResult {
            success: true,
            mode: if options.async_mode { "ASYNC" } else { "SYNC" }.to_string(),
        })
    }

    async fn copy_key(&self, _options: CopyKeyOptions) -> Result<bool, CacheError> {
        Ok(true)
    }

    async fn move_key(&self, _options: MoveKeyOptions) -> Result<bool, CacheError> {
        Ok(true)
    }

    async fn swap_db(&self, _db1: u8, _db2: u8) -> Result<(), CacheError> {
        Ok(())
    }

    async fn config_get(&self, pattern: &str) -> Result<HashMap<String, String>, CacheError> {
        let mut data = HashMap::new();
        data.insert(pattern.to_string(), "value".to_string());
        Ok(data)
    }

    async fn config_set(&self, _parameter: &str, _value: &str) -> Result<(), CacheError> {
        Ok(())
    }

    async fn config_rewrite(&self) -> Result<(), CacheError> {
        Ok(())
    }

    async fn config_resetstat(&self) -> Result<(), CacheError> {
        Ok(())
    }

    async fn save(&self) -> Result<(), CacheError> {
        Ok(())
    }

    async fn bgsave(&self) -> Result<BgSaveResult, CacheError> {
        Ok(BgSaveResult {
            started: true,
            message: "OK".to_string(),
        })
    }

    async fn bgrewriteaof(&self) -> Result<BgRewriteAofResult, CacheError> {
        Ok(BgRewriteAofResult {
            started: true,
            message: "OK".to_string(),
        })
    }

    async fn client_list(&self) -> Result<Vec<ClientInfo>, CacheError> {
        let mut info = ClientInfo::default();
        info.id = 1;
        Ok(vec![info])
    }

    async fn client_kill(&self, _options: ClientKillOptions) -> Result<i64, CacheError> {
        Ok(1)
    }

    async fn client_pause(&self, _options: ClientPauseOptions) -> Result<(), CacheError> {
        Ok(())
    }

    async fn client_unpause(&self) -> Result<(), CacheError> {
        Ok(())
    }

    async fn client_setname(&self, _name: &str) -> Result<(), CacheError> {
        Ok(())
    }

    async fn client_getname(&self) -> Result<Option<String>, CacheError> {
        Ok(Some("client".to_string()))
    }

    async fn client_id(&self) -> Result<i64, CacheError> {
        Ok(99)
    }

    async fn client_info(&self) -> Result<ClientInfo, CacheError> {
        Ok(ClientInfo::default())
    }

    async fn slowlog_get(&self, _count: i64) -> Result<Vec<SlowlogEntry>, CacheError> {
        Ok(vec![])
    }

    async fn slowlog_len(&self) -> Result<i64, CacheError> {
        Ok(0)
    }

    async fn slowlog_reset(&self) -> Result<(), CacheError> {
        Ok(())
    }

    async fn latency_latest(&self) -> Result<Vec<LatencyEvent>, CacheError> {
        Ok(vec![LatencyEvent {
            event: "command".to_string(),
            timestamp: 1,
            latency_ms: 2,
        }])
    }

    async fn latency_history(&self, event: &str) -> Result<Vec<LatencyEvent>, CacheError> {
        Ok(vec![LatencyEvent {
            event: event.to_string(),
            timestamp: 1,
            latency_ms: 2,
        }])
    }

    async fn latency_doctor(&self) -> Result<String, CacheError> {
        Ok("OK".to_string())
    }

    async fn latency_reset(&self, _events: &[String]) -> Result<(), CacheError> {
        Ok(())
    }

    async fn latency_graph(&self, event: &str) -> Result<String, CacheError> {
        Ok(format!("graph:{event}"))
    }

    async fn acl_list(&self) -> Result<Vec<String>, CacheError> {
        Ok(vec!["user default on".to_string()])
    }

    async fn acl_users(&self) -> Result<Vec<String>, CacheError> {
        Ok(vec!["default".to_string()])
    }

    async fn acl_whoami(&self) -> Result<String, CacheError> {
        Ok("default".to_string())
    }

    async fn acl_cat(&self, category: Option<&str>) -> Result<Vec<String>, CacheError> {
        Ok(vec![category.unwrap_or("all").to_string()])
    }

    async fn acl_genpass(&self, bits: u32) -> Result<String, CacheError> {
        Ok(format!("pass-{bits}"))
    }

    async fn acl_log(
        &self,
        _count: Option<i64>,
        _reset: bool,
    ) -> Result<Vec<AclLogEntry>, CacheError> {
        Ok(vec![AclLogEntry::default()])
    }

    async fn acl_dryrun(
        &self,
        _username: &str,
        _command: &[String],
    ) -> Result<AclDryrunResult, CacheError> {
        Ok(AclDryrunResult {
            allowed: true,
            reason: None,
        })
    }

    async fn acl_setuser(&self, _username: &str, _rules: &[String]) -> Result<(), CacheError> {
        Ok(())
    }

    async fn acl_deluser(&self, usernames: &[String]) -> Result<i64, CacheError> {
        Ok(usernames.len() as i64)
    }

    async fn acl_load(&self) -> Result<(), CacheError> {
        Ok(())
    }

    async fn acl_save(&self) -> Result<(), CacheError> {
        Ok(())
    }

    async fn command_list(&self, _filter: Option<&str>) -> Result<Vec<String>, CacheError> {
        Ok(vec![
            "get".to_string(),
            "set".to_string(),
            "del".to_string(),
        ])
    }

    async fn command_count(&self) -> Result<i64, CacheError> {
        Ok(242)
    }

    async fn command_docs(&self, _commands: &[String]) -> Result<serde_json::Value, CacheError> {
        Ok(serde_json::json!({"get": {"summary": "Get the value of a key"}}))
    }

    async fn command_info(&self, _commands: &[String]) -> Result<serde_json::Value, CacheError> {
        Ok(serde_json::json!([[
            "get",
            2,
            ["readonly", "fast"],
            1,
            1,
            1
        ]]))
    }

    async fn command_getkeys(&self, _command: &[String]) -> Result<Vec<String>, CacheError> {
        Ok(vec!["mykey".to_string()])
    }

    async fn command_getkeysandflags(
        &self,
        _command: &[String],
    ) -> Result<Vec<KeyAndFlags>, CacheError> {
        Ok(vec![KeyAndFlags {
            key: "mykey".to_string(),
            flags: vec!["RW".to_string(), "access".to_string(), "update".to_string()],
        }])
    }

    async fn latency_histogram(
        &self,
        _commands: &[String],
    ) -> Result<serde_json::Value, CacheError> {
        Ok(serde_json::json!({
            "set": { "calls": 10, "histogram_usec": { "1": 5, "2": 5 } }
        }))
    }

    async fn memory_malloc_stats(&self) -> Result<String, CacheError> {
        Ok("mock allocator stats".to_string())
    }
}

#[derive(Default)]
pub struct MockFunctionRepository;

#[async_trait]
impl FunctionRepository for MockFunctionRepository {
    async fn function_load(&self, code: &str, _replace: bool) -> Result<String, CacheError> {
        let name = code
            .lines()
            .find_map(|line| line.strip_prefix("#!lua name="))
            .unwrap_or("library");
        Ok(name.to_string())
    }

    async fn function_delete(&self, _name: &str) -> Result<(), CacheError> {
        Ok(())
    }

    async fn function_flush(&self, _mode: Option<FunctionFlushMode>) -> Result<(), CacheError> {
        Ok(())
    }

    async fn function_dump(&self) -> Result<Vec<u8>, CacheError> {
        Ok(vec![1, 2, 3])
    }

    async fn function_restore(
        &self,
        _payload: &[u8],
        _policy: Option<FunctionRestorePolicy>,
    ) -> Result<(), CacheError> {
        Ok(())
    }

    async fn function_list(&self, with_code: bool) -> Result<serde_json::Value, CacheError> {
        Ok(serde_json::json!([{
            "library_name": "lib",
            "engine": "LUA",
            "functions": [{"name": "echo"}],
            "library_code": if with_code { "code" } else { "" }
        }]))
    }

    async fn function_stats(&self) -> Result<serde_json::Value, CacheError> {
        Ok(serde_json::json!({"running_script": null}))
    }

    async fn function_kill(&self) -> Result<(), CacheError> {
        Ok(())
    }

    async fn fcall(
        &self,
        _name: &str,
        _keys: &[String],
        args: &[serde_json::Value],
        _readonly: bool,
    ) -> Result<serde_json::Value, CacheError> {
        Ok(args.first().cloned().unwrap_or(serde_json::Value::Null))
    }
}

#[derive(Default)]
pub struct MockTimeSeriesRepository {
    data: Mutex<HashMap<String, Vec<TimeSeriesSample>>>,
    labels: Mutex<HashMap<String, HashMap<String, String>>>,
}

impl MockTimeSeriesRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TimeSeriesRepository for MockTimeSeriesRepository {
    async fn ts_create(
        &self,
        key: &str,
        options: TimeSeriesCreateOptions,
    ) -> Result<(), CacheError> {
        self.data
            .lock()
            .expect("lock")
            .entry(key.to_string())
            .or_default();
        self.labels
            .lock()
            .expect("lock")
            .insert(key.to_string(), options.labels);
        Ok(())
    }

    async fn ts_alter(
        &self,
        key: &str,
        options: TimeSeriesCreateOptions,
    ) -> Result<(), CacheError> {
        self.labels
            .lock()
            .expect("lock")
            .insert(key.to_string(), options.labels);
        Ok(())
    }

    async fn ts_add(
        &self,
        key: &str,
        sample: TimeSeriesSample,
        _options: TimeSeriesAddOptions,
    ) -> Result<i64, CacheError> {
        self.data
            .lock()
            .expect("lock")
            .entry(key.to_string())
            .or_default()
            .push(sample.clone());
        Ok(sample.timestamp)
    }

    async fn ts_madd(&self, items: &[(String, TimeSeriesSample)]) -> Result<Vec<i64>, CacheError> {
        let mut data = self.data.lock().expect("lock");
        let mut timestamps = Vec::with_capacity(items.len());
        for (key, sample) in items {
            data.entry(key.clone()).or_default().push(sample.clone());
            timestamps.push(sample.timestamp);
        }
        Ok(timestamps)
    }

    async fn ts_incr_by(
        &self,
        key: &str,
        value: f64,
        timestamp: Option<i64>,
    ) -> Result<i64, CacheError> {
        let timestamp = timestamp.unwrap_or(1);
        self.data
            .lock()
            .expect("lock")
            .entry(key.to_string())
            .or_default()
            .push(TimeSeriesSample { timestamp, value });
        Ok(timestamp)
    }

    async fn ts_decr_by(
        &self,
        key: &str,
        value: f64,
        timestamp: Option<i64>,
    ) -> Result<i64, CacheError> {
        self.ts_incr_by(key, -value, timestamp).await
    }

    async fn ts_del(&self, key: &str, from: i64, to: i64) -> Result<i64, CacheError> {
        let mut data = self.data.lock().expect("lock");
        let series = data.entry(key.to_string()).or_default();
        let before = series.len();
        series.retain(|sample| sample.timestamp < from || sample.timestamp > to);
        Ok((before - series.len()) as i64)
    }

    async fn ts_get(&self, key: &str) -> Result<Option<TimeSeriesSample>, CacheError> {
        Ok(self
            .data
            .lock()
            .expect("lock")
            .get(key)
            .and_then(|series| series.last().cloned()))
    }

    async fn ts_mget(&self, filters: &[String]) -> Result<Vec<TimeSeriesMGetResult>, CacheError> {
        let data = self.data.lock().expect("lock");
        let labels = self.labels.lock().expect("lock");
        Ok(filters
            .iter()
            .map(|filter| TimeSeriesMGetResult {
                key: filter.clone(),
                labels: labels.get(filter).cloned().unwrap_or_default(),
                sample: data.get(filter).and_then(|series| series.last().cloned()),
            })
            .collect())
    }

    async fn ts_range(
        &self,
        key: &str,
        from: i64,
        to: i64,
        _options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesSample>, CacheError> {
        Ok(self
            .data
            .lock()
            .expect("lock")
            .get(key)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|sample| sample.timestamp >= from && sample.timestamp <= to)
            .collect())
    }

    async fn ts_rev_range(
        &self,
        key: &str,
        from: i64,
        to: i64,
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesSample>, CacheError> {
        let mut samples = self.ts_range(key, from, to, options).await?;
        samples.reverse();
        Ok(samples)
    }

    async fn ts_mrange(
        &self,
        from: i64,
        to: i64,
        filters: &[String],
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesRangeResult>, CacheError> {
        let labels = self.labels.lock().expect("lock").clone();
        let mut results = Vec::with_capacity(filters.len());
        for filter in filters {
            results.push(TimeSeriesRangeResult {
                key: filter.clone(),
                labels: labels.get(filter).cloned().unwrap_or_default(),
                samples: self.ts_range(filter, from, to, options.clone()).await?,
            });
        }
        Ok(results)
    }

    async fn ts_mrev_range(
        &self,
        from: i64,
        to: i64,
        filters: &[String],
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesRangeResult>, CacheError> {
        let mut results = self.ts_mrange(from, to, filters, options).await?;
        for result in &mut results {
            result.samples.reverse();
        }
        Ok(results)
    }

    async fn ts_query_index(&self, filters: &[String]) -> Result<Vec<String>, CacheError> {
        Ok(filters.to_vec())
    }

    async fn ts_info(&self, key: &str) -> Result<serde_json::Value, CacheError> {
        Ok(serde_json::json!({ "key": key }))
    }

    async fn ts_create_rule(
        &self,
        _source: &str,
        _dest: &str,
        _aggregation: TsAggregation,
        _bucket_duration_ms: u64,
        _align_timestamp_ms: Option<i64>,
    ) -> Result<(), CacheError> {
        Ok(())
    }

    async fn ts_delete_rule(&self, _source: &str, _dest: &str) -> Result<(), CacheError> {
        Ok(())
    }
}

#[derive(Default)]
pub struct MockKeyRepository {
    store: Mutex<HashMap<String, String>>,
}

impl MockKeyRepository {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, key: &str, value: &str) {
        self.store
            .lock()
            .expect("store lock")
            .insert(key.to_string(), value.to_string());
    }
}

#[async_trait]
impl KeyRepository for MockKeyRepository {
    async fn delete(&self, keys: &[String]) -> Result<DeleteResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let mut deleted = Vec::new();
        let mut not_found = Vec::new();
        for key in keys {
            if store.remove(key).is_some() {
                deleted.push(key.clone());
            } else {
                not_found.push(key.clone());
            }
        }
        let count = deleted.len();
        Ok(DeleteResult {
            deleted,
            not_found,
            count,
        })
    }

    async fn exists(&self, keys: &[String]) -> Result<ExistsResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        let mut existing = Vec::new();
        let mut missing = Vec::new();
        for key in keys {
            if store.contains_key(key) {
                existing.push(key.clone());
            } else {
                missing.push(key.clone());
            }
        }
        let count = existing.len();
        Ok(ExistsResult {
            existing,
            missing,
            count,
        })
    }

    async fn expire(
        &self,
        key: &str,
        _seconds: i64,
        _options: ExpireOptions,
    ) -> Result<ExpireResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(ExpireResult {
            key: key.to_string(),
            success: store.contains_key(key),
            new_ttl: None,
        })
    }

    async fn expire_at(
        &self,
        key: &str,
        _timestamp: i64,
        _options: ExpireOptions,
    ) -> Result<ExpireResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(ExpireResult {
            key: key.to_string(),
            success: store.contains_key(key),
            new_ttl: None,
        })
    }

    async fn pexpire(
        &self,
        key: &str,
        _milliseconds: i64,
        _options: ExpireOptions,
    ) -> Result<ExpireResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(ExpireResult {
            key: key.to_string(),
            success: store.contains_key(key),
            new_ttl: None,
        })
    }

    async fn pexpire_at(
        &self,
        key: &str,
        _timestamp: i64,
        _options: ExpireOptions,
    ) -> Result<ExpireResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(ExpireResult {
            key: key.to_string(),
            success: store.contains_key(key),
            new_ttl: None,
        })
    }

    async fn ttl(&self, key: &str) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        if store.contains_key(key) {
            Ok(-1) // No TTL set
        } else {
            Ok(-2) // Key doesn't exist
        }
    }

    async fn pttl(&self, key: &str) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        if store.contains_key(key) {
            Ok(-1) // No TTL set
        } else {
            Ok(-2) // Key doesn't exist
        }
    }

    async fn persist(&self, key: &str) -> Result<PersistResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(PersistResult {
            key: key.to_string(),
            success: store.contains_key(key),
        })
    }

    async fn key_type(&self, key: &str) -> Result<String, CacheError> {
        let store = self.store.lock().expect("store lock");
        if store.contains_key(key) {
            Ok("string".to_string())
        } else {
            Ok("none".to_string())
        }
    }

    async fn rename(&self, key: &str, new_key: &str) -> Result<RenameResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if let Some(value) = store.remove(key) {
            store.insert(new_key.to_string(), value);
            Ok(RenameResult {
                old_key: key.to_string(),
                new_key: new_key.to_string(),
                success: true,
            })
        } else {
            Err(CacheError::KeyNotFound(key.to_string()))
        }
    }

    async fn rename_nx(&self, key: &str, new_key: &str) -> Result<RenameResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if store.contains_key(new_key) {
            return Ok(RenameResult {
                old_key: key.to_string(),
                new_key: new_key.to_string(),
                success: false,
            });
        }
        if let Some(value) = store.remove(key) {
            store.insert(new_key.to_string(), value);
            Ok(RenameResult {
                old_key: key.to_string(),
                new_key: new_key.to_string(),
                success: true,
            })
        } else {
            Err(CacheError::KeyNotFound(key.to_string()))
        }
    }

    async fn copy(
        &self,
        source: &str,
        destination: &str,
        options: CopyOptions,
    ) -> Result<CopyResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if !options.replace && store.contains_key(destination) {
            return Ok(CopyResult {
                source: source.to_string(),
                destination: destination.to_string(),
                success: false,
            });
        }
        if let Some(value) = store.get(source).cloned() {
            store.insert(destination.to_string(), value);
            Ok(CopyResult {
                source: source.to_string(),
                destination: destination.to_string(),
                success: true,
            })
        } else {
            Ok(CopyResult {
                source: source.to_string(),
                destination: destination.to_string(),
                success: false,
            })
        }
    }

    async fn scan(
        &self,
        cursor: u64,
        pattern: Option<&str>,
        count: Option<u64>,
        _key_type: Option<&str>,
    ) -> Result<ScanResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        let keys: Vec<String> = store
            .keys()
            .filter(|k| {
                if let Some(pat) = pattern {
                    let pat = pat.replace('*', "");
                    k.contains(&pat)
                } else {
                    true
                }
            })
            .take(count.unwrap_or(10) as usize)
            .cloned()
            .collect();

        let key_count = keys.len();
        let next_cursor = if cursor == 0 && !keys.is_empty() {
            0
        } else {
            0
        };
        Ok(ScanResult {
            cursor: next_cursor,
            keys,
            count: key_count,
        })
    }

    async fn keys(&self, pattern: &str) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let pat = pattern.replace('*', "");
        Ok(store
            .keys()
            .filter(|k| pat.is_empty() || k.contains(&pat))
            .cloned()
            .collect())
    }

    async fn random_key(&self) -> Result<RandomKeyResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(RandomKeyResult {
            key: store.keys().next().cloned(),
        })
    }

    async fn touch(&self, keys: &[String]) -> Result<TouchResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        let count = keys.iter().filter(|k| store.contains_key(*k)).count();
        Ok(TouchResult { count })
    }

    async fn unlink(&self, keys: &[String]) -> Result<DeleteResult, CacheError> {
        self.delete(keys).await
    }

    async fn dump(&self, key: &str) -> Result<DumpResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        if let Some(value) = store.get(key) {
            Ok(DumpResult {
                key: key.to_string(),
                data: Some(value.clone()),
            })
        } else {
            Ok(DumpResult {
                key: key.to_string(),
                data: None,
            })
        }
    }

    async fn restore(
        &self,
        key: &str,
        _ttl: i64,
        data: &[u8],
        replace: bool,
    ) -> Result<bool, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if !replace && store.contains_key(key) {
            return Ok(false);
        }
        let value = String::from_utf8_lossy(data).to_string();
        store.insert(key.to_string(), value);
        Ok(true)
    }

    async fn object_encoding(&self, key: &str) -> Result<Option<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        if store.contains_key(key) {
            Ok(Some("embstr".to_string()))
        } else {
            Ok(None)
        }
    }

    async fn object_idletime(&self, key: &str) -> Result<Option<u64>, CacheError> {
        let store = self.store.lock().expect("store lock");
        if store.contains_key(key) {
            Ok(Some(0))
        } else {
            Ok(None)
        }
    }

    async fn object_refcount(&self, key: &str) -> Result<Option<u64>, CacheError> {
        let store = self.store.lock().expect("store lock");
        if store.contains_key(key) {
            Ok(Some(1))
        } else {
            Ok(None)
        }
    }

    async fn object_freq(&self, key: &str) -> Result<Option<u64>, CacheError> {
        let store = self.store.lock().expect("store lock");
        if store.contains_key(key) {
            Ok(Some(0))
        } else {
            Ok(None)
        }
    }

    async fn key_info(&self, key: &str) -> Result<KeyInfo, CacheError> {
        let store = self.store.lock().expect("store lock");
        if store.contains_key(key) {
            Ok(KeyInfo::new(key.to_string(), "string".to_string(), -1))
        } else {
            Ok(KeyInfo::not_found(key.to_string()))
        }
    }

    async fn expire_time(&self, key: &str) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        if store.contains_key(key) {
            Ok(-1)
        } else {
            Ok(-2)
        }
    }

    async fn pexpire_time(&self, key: &str) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        if store.contains_key(key) {
            Ok(-1)
        } else {
            Ok(-2)
        }
    }

    async fn sort(
        &self,
        _key: &str,
        _options: SortOptions,
    ) -> Result<Vec<Option<String>>, CacheError> {
        Ok(vec![])
    }

    async fn sort_store(
        &self,
        _key: &str,
        _destination: &str,
        _options: SortOptions,
    ) -> Result<i64, CacheError> {
        Ok(0)
    }

    async fn sort_ro(
        &self,
        _key: &str,
        _options: SortOptions,
    ) -> Result<Vec<Option<String>>, CacheError> {
        Ok(vec![])
    }
}

#[derive(Default)]
pub struct MockHashRepository {
    store: Mutex<HashMap<String, HashMap<String, String>>>,
}

impl MockHashRepository {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, key: &str, field: &str, value: &str) {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        entry.insert(field.to_string(), value.to_string());
    }
}

#[async_trait]
impl HashRepository for MockHashRepository {
    async fn hget(&self, key: &str, field: &str) -> Result<Option<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).and_then(|map| map.get(field).cloned()))
    }

    async fn hset(&self, key: &str, pairs: Vec<(String, String)>) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let mut new_fields = 0;
        for (field, value) in pairs {
            if !entry.contains_key(&field) {
                new_fields += 1;
            }
            entry.insert(field, value);
        }
        Ok(new_fields)
    }

    async fn hset_nx(&self, key: &str, field: &str, value: &str) -> Result<bool, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        if entry.contains_key(field) {
            return Ok(false);
        }
        entry.insert(field.to_string(), value.to_string());
        Ok(true)
    }

    async fn hgetall(&self, key: &str) -> Result<HashMap<String, String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).cloned().unwrap_or_default())
    }

    async fn hmget(&self, key: &str, fields: &[String]) -> Result<Vec<Option<String>>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let map = store.get(key);
        Ok(fields
            .iter()
            .map(|field| map.and_then(|m| m.get(field).cloned()))
            .collect())
    }

    async fn hmset(&self, key: &str, pairs: Vec<(String, String)>) -> Result<(), CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        for (field, value) in pairs {
            entry.insert(field, value);
        }
        Ok(())
    }

    async fn hdel(&self, key: &str, fields: &[String]) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let mut removed = 0;
        for field in fields {
            if entry.remove(field).is_some() {
                removed += 1;
            }
        }
        Ok(removed)
    }

    async fn hexists(&self, key: &str, field: &str) -> Result<bool, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).is_some_and(|map| map.contains_key(field)))
    }

    async fn hkeys(&self, key: &str) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store
            .get(key)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default())
    }

    async fn hvals(&self, key: &str) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store
            .get(key)
            .map(|map| map.values().cloned().collect())
            .unwrap_or_default())
    }

    async fn hlen(&self, key: &str) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).map_or(0, |map| map.len() as i64))
    }

    async fn hincr_by(&self, key: &str, field: &str, delta: i64) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let current = entry
            .get(field)
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(0);
        let next = current + delta;
        entry.insert(field.to_string(), next.to_string());
        Ok(next)
    }

    async fn hincr_by_float(&self, key: &str, field: &str, delta: f64) -> Result<f64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let current = entry
            .get(field)
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.0);
        let next = current + delta;
        entry.insert(field.to_string(), next.to_string());
        Ok(next)
    }

    async fn hstr_len(&self, key: &str, field: &str) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store
            .get(key)
            .and_then(|map| map.get(field))
            .map_or(0, |value| value.len() as i64))
    }

    async fn hrand_field(
        &self,
        key: &str,
        count: Option<i64>,
        with_values: bool,
    ) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let map = match store.get(key) {
            Some(map) => map,
            None => return Ok(Vec::new()),
        };
        let mut fields: Vec<String> = map.keys().cloned().collect();
        fields.sort();

        if count.is_none() {
            return Ok(fields.into_iter().take(1).collect());
        }

        let count = count.unwrap();
        let count = if count < 0 { -count } else { count } as usize;
        let selected = fields.into_iter().take(count).collect::<Vec<_>>();

        if with_values {
            let mut result = Vec::new();
            for field in selected {
                if let Some(value) = map.get(&field) {
                    result.push(field);
                    result.push(value.clone());
                }
            }
            Ok(result)
        } else {
            Ok(selected)
        }
    }

    async fn hscan(
        &self,
        key: &str,
        _cursor: u64,
        pattern: Option<String>,
        count: Option<u64>,
    ) -> Result<(u64, Vec<String>), CacheError> {
        let store = self.store.lock().expect("store lock");
        let map = match store.get(key) {
            Some(map) => map,
            None => return Ok((0, Vec::new())),
        };
        let pattern = pattern.unwrap_or_default().replace('*', "");
        let mut result = Vec::new();
        for (field, value) in map.iter() {
            if pattern.is_empty() || field.contains(&pattern) {
                result.push(field.clone());
                result.push(value.clone());
            }
            if let Some(limit) = count
                && (result.len() / 2) as u64 >= limit
            {
                break;
            }
        }
        Ok((0, result))
    }

    async fn hexpire(
        &self,
        _key: &str,
        _seconds: i64,
        fields: &[String],
        _condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        Ok(vec![1; fields.len()])
    }

    async fn hpexpire(
        &self,
        _key: &str,
        _milliseconds: i64,
        fields: &[String],
        _condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        Ok(vec![1; fields.len()])
    }

    async fn hexpire_at(
        &self,
        _key: &str,
        _unix_time: i64,
        fields: &[String],
        _condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        Ok(vec![1; fields.len()])
    }

    async fn hpexpire_at(
        &self,
        _key: &str,
        _unix_time_ms: i64,
        fields: &[String],
        _condition: Option<ExpireCondition>,
    ) -> Result<Vec<i64>, CacheError> {
        Ok(vec![1; fields.len()])
    }

    async fn hexpire_time(&self, _key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError> {
        Ok(vec![-1; fields.len()])
    }

    async fn hpexpire_time(&self, _key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError> {
        Ok(vec![-1; fields.len()])
    }

    async fn httl(&self, _key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError> {
        Ok(vec![-1; fields.len()])
    }

    async fn hpttl(&self, _key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError> {
        Ok(vec![-1; fields.len()])
    }

    async fn hpersist(&self, _key: &str, fields: &[String]) -> Result<Vec<i64>, CacheError> {
        Ok(vec![1; fields.len()])
    }

    async fn hgetex(
        &self,
        key: &str,
        fields: &[String],
        _expiration: Option<HashExpiration>,
    ) -> Result<Vec<Option<String>>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let map = store.get(key);
        Ok(fields
            .iter()
            .map(|field| map.and_then(|m| m.get(field).cloned()))
            .collect())
    }

    async fn hsetex(
        &self,
        key: &str,
        field_values: &[(String, String)],
        condition: Option<HSetExCondition>,
        _expiration: Option<HashExpiration>,
    ) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let mut count = 0i64;
        for (field, value) in field_values {
            match &condition {
                Some(HSetExCondition::FNX) => {
                    if !entry.contains_key(field) {
                        entry.insert(field.clone(), value.clone());
                        count += 1;
                    }
                }
                Some(HSetExCondition::FXX) => {
                    if entry.contains_key(field) {
                        entry.insert(field.clone(), value.clone());
                        count += 1;
                    }
                }
                None => {
                    entry.insert(field.clone(), value.clone());
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    async fn hgetdel(
        &self,
        key: &str,
        fields: &[String],
    ) -> Result<Vec<Option<String>>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        Ok(fields.iter().map(|field| entry.remove(field)).collect())
    }
}

/// Mock List Repository for testing
#[derive(Default)]
pub struct MockListRepository {
    store: Mutex<HashMap<String, Vec<String>>>,
}

impl MockListRepository {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, key: &str, values: Vec<String>) {
        let mut store = self.store.lock().expect("store lock");
        store.insert(key.to_string(), values);
    }
}

#[async_trait]
impl ListRepository for MockListRepository {
    async fn lpush(&self, key: &str, values: &[String]) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        // Redis LPUSH pushes values left-to-right, each at the head
        // So LPUSH key a b c results in [c, b, a, ...]
        for value in values.iter() {
            entry.insert(0, value.clone());
        }
        Ok(entry.len() as i64)
    }

    async fn rpush(&self, key: &str, values: &[String]) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        entry.extend(values.iter().cloned());
        Ok(entry.len() as i64)
    }

    async fn lpush_x(&self, key: &str, values: &[String]) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if let Some(entry) = store.get_mut(key) {
            for value in values.iter() {
                entry.insert(0, value.clone());
            }
            Ok(entry.len() as i64)
        } else {
            Ok(0)
        }
    }

    async fn rpush_x(&self, key: &str, values: &[String]) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if let Some(entry) = store.get_mut(key) {
            entry.extend(values.iter().cloned());
            Ok(entry.len() as i64)
        } else {
            Ok(0)
        }
    }

    async fn lpop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let count = count.unwrap_or(1) as usize;
        let result: Vec<String> = entry.drain(..count.min(entry.len())).collect();
        Ok(result)
    }

    async fn rpop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let count = count.unwrap_or(1) as usize;
        let start = entry.len().saturating_sub(count);
        let result: Vec<String> = entry.drain(start..).collect();
        Ok(result)
    }

    async fn lrange(&self, key: &str, start: i64, stop: i64) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let entry = store.get(key).cloned().unwrap_or_default();
        let len = entry.len() as i64;

        let start = if start < 0 {
            (len + start).max(0)
        } else {
            start
        };
        let stop = if stop < 0 { len + stop } else { stop };

        let start = start as usize;
        let stop = (stop + 1).min(len) as usize;

        if start >= entry.len() || start >= stop {
            return Ok(Vec::new());
        }

        Ok(entry[start..stop].to_vec())
    }

    async fn llen(&self, key: &str) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).map_or(0, |v| v.len() as i64))
    }

    async fn lindex(&self, key: &str, index: i64) -> Result<Option<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let entry = store.get(key).cloned().unwrap_or_default();
        let len = entry.len() as i64;
        let index = if index < 0 { len + index } else { index };
        if index < 0 || index >= len {
            return Ok(None);
        }
        Ok(entry.get(index as usize).cloned())
    }

    async fn lset(&self, key: &str, index: i64, value: &str) -> Result<(), CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let len = entry.len() as i64;
        let index = if index < 0 { len + index } else { index };
        if index < 0 || index >= len {
            return Err(CacheError::InvalidInput("index out of range".to_string()));
        }
        entry[index as usize] = value.to_string();
        Ok(())
    }

    async fn linsert(
        &self,
        key: &str,
        position: InsertPosition,
        pivot: &str,
        value: &str,
    ) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        if let Some(pos) = entry.iter().position(|v| v == pivot) {
            let insert_pos = match position {
                InsertPosition::Before => pos,
                InsertPosition::After => pos + 1,
            };
            entry.insert(insert_pos, value.to_string());
            Ok(entry.len() as i64)
        } else {
            Ok(-1) // pivot not found
        }
    }

    async fn lrem(&self, key: &str, count: i64, value: &str) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let mut removed = 0i64;
        let abs_count = count.unsigned_abs() as usize;

        if count == 0 {
            // Remove all occurrences
            entry.retain(|v| {
                if v == value {
                    removed += 1;
                    false
                } else {
                    true
                }
            });
        } else if count > 0 {
            // Remove from head
            let mut indices = Vec::new();
            for (i, v) in entry.iter().enumerate() {
                if v == value && indices.len() < abs_count {
                    indices.push(i);
                }
            }
            for i in indices.into_iter().rev() {
                entry.remove(i);
                removed += 1;
            }
        } else {
            // Remove from tail
            let mut indices = Vec::new();
            for (i, v) in entry.iter().enumerate().rev() {
                if v == value && indices.len() < abs_count {
                    indices.push(i);
                }
            }
            indices.sort();
            for i in indices.into_iter().rev() {
                entry.remove(i);
                removed += 1;
            }
        }
        Ok(removed)
    }

    async fn ltrim(&self, key: &str, start: i64, stop: i64) -> Result<(), CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let len = entry.len() as i64;

        let start = if start < 0 {
            (len + start).max(0)
        } else {
            start
        };
        let stop = if stop < 0 { len + stop } else { stop };

        let start = start as usize;
        let stop = (stop + 1).min(len) as usize;

        if start >= entry.len() || start >= stop {
            entry.clear();
        } else {
            *entry = entry[start..stop].to_vec();
        }
        Ok(())
    }

    async fn lpos(
        &self,
        key: &str,
        element: &str,
        options: LPosOptions,
    ) -> Result<Vec<i64>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let entry = store.get(key).cloned().unwrap_or_default();

        let mut indices = Vec::new();
        let count = options.count.unwrap_or(1) as usize;

        for (i, v) in entry.iter().enumerate() {
            if v == element {
                indices.push(i as i64);
                if indices.len() >= count {
                    break;
                }
            }
        }
        Ok(indices)
    }

    async fn lmove(
        &self,
        source: &str,
        destination: &str,
        src_dir: ListDirection,
        dst_dir: ListDirection,
    ) -> Result<Option<String>, CacheError> {
        let mut store = self.store.lock().expect("store lock");

        let value = {
            let src = store.get_mut(source);
            match (src, src_dir) {
                (Some(list), ListDirection::Left) if !list.is_empty() => Some(list.remove(0)),
                (Some(list), ListDirection::Right) if !list.is_empty() => list.pop(),
                _ => None,
            }
        };

        if let Some(v) = value.clone() {
            let dst = store.entry(destination.to_string()).or_default();
            match dst_dir {
                ListDirection::Left => dst.insert(0, v),
                ListDirection::Right => dst.push(v),
            }
        }

        Ok(value)
    }

    async fn rpop_lpush(
        &self,
        source: &str,
        destination: &str,
    ) -> Result<Option<String>, CacheError> {
        self.lmove(
            source,
            destination,
            ListDirection::Right,
            ListDirection::Left,
        )
        .await
    }

    async fn blpop(
        &self,
        keys: &[String],
        _timeout: Duration,
    ) -> Result<Option<BlockingPopResult>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        for key in keys {
            if let Some(list) = store.get_mut(key)
                && !list.is_empty()
            {
                let value = list.remove(0);
                return Ok(Some(BlockingPopResult {
                    key: key.clone(),
                    value,
                }));
            }
        }
        Ok(None)
    }

    async fn brpop(
        &self,
        keys: &[String],
        _timeout: Duration,
    ) -> Result<Option<BlockingPopResult>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        for key in keys {
            if let Some(list) = store.get_mut(key)
                && !list.is_empty()
            {
                let value = list.pop().unwrap();
                return Ok(Some(BlockingPopResult {
                    key: key.clone(),
                    value,
                }));
            }
        }
        Ok(None)
    }

    async fn blmove(
        &self,
        source: &str,
        destination: &str,
        src_dir: ListDirection,
        dst_dir: ListDirection,
        _timeout: Duration,
    ) -> Result<Option<String>, CacheError> {
        self.lmove(source, destination, src_dir, dst_dir).await
    }

    async fn brpop_lpush(
        &self,
        source: &str,
        destination: &str,
        _timeout: Duration,
    ) -> Result<Option<String>, CacheError> {
        self.rpop_lpush(source, destination).await
    }

    async fn lmpop(
        &self,
        keys: &[String],
        direction: ListDirection,
        count: Option<u32>,
    ) -> Result<Option<LMPopResult>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let count = count.unwrap_or(1) as usize;
        for key in keys {
            if let Some(list) = store.get_mut(key.as_str())
                && !list.is_empty()
            {
                let mut elements = Vec::new();
                for _ in 0..count.min(list.len()) {
                    let elem = match direction {
                        ListDirection::Left => list.remove(0),
                        ListDirection::Right => list.pop().unwrap(),
                    };
                    elements.push(elem);
                }
                return Ok(Some(LMPopResult {
                    key: key.clone(),
                    elements,
                }));
            }
        }
        Ok(None)
    }

    async fn blmpop(
        &self,
        keys: &[String],
        direction: ListDirection,
        _timeout: Duration,
        count: Option<u32>,
    ) -> Result<Option<LMPopResult>, CacheError> {
        // Mock just delegates to lmpop (no actual blocking in tests)
        self.lmpop(keys, direction, count).await
    }
}

/// Mock Set Repository for testing
#[derive(Default)]
pub struct MockSetRepository {
    store: Mutex<HashMap<String, std::collections::HashSet<String>>>,
}

impl MockSetRepository {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, key: &str, members: Vec<String>) {
        let mut store = self.store.lock().expect("store lock");
        let set = store.entry(key.to_string()).or_default();
        for member in members {
            set.insert(member);
        }
    }
}

#[async_trait]
impl SetRepository for MockSetRepository {
    async fn sadd(&self, key: &str, members: &[String]) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let set = store.entry(key.to_string()).or_default();
        let mut added = 0i64;
        for member in members {
            if set.insert(member.clone()) {
                added += 1;
            }
        }
        Ok(added)
    }

    async fn srem(&self, key: &str, members: &[String]) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if let Some(set) = store.get_mut(key) {
            let mut removed = 0i64;
            for member in members {
                if set.remove(member) {
                    removed += 1;
                }
            }
            Ok(removed)
        } else {
            Ok(0)
        }
    }

    async fn smembers(&self, key: &str) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store
            .get(key)
            .map_or(Vec::new(), |s| s.iter().cloned().collect()))
    }

    async fn sismember(&self, key: &str, member: &str) -> Result<bool, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).is_some_and(|s| s.contains(member)))
    }

    async fn smismember(&self, key: &str, members: &[String]) -> Result<Vec<bool>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let set = store.get(key);
        Ok(members
            .iter()
            .map(|m| set.is_some_and(|s| s.contains(m)))
            .collect())
    }

    async fn scard(&self, key: &str) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).map_or(0, |s| s.len() as i64))
    }

    async fn srandmember(&self, key: &str, count: Option<i64>) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        if let Some(set) = store.get(key) {
            let count = count.unwrap_or(1).unsigned_abs() as usize;
            Ok(set.iter().take(count).cloned().collect())
        } else {
            Ok(Vec::new())
        }
    }

    async fn spop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if let Some(set) = store.get_mut(key) {
            let count = count.unwrap_or(1) as usize;
            let members: Vec<String> = set.iter().take(count).cloned().collect();
            for m in &members {
                set.remove(m);
            }
            Ok(members)
        } else {
            Ok(Vec::new())
        }
    }

    async fn smove(
        &self,
        source: &str,
        destination: &str,
        member: &str,
    ) -> Result<bool, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let removed = store.get_mut(source).is_some_and(|s| s.remove(member));
        if removed {
            store
                .entry(destination.to_string())
                .or_default()
                .insert(member.to_string());
        }
        Ok(removed)
    }

    async fn sinter(&self, keys: &[String]) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let first = store.get(&keys[0]).cloned().unwrap_or_default();
        let result: std::collections::HashSet<String> = keys[1..].iter().fold(first, |acc, k| {
            store
                .get(k)
                .map_or_else(std::collections::HashSet::new, |s| {
                    acc.intersection(s).cloned().collect()
                })
        });
        Ok(result.into_iter().collect())
    }

    async fn sinterstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError> {
        let result = self.sinter(keys).await?;
        let count = result.len() as i64;
        let mut store = self.store.lock().expect("store lock");
        store.insert(destination.to_string(), result.into_iter().collect());
        Ok(count)
    }

    async fn sintercard(&self, keys: &[String], limit: Option<u64>) -> Result<i64, CacheError> {
        let result = self.sinter(keys).await?;
        let count = result.len() as i64;
        Ok(limit.map_or(count, |l| count.min(l as i64)))
    }

    async fn sunion(&self, keys: &[String]) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let mut result = std::collections::HashSet::new();
        for key in keys {
            if let Some(set) = store.get(key) {
                result.extend(set.iter().cloned());
            }
        }
        Ok(result.into_iter().collect())
    }

    async fn sunionstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError> {
        let result = self.sunion(keys).await?;
        let count = result.len() as i64;
        let mut store = self.store.lock().expect("store lock");
        store.insert(destination.to_string(), result.into_iter().collect());
        Ok(count)
    }

    async fn sdiff(&self, keys: &[String]) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let first = store.get(&keys[0]).cloned().unwrap_or_default();
        let result: std::collections::HashSet<String> = keys[1..].iter().fold(first, |acc, k| {
            store
                .get(k)
                .map_or(acc.clone(), |s| acc.difference(s).cloned().collect())
        });
        Ok(result.into_iter().collect())
    }

    async fn sdiffstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError> {
        let result = self.sdiff(keys).await?;
        let count = result.len() as i64;
        let mut store = self.store.lock().expect("store lock");
        store.insert(destination.to_string(), result.into_iter().collect());
        Ok(count)
    }

    async fn sscan(
        &self,
        key: &str,
        _cursor: u64,
        _pattern: Option<&str>,
        _count: Option<u64>,
    ) -> Result<SetScanResult, CacheError> {
        let members = self.smembers(key).await?;
        Ok(SetScanResult { cursor: 0, members })
    }
}

pub fn test_state_with_repos(
    string_repo: Arc<MockStringRepository>,
    key_repo: Arc<MockKeyRepository>,
    admin_repo: Arc<MockAdminRepository>,
) -> AppState {
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    test_state_with_all_repos(
        string_repo,
        hash_repo,
        list_repo,
        set_repo,
        sorted_set_repo,
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo,
        json_repo,
        search_repo,
        bloom_repo,
        probabilistic_repo,
        geo_repo,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn test_state_with_all_repos(
    string_repo: Arc<MockStringRepository>,
    hash_repo: Arc<MockHashRepository>,
    list_repo: Arc<MockListRepository>,
    set_repo: Arc<MockSetRepository>,
    sorted_set_repo: Arc<MockSortedSetRepository>,
    bitmap_repo: Arc<MockBitMapRepository>,
    key_repo: Arc<MockKeyRepository>,
    admin_repo: Arc<MockAdminRepository>,
    stream_repo: Arc<MockStreamRepository>,
    json_repo: Arc<MockJsonRepository>,
    search_repo: Arc<MockSearchRepository>,
    bloom_repo: Arc<MockBloomRepository>,
    probabilistic_repo: Arc<MockProbabilisticRepository>,
    geo_repo: Arc<MockGeoRepository>,
) -> AppState {
    test_state_with_all_repos_and_config(
        string_repo,
        hash_repo,
        list_repo,
        set_repo,
        sorted_set_repo,
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo,
        json_repo,
        search_repo,
        bloom_repo,
        probabilistic_repo,
        geo_repo,
        Settings::default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn test_state_with_all_repos_and_config(
    string_repo: Arc<MockStringRepository>,
    hash_repo: Arc<MockHashRepository>,
    list_repo: Arc<MockListRepository>,
    set_repo: Arc<MockSetRepository>,
    sorted_set_repo: Arc<MockSortedSetRepository>,
    bitmap_repo: Arc<MockBitMapRepository>,
    key_repo: Arc<MockKeyRepository>,
    admin_repo: Arc<MockAdminRepository>,
    stream_repo: Arc<MockStreamRepository>,
    json_repo: Arc<MockJsonRepository>,
    search_repo: Arc<MockSearchRepository>,
    bloom_repo: Arc<MockBloomRepository>,
    probabilistic_repo: Arc<MockProbabilisticRepository>,
    geo_repo: Arc<MockGeoRepository>,
    config: Settings,
) -> AppState {
    let pool = Arc::new(InstrumentedPool::new_for_tests());
    let config = Arc::new(config);
    let capabilities = Arc::new(RedisCapabilities::default_capabilities());
    let sse_semaphore = Arc::new(tokio::sync::Semaphore::new(
        config.blocking.max_sse_connections,
    ));
    let string_service = Arc::new(StringService::new_with_repository(string_repo));
    let hash_service = Arc::new(HashService::new_with_repository(hash_repo));
    let list_service = Arc::new(ListService::new_with_repository(list_repo));
    let set_service = Arc::new(SetService::new_with_repository(set_repo));
    let sorted_set_service = Arc::new(SortedSetService::new_with_repository(sorted_set_repo));
    let bitmap_service = Arc::new(BitMapService::new_with_repository(bitmap_repo));
    let key_service = Arc::new(KeyService::new_with_repository(key_repo));
    let admin_service = Arc::new(AdminService::new_with_repository(admin_repo));
    let stream_service = Arc::new(StreamService::new_with_repository(stream_repo));
    let json_service = Arc::new(JsonService::new_with_repository(json_repo));
    let search_service = Arc::new(SearchService::new_with_repository(search_repo));
    let bloom_service = Arc::new(BloomService::new_with_repository(bloom_repo));
    let probabilistic_service = Arc::new(ProbabilisticService::new_with_repository(
        probabilistic_repo,
    ));
    let geo_service = Arc::new(GeoService::new_with_repository(geo_repo));

    // Create PubSubManager and PubSubService for tests (uses localhost which may not be available)
    let pubsub_manager = Arc::new(
        PubSubManager::new(&config.redis.url, config.pubsub.clone())
            .expect("Failed to create PubSubManager for tests"),
    );
    let pubsub_service = Arc::new(PubSubService::new(pool.clone(), pubsub_manager));
    let transaction_service = Arc::new(crate::application::services::TransactionService::new(
        pool.clone(),
    ));
    let scripting_service = Arc::new(crate::application::services::ScriptingService::new(
        pool.clone(),
    ));
    let function_service = Arc::new(FunctionService::new_with_repository(Arc::new(
        MockFunctionRepository,
    )));
    let timeseries_service = Arc::new(TimeSeriesService::new_with_repository(Arc::new(
        MockTimeSeriesRepository::new(),
    )));
    let cluster_service = Arc::new(ClusterService::new(Arc::new(MockClusterRepository)));
    let vector_service = Arc::new(crate::application::services::VectorService::new(Arc::new(
        MockVectorRepository,
    )));

    AppState::new_with_services(
        pool,
        config,
        capabilities,
        sse_semaphore,
        string_service,
        hash_service,
        list_service,
        set_service,
        sorted_set_service,
        bitmap_service,
        key_service,
        admin_service,
        stream_service,
        json_service,
        search_service,
        bloom_service,
        probabilistic_service,
        geo_service,
        pubsub_service,
        transaction_service,
        scripting_service,
        function_service,
        timeseries_service,
        cluster_service,
        vector_service,
        None,
        None,
    )
}

/// Create test state with custom config
pub fn test_state_with_config(
    config: Settings,
) -> (
    AppState,
    Arc<MockStringRepository>,
    Arc<MockKeyRepository>,
    Arc<MockAdminRepository>,
) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos_and_config(
        string_repo.clone(),
        hash_repo,
        list_repo,
        set_repo,
        sorted_set_repo,
        bitmap_repo,
        key_repo.clone(),
        admin_repo.clone(),
        stream_repo,
        json_repo,
        search_repo,
        bloom_repo,
        probabilistic_repo,
        geo_repo,
        config,
    );
    (state, string_repo, key_repo, admin_repo)
}

pub fn test_state() -> (
    AppState,
    Arc<MockStringRepository>,
    Arc<MockKeyRepository>,
    Arc<MockAdminRepository>,
) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let state = test_state_with_repos(string_repo.clone(), key_repo.clone(), admin_repo.clone());
    (state, string_repo, key_repo, admin_repo)
}

pub fn test_state_with_hash_repo() -> (AppState, Arc<MockHashRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos(
        string_repo,
        hash_repo.clone(),
        list_repo,
        set_repo,
        sorted_set_repo,
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo,
        json_repo,
        search_repo,
        bloom_repo,
        probabilistic_repo,
        geo_repo,
    );
    (state, hash_repo)
}

pub fn test_state_with_function_repo() -> (AppState, Arc<MockFunctionRepository>) {
    let function_repo = Arc::new(MockFunctionRepository);
    let mut state = test_state_with_repos(
        Arc::new(MockStringRepository::new()),
        Arc::new(MockKeyRepository::new()),
        Arc::new(MockAdminRepository),
    );
    state.function_service = Arc::new(FunctionService::new_with_repository(function_repo.clone()));
    let mut caps = (*state.capabilities).clone();
    caps.features.functions = true;
    state.capabilities = Arc::new(caps);
    (state, function_repo)
}

pub fn test_state_with_timeseries_repo() -> (AppState, Arc<MockTimeSeriesRepository>) {
    let timeseries_repo = Arc::new(MockTimeSeriesRepository::new());
    let mut state = test_state_with_repos(
        Arc::new(MockStringRepository::new()),
        Arc::new(MockKeyRepository::new()),
        Arc::new(MockAdminRepository),
    );
    state.timeseries_service = Arc::new(TimeSeriesService::new_with_repository(
        timeseries_repo.clone(),
    ));
    let mut caps = (*state.capabilities).clone();
    caps.modules.timeseries = true;
    state.capabilities = Arc::new(caps);
    (state, timeseries_repo)
}

pub fn test_state_with_list_repo() -> (AppState, Arc<MockListRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos(
        string_repo,
        hash_repo,
        list_repo.clone(),
        set_repo,
        sorted_set_repo,
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo,
        json_repo,
        search_repo,
        bloom_repo,
        probabilistic_repo,
        geo_repo,
    );
    (state, list_repo)
}

pub fn test_state_with_set_repo() -> (AppState, Arc<MockSetRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos(
        string_repo,
        hash_repo,
        list_repo,
        set_repo.clone(),
        sorted_set_repo,
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo,
        json_repo,
        search_repo,
        bloom_repo,
        probabilistic_repo,
        geo_repo,
    );
    (state, set_repo)
}

pub fn test_state_with_sorted_set_repo() -> (AppState, Arc<MockSortedSetRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos(
        string_repo,
        hash_repo,
        list_repo,
        set_repo,
        sorted_set_repo.clone(),
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo,
        json_repo,
        search_repo,
        bloom_repo,
        probabilistic_repo,
        geo_repo,
    );
    (state, sorted_set_repo)
}

pub fn test_state_with_stream_repo() -> (AppState, Arc<MockStreamRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos(
        string_repo,
        hash_repo,
        list_repo,
        set_repo,
        sorted_set_repo,
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo.clone(),
        json_repo,
        search_repo,
        bloom_repo,
        probabilistic_repo,
        geo_repo,
    );
    (state, stream_repo)
}

pub fn test_state_with_json_repo() -> (AppState, Arc<MockJsonRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos(
        string_repo,
        hash_repo,
        list_repo,
        set_repo,
        sorted_set_repo,
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo,
        json_repo.clone(),
        search_repo,
        bloom_repo,
        probabilistic_repo,
        geo_repo,
    );
    (state, json_repo)
}

pub fn test_state_with_search_repo() -> (AppState, Arc<MockSearchRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos(
        string_repo,
        hash_repo,
        list_repo,
        set_repo,
        sorted_set_repo,
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo,
        json_repo,
        search_repo.clone(),
        bloom_repo,
        probabilistic_repo,
        geo_repo,
    );
    (state, search_repo)
}

pub fn test_state_with_bloom_repo() -> (AppState, Arc<MockBloomRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos(
        string_repo,
        hash_repo,
        list_repo,
        set_repo,
        sorted_set_repo,
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo,
        json_repo,
        search_repo,
        bloom_repo.clone(),
        probabilistic_repo,
        geo_repo,
    );
    (state, bloom_repo)
}

pub fn test_state_with_probabilistic_repo() -> (AppState, Arc<MockProbabilisticRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos(
        string_repo,
        hash_repo,
        list_repo,
        set_repo,
        sorted_set_repo,
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo,
        json_repo,
        search_repo,
        bloom_repo,
        probabilistic_repo.clone(),
        geo_repo,
    );
    (state, probabilistic_repo)
}

pub fn test_state_with_bitmap_repo() -> (AppState, Arc<MockBitMapRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos(
        string_repo,
        hash_repo,
        list_repo,
        set_repo,
        sorted_set_repo,
        bitmap_repo.clone(),
        key_repo,
        admin_repo,
        stream_repo,
        json_repo,
        search_repo,
        bloom_repo,
        probabilistic_repo,
        geo_repo,
    );
    (state, bitmap_repo)
}

pub fn test_state_with_geo_repo() -> (AppState, Arc<MockGeoRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository);
    let hash_repo = Arc::new(MockHashRepository::new());
    let list_repo = Arc::new(MockListRepository::new());
    let set_repo = Arc::new(MockSetRepository::new());
    let sorted_set_repo = Arc::new(MockSortedSetRepository::new());
    let bitmap_repo = Arc::new(MockBitMapRepository::new());
    let stream_repo = Arc::new(MockStreamRepository::new());
    let json_repo = Arc::new(MockJsonRepository::new());
    let search_repo = Arc::new(MockSearchRepository::new());
    let bloom_repo = Arc::new(MockBloomRepository::new());
    let probabilistic_repo = Arc::new(MockProbabilisticRepository::new());
    let geo_repo = Arc::new(MockGeoRepository::new());
    let state = test_state_with_all_repos(
        string_repo,
        hash_repo,
        list_repo,
        set_repo,
        sorted_set_repo,
        bitmap_repo,
        key_repo,
        admin_repo,
        stream_repo,
        json_repo,
        search_repo,
        bloom_repo,
        probabilistic_repo,
        geo_repo.clone(),
    );
    (state, geo_repo)
}

/// Mock Sorted Set Repository for testing
#[derive(Default)]
pub struct MockSortedSetRepository {
    store: Mutex<HashMap<String, Vec<ScoredMember>>>,
}

impl MockSortedSetRepository {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    fn sort_members(members: &mut [ScoredMember]) {
        members.sort_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.member.cmp(&b.member))
        });
    }
}

#[async_trait]
impl SortedSetRepository for MockSortedSetRepository {
    async fn zadd(
        &self,
        key: &str,
        members: &[ScoredMember],
        _options: Option<ZAddOptions>,
    ) -> Result<ZAddResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let mut added = 0i64;

        for member in members {
            let existing = entry.iter().position(|m| m.member == member.member);
            match existing {
                Some(pos) => {
                    entry[pos] = member.clone();
                }
                None => {
                    entry.push(member.clone());
                    added += 1;
                }
            }
        }

        Self::sort_members(entry);
        Ok(ZAddResult {
            count: added,
            new_score: None,
        })
    }

    async fn zadd_incr(
        &self,
        key: &str,
        member: &str,
        score: f64,
        _options: Option<ZAddOptions>,
    ) -> Result<Option<f64>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();

        let existing = entry.iter().position(|m| m.member == member);
        let new_score = match existing {
            Some(pos) => {
                entry[pos].score += score;
                entry[pos].score
            }
            None => {
                entry.push(ScoredMember::new(member.to_string(), score));
                score
            }
        };

        Self::sort_members(entry);
        Ok(Some(new_score))
    }

    async fn zrem(&self, key: &str, members: &[String]) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if let Some(entry) = store.get_mut(key) {
            let before = entry.len();
            entry.retain(|m| !members.contains(&m.member));
            Ok((before - entry.len()) as i64)
        } else {
            Ok(0)
        }
    }

    async fn zscore(&self, key: &str, member: &str) -> Result<Option<f64>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store
            .get(key)
            .and_then(|entry| entry.iter().find(|m| m.member == member))
            .map(|m| m.score))
    }

    async fn zmscore(&self, key: &str, members: &[String]) -> Result<Vec<Option<f64>>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let entry = store.get(key);
        Ok(members
            .iter()
            .map(|m| entry.and_then(|e| e.iter().find(|sm| sm.member == *m).map(|sm| sm.score)))
            .collect())
    }

    async fn zincrby(&self, key: &str, member: &str, increment: f64) -> Result<f64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();

        let existing = entry.iter().position(|m| m.member == member);
        let new_score = match existing {
            Some(pos) => {
                entry[pos].score += increment;
                entry[pos].score
            }
            None => {
                entry.push(ScoredMember::new(member.to_string(), increment));
                increment
            }
        };

        Self::sort_members(entry);
        Ok(new_score)
    }

    async fn zcard(&self, key: &str) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).map_or(0, |e| e.len() as i64))
    }

    async fn zcount(&self, key: &str, range: &ScoreRange) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).map_or(0, |entry| {
            entry
                .iter()
                .filter(|m| {
                    let min_ok = if range.min_exclusive {
                        m.score > range.min
                    } else {
                        m.score >= range.min
                    };
                    let max_ok = if range.max_exclusive {
                        m.score < range.max
                    } else {
                        m.score <= range.max
                    };
                    min_ok && max_ok
                })
                .count() as i64
        }))
    }

    async fn zlexcount(&self, key: &str, _range: &LexRange) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).map_or(0, |e| e.len() as i64))
    }

    async fn zrank(&self, key: &str, member: &str) -> Result<Option<i64>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store
            .get(key)
            .and_then(|entry| entry.iter().position(|m| m.member == member))
            .map(|p| p as i64))
    }

    async fn zrevrank(&self, key: &str, member: &str) -> Result<Option<i64>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).and_then(|entry| {
            entry
                .iter()
                .position(|m| m.member == member)
                .map(|p| (entry.len() - 1 - p) as i64)
        }))
    }

    async fn zrange(
        &self,
        key: &str,
        start: i64,
        stop: i64,
        options: Option<ZRangeOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let entry = store.get(key).cloned().unwrap_or_default();
        let len = entry.len() as i64;

        let start = if start < 0 {
            (len + start).max(0)
        } else {
            start
        };
        let stop = if stop < 0 { len + stop } else { stop };
        let stop = (stop + 1).min(len) as usize;
        let start = start as usize;

        if start >= entry.len() || start >= stop {
            return Ok(Vec::new());
        }

        let mut result: Vec<ScoredMember> = entry[start..stop].to_vec();

        if let Some(opts) = options
            && opts.rev
        {
            result.reverse();
        }

        Ok(result)
    }

    async fn zrangebyscore(
        &self,
        key: &str,
        range: &ScoreRange,
        options: Option<ZRangeOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let entry = store.get(key).cloned().unwrap_or_default();

        let mut result: Vec<ScoredMember> = entry
            .into_iter()
            .filter(|m| {
                let min_ok = if range.min_exclusive {
                    m.score > range.min
                } else {
                    m.score >= range.min
                };
                let max_ok = if range.max_exclusive {
                    m.score < range.max
                } else {
                    m.score <= range.max
                };
                min_ok && max_ok
            })
            .collect();

        if let Some(opts) = options {
            if opts.rev {
                result.reverse();
            }
            if let (Some(offset), Some(count)) = (opts.offset, opts.count) {
                result = result
                    .into_iter()
                    .skip(offset as usize)
                    .take(count as usize)
                    .collect();
            }
        }

        Ok(result)
    }

    async fn zrangebylex(
        &self,
        key: &str,
        _range: &LexRange,
        _options: Option<ZRangeOptions>,
    ) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store
            .get(key)
            .map(|e| e.iter().map(|m| m.member.clone()).collect())
            .unwrap_or_default())
    }

    async fn zrangestore(
        &self,
        destination: &str,
        source: &str,
        start: i64,
        stop: i64,
        options: Option<ZRangeOptions>,
    ) -> Result<i64, CacheError> {
        let members = self.zrange(source, start, stop, options).await?;
        let count = members.len() as i64;
        let mut store = self.store.lock().expect("store lock");
        store.insert(destination.to_string(), members);
        Ok(count)
    }

    async fn zremrangebyrank(&self, key: &str, start: i64, stop: i64) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if let Some(entry) = store.get_mut(key) {
            let len = entry.len() as i64;
            let start = if start < 0 {
                (len + start).max(0)
            } else {
                start
            };
            let stop = if stop < 0 { len + stop } else { stop };
            let stop = (stop + 1).min(len) as usize;
            let start = start as usize;

            if start >= entry.len() || start >= stop {
                return Ok(0);
            }

            let removed = stop - start;
            entry.drain(start..stop);
            Ok(removed as i64)
        } else {
            Ok(0)
        }
    }

    async fn zremrangebyscore(&self, key: &str, range: &ScoreRange) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if let Some(entry) = store.get_mut(key) {
            let before = entry.len();
            entry.retain(|m| {
                let min_ok = if range.min_exclusive {
                    m.score <= range.min
                } else {
                    m.score < range.min
                };
                let max_ok = if range.max_exclusive {
                    m.score >= range.max
                } else {
                    m.score > range.max
                };
                min_ok || max_ok
            });
            Ok((before - entry.len()) as i64)
        } else {
            Ok(0)
        }
    }

    async fn zremrangebylex(&self, key: &str, _range: &LexRange) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).map_or(0, |_| 0))
    }

    async fn zpopmin(
        &self,
        key: &str,
        count: Option<i64>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if let Some(entry) = store.get_mut(key) {
            let count = count.unwrap_or(1) as usize;
            let count = count.min(entry.len());
            Ok(entry.drain(..count).collect())
        } else {
            Ok(Vec::new())
        }
    }

    async fn zpopmax(
        &self,
        key: &str,
        count: Option<i64>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if let Some(entry) = store.get_mut(key) {
            let count = count.unwrap_or(1) as usize;
            let count = count.min(entry.len());
            let start = entry.len() - count;
            Ok(entry.drain(start..).rev().collect())
        } else {
            Ok(Vec::new())
        }
    }

    async fn bzpopmin(
        &self,
        keys: &[String],
        _timeout: f64,
    ) -> Result<Option<ZPopResult>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        for key in keys {
            if let Some(entry) = store.get_mut(key)
                && !entry.is_empty()
            {
                let member = entry.remove(0);
                return Ok(Some(ZPopResult {
                    key: key.clone(),
                    members: vec![member],
                }));
            }
        }
        Ok(None)
    }

    async fn bzpopmax(
        &self,
        keys: &[String],
        _timeout: f64,
    ) -> Result<Option<ZPopResult>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        for key in keys {
            if let Some(entry) = store.get_mut(key)
                && !entry.is_empty()
            {
                let member = entry.pop().unwrap();
                return Ok(Some(ZPopResult {
                    key: key.clone(),
                    members: vec![member],
                }));
            }
        }
        Ok(None)
    }

    async fn zmpop(
        &self,
        keys: &[String],
        direction: ZPopDirection,
        count: Option<i64>,
    ) -> Result<Option<ZPopResult>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        for key in keys {
            if let Some(entry) = store.get_mut(key)
                && !entry.is_empty()
            {
                let count = count.unwrap_or(1) as usize;
                let count = count.min(entry.len());
                let members = match direction {
                    ZPopDirection::Min => entry.drain(..count).collect(),
                    ZPopDirection::Max => {
                        let start = entry.len() - count;
                        entry.drain(start..).rev().collect()
                    }
                };
                return Ok(Some(ZPopResult {
                    key: key.clone(),
                    members,
                }));
            }
        }
        Ok(None)
    }

    async fn bzmpop(
        &self,
        keys: &[String],
        direction: ZPopDirection,
        _timeout: f64,
        count: Option<i64>,
    ) -> Result<Option<ZPopResult>, CacheError> {
        self.zmpop(keys, direction, count).await
    }

    async fn zrandmember(
        &self,
        key: &str,
        count: Option<i64>,
        _with_scores: bool,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let store = self.store.lock().expect("store lock");
        if let Some(entry) = store.get(key) {
            let count = count.unwrap_or(1).unsigned_abs() as usize;
            Ok(entry.iter().take(count).cloned().collect())
        } else {
            Ok(Vec::new())
        }
    }

    async fn zunion(
        &self,
        keys: &[String],
        _options: Option<ZSetAlgebraOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let mut result: HashMap<String, f64> = HashMap::new();

        for key in keys {
            if let Some(entry) = store.get(key) {
                for member in entry {
                    *result.entry(member.member.clone()).or_insert(0.0) += member.score;
                }
            }
        }

        let mut members: Vec<ScoredMember> = result
            .into_iter()
            .map(|(member, score)| ScoredMember::new(member, score))
            .collect();
        Self::sort_members(&mut members);
        Ok(members)
    }

    async fn zunionstore(
        &self,
        destination: &str,
        keys: &[String],
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<i64, CacheError> {
        let members = self.zunion(keys, options).await?;
        let count = members.len() as i64;
        let mut store = self.store.lock().expect("store lock");
        store.insert(destination.to_string(), members);
        Ok(count)
    }

    async fn zinter(
        &self,
        keys: &[String],
        _options: Option<ZSetAlgebraOptions>,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let store = self.store.lock().expect("store lock");
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let first = store.get(&keys[0]).cloned().unwrap_or_default();
        let mut result: HashMap<String, f64> =
            first.into_iter().map(|m| (m.member, m.score)).collect();

        for key in &keys[1..] {
            if let Some(entry) = store.get(key) {
                let entry_map: HashMap<String, f64> =
                    entry.iter().map(|m| (m.member.clone(), m.score)).collect();
                result.retain(|m, s| {
                    if let Some(other_score) = entry_map.get(m) {
                        *s += *other_score;
                        true
                    } else {
                        false
                    }
                });
            } else {
                return Ok(Vec::new());
            }
        }

        let mut members: Vec<ScoredMember> = result
            .into_iter()
            .map(|(member, score)| ScoredMember::new(member, score))
            .collect();
        Self::sort_members(&mut members);
        Ok(members)
    }

    async fn zinterstore(
        &self,
        destination: &str,
        keys: &[String],
        options: Option<ZSetAlgebraOptions>,
    ) -> Result<i64, CacheError> {
        let members = self.zinter(keys, options).await?;
        let count = members.len() as i64;
        let mut store = self.store.lock().expect("store lock");
        store.insert(destination.to_string(), members);
        Ok(count)
    }

    async fn zintercard(&self, keys: &[String], limit: Option<u64>) -> Result<i64, CacheError> {
        let members = self.zinter(keys, None).await?;
        let count = members.len() as i64;
        Ok(limit.map_or(count, |l| count.min(l as i64)))
    }

    async fn zdiff(
        &self,
        keys: &[String],
        _with_scores: bool,
    ) -> Result<Vec<ScoredMember>, CacheError> {
        let store = self.store.lock().expect("store lock");
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let first = store.get(&keys[0]).cloned().unwrap_or_default();
        let mut other_members: std::collections::HashSet<String> = std::collections::HashSet::new();

        for key in &keys[1..] {
            if let Some(entry) = store.get(key) {
                for m in entry {
                    other_members.insert(m.member.clone());
                }
            }
        }

        let mut result: Vec<ScoredMember> = first
            .into_iter()
            .filter(|m| !other_members.contains(&m.member))
            .collect();
        Self::sort_members(&mut result);
        Ok(result)
    }

    async fn zdiffstore(&self, destination: &str, keys: &[String]) -> Result<i64, CacheError> {
        let members = self.zdiff(keys, false).await?;
        let count = members.len() as i64;
        let mut store = self.store.lock().expect("store lock");
        store.insert(destination.to_string(), members);
        Ok(count)
    }

    async fn zscan(
        &self,
        key: &str,
        _cursor: u64,
        _pattern: Option<&str>,
        _count: Option<u64>,
    ) -> Result<ZScanResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        let members = store.get(key).cloned().unwrap_or_default();
        Ok(ZScanResult { cursor: 0, members })
    }
}

/// Mock Stream Repository for testing
#[derive(Default)]
pub struct MockStreamRepository;

impl MockStreamRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
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

    async fn xtrim(&self, _key: &str, _options: XTrimOptions) -> Result<i64, CacheError> {
        Ok(5)
    }

    async fn xdelex(
        &self,
        _key: &str,
        _mode: crate::domain::entities::XAckDelMode,
        ids: &[String],
    ) -> Result<Vec<crate::domain::entities::XAckDelEntryResult>, CacheError> {
        Ok(ids
            .iter()
            .map(|id| crate::domain::entities::XAckDelEntryResult::new(id.clone(), 1))
            .collect())
    }

    async fn xcfgset(
        &self,
        _key: &str,
        _options: crate::domain::entities::XCfgSetOptions,
    ) -> Result<(), CacheError> {
        Ok(())
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

    async fn xack(&self, _key: &str, _group: &str, ids: &[String]) -> Result<i64, CacheError> {
        Ok(ids.len() as i64)
    }

    async fn xackdel(
        &self,
        _key: &str,
        _group: &str,
        _mode: crate::domain::entities::XAckDelMode,
        ids: &[String],
    ) -> Result<Vec<crate::domain::entities::XAckDelEntryResult>, CacheError> {
        // Mock returns "deleted" for every supplied id, in order.
        Ok(ids
            .iter()
            .map(|id| crate::domain::entities::XAckDelEntryResult::new(id.clone(), 1))
            .collect())
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

/// Mock JSON Repository for testing
#[derive(Default)]
pub struct MockJsonRepository;

impl MockJsonRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl JsonRepository for MockJsonRepository {
    async fn json_set(
        &self,
        key: &str,
        path: &str,
        _value: Value,
        _options: JsonSetOptions,
    ) -> Result<JsonSetResult, CacheError> {
        Ok(JsonSetResult {
            key: key.to_string(),
            path: path.to_string(),
            success: true,
        })
    }

    async fn json_get(
        &self,
        key: &str,
        paths: &[String],
    ) -> Result<Option<JsonGetResult>, CacheError> {
        Ok(Some(JsonGetResult {
            key: key.to_string(),
            paths: paths.to_vec(),
            value: Value::Null,
        }))
    }

    async fn json_mget(&self, keys: &[String], path: &str) -> Result<JsonMGetResult, CacheError> {
        Ok(JsonMGetResult {
            results: keys
                .iter()
                .map(|k| JsonMGetItem {
                    key: k.clone(),
                    value: Some(Value::Null),
                })
                .collect(),
            path: path.to_string(),
        })
    }

    async fn json_mset(&self, _items: &[JsonMSetItem]) -> Result<(), CacheError> {
        Ok(())
    }

    async fn json_del(&self, key: &str, path: &str) -> Result<JsonDelResult, CacheError> {
        Ok(JsonDelResult {
            key: key.to_string(),
            path: path.to_string(),
            deleted_count: 1,
        })
    }

    async fn json_type(&self, key: &str, path: &str) -> Result<JsonTypeResult, CacheError> {
        Ok(JsonTypeResult {
            key: key.to_string(),
            path: path.to_string(),
            types: vec![Some("object".to_string())],
        })
    }

    async fn json_str_len(&self, key: &str, path: &str) -> Result<JsonStrLenResult, CacheError> {
        Ok(JsonStrLenResult {
            key: key.to_string(),
            path: path.to_string(),
            lengths: vec![Some(5)],
        })
    }

    async fn json_str_append(
        &self,
        key: &str,
        path: &str,
        _value: &str,
    ) -> Result<JsonStrAppendResult, CacheError> {
        Ok(JsonStrAppendResult {
            key: key.to_string(),
            path: path.to_string(),
            new_lengths: vec![Some(10)],
        })
    }

    async fn json_num_incr_by(
        &self,
        key: &str,
        path: &str,
        _value: f64,
    ) -> Result<JsonNumResult, CacheError> {
        Ok(JsonNumResult {
            key: key.to_string(),
            path: path.to_string(),
            values: Value::Array(vec![Value::Number(serde_json::Number::from(1))]),
        })
    }

    async fn json_num_mult_by(
        &self,
        key: &str,
        path: &str,
        _value: f64,
    ) -> Result<JsonNumResult, CacheError> {
        Ok(JsonNumResult {
            key: key.to_string(),
            path: path.to_string(),
            values: Value::Array(vec![Value::Number(serde_json::Number::from(2))]),
        })
    }

    async fn json_toggle(&self, key: &str, path: &str) -> Result<JsonToggleResult, CacheError> {
        Ok(JsonToggleResult {
            key: key.to_string(),
            path: path.to_string(),
            values: vec![Some(true)],
        })
    }

    async fn json_clear(&self, key: &str, path: &str) -> Result<JsonClearResult, CacheError> {
        Ok(JsonClearResult {
            key: key.to_string(),
            path: path.to_string(),
            cleared_count: 1,
        })
    }

    async fn json_arr_len(&self, key: &str, path: &str) -> Result<JsonArrLenResult, CacheError> {
        Ok(JsonArrLenResult {
            key: key.to_string(),
            path: path.to_string(),
            lengths: vec![Some(3)],
        })
    }

    async fn json_arr_append(
        &self,
        key: &str,
        path: &str,
        _values: &[Value],
    ) -> Result<JsonArrAppendResult, CacheError> {
        Ok(JsonArrAppendResult {
            key: key.to_string(),
            path: path.to_string(),
            new_lengths: vec![Some(5)],
        })
    }

    async fn json_arr_index(
        &self,
        key: &str,
        path: &str,
        _value: &Value,
        _start: Option<i64>,
        _stop: Option<i64>,
    ) -> Result<JsonArrIndexResult, CacheError> {
        Ok(JsonArrIndexResult {
            key: key.to_string(),
            path: path.to_string(),
            indices: vec![Some(0)],
        })
    }

    async fn json_arr_insert(
        &self,
        key: &str,
        path: &str,
        _index: i64,
        _values: &[Value],
    ) -> Result<JsonArrInsertResult, CacheError> {
        Ok(JsonArrInsertResult {
            key: key.to_string(),
            path: path.to_string(),
            new_lengths: vec![Some(5)],
        })
    }

    async fn json_arr_pop(
        &self,
        key: &str,
        path: &str,
        _index: Option<i64>,
    ) -> Result<JsonArrPopResult, CacheError> {
        Ok(JsonArrPopResult {
            key: key.to_string(),
            path: path.to_string(),
            values: vec![Some(Value::String("popped".to_string()))],
        })
    }

    async fn json_arr_trim(
        &self,
        key: &str,
        path: &str,
        _start: i64,
        _stop: i64,
    ) -> Result<JsonArrTrimResult, CacheError> {
        Ok(JsonArrTrimResult {
            key: key.to_string(),
            path: path.to_string(),
            new_lengths: vec![Some(2)],
        })
    }

    async fn json_obj_len(&self, key: &str, path: &str) -> Result<JsonObjLenResult, CacheError> {
        Ok(JsonObjLenResult {
            key: key.to_string(),
            path: path.to_string(),
            lengths: vec![Some(3)],
        })
    }

    async fn json_obj_keys(&self, key: &str, path: &str) -> Result<JsonObjKeysResult, CacheError> {
        Ok(JsonObjKeysResult {
            key: key.to_string(),
            path: path.to_string(),
            keys: vec![Some(vec!["a".to_string(), "b".to_string()])],
        })
    }

    async fn json_debug_memory(
        &self,
        key: &str,
        path: &str,
    ) -> Result<JsonDebugMemoryResult, CacheError> {
        Ok(JsonDebugMemoryResult {
            key: key.to_string(),
            path: path.to_string(),
            memory_bytes: vec![Some(128)],
        })
    }

    async fn json_resp(&self, key: &str, path: &str) -> Result<JsonRespResult, CacheError> {
        Ok(JsonRespResult {
            key: key.to_string(),
            path: path.to_string(),
            resp: Value::Array(vec![Value::String("{".to_string())]),
        })
    }
}

/// Mock Search Repository for testing
#[derive(Default)]
pub struct MockSearchRepository;

impl MockSearchRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SearchRepository for MockSearchRepository {
    async fn ft_create(
        &self,
        index: &str,
        _options: &IndexCreateOptions,
        _schema: &[SearchFieldSchema],
    ) -> Result<IndexCreateResult, CacheError> {
        Ok(IndexCreateResult {
            index: index.to_string(),
            success: true,
        })
    }

    async fn ft_drop_index(
        &self,
        index: &str,
        delete_docs: bool,
    ) -> Result<IndexDropResult, CacheError> {
        Ok(IndexDropResult {
            index: index.to_string(),
            delete_docs,
            success: true,
        })
    }

    async fn ft_info(&self, index: &str) -> Result<IndexInfo, CacheError> {
        Ok(IndexInfo {
            index_name: index.to_string(),
            index_options: vec![],
            index_definition: std::collections::HashMap::new(),
            attributes: vec![],
            num_docs: 0,
            max_doc_id: None,
            num_terms: 0,
            num_records: None,
            inverted_sz_mb: None,
            vector_index_sz_mb: None,
            total_inverted_index_blocks: None,
            offset_vectors_sz_mb: None,
            doc_table_size_mb: None,
            sortable_values_size_mb: None,
            key_table_size_mb: None,
            records_per_doc_avg: None,
            bytes_per_record_avg: None,
            offsets_per_term_avg: None,
            offset_bits_per_record_avg: None,
            indexing: false,
            percent_indexed: Some(100.0),
            hash_indexing_failures: None,
            gc_stats: std::collections::HashMap::new(),
            cursor_stats: std::collections::HashMap::new(),
        })
    }

    async fn ft_list(&self) -> Result<Vec<String>, CacheError> {
        Ok(vec!["test_index".to_string()])
    }

    async fn ft_alter(
        &self,
        index: &str,
        field: &SearchFieldSchema,
    ) -> Result<IndexAlterResult, CacheError> {
        Ok(IndexAlterResult {
            index: index.to_string(),
            field: field.name.clone(),
            success: true,
        })
    }

    async fn ft_search(
        &self,
        _index: &str,
        _query: &str,
        _options: &SearchOptions,
    ) -> Result<SearchResult, CacheError> {
        Ok(SearchResult {
            total_results: 0,
            documents: vec![],
        })
    }

    async fn ft_aggregate(
        &self,
        _index: &str,
        _query: &str,
        _options: &AggregateOptions,
    ) -> Result<AggregateResult, CacheError> {
        Ok(AggregateResult {
            total_results: 0,
            rows: vec![],
            cursor_id: None,
        })
    }

    async fn ft_explain(
        &self,
        _index: &str,
        _query: &str,
        _dialect: Option<u32>,
    ) -> Result<ExplainResult, CacheError> {
        Ok(ExplainResult {
            plan: "INTERSECT".to_string(),
        })
    }

    async fn ft_profile(
        &self,
        _index: &str,
        _profile_type: ProfileType,
        _limited: bool,
        _query: &str,
        _search_options: Option<&SearchOptions>,
        _aggregate_options: Option<&AggregateOptions>,
    ) -> Result<ProfileResult, CacheError> {
        Ok(ProfileResult {
            results: serde_json::Value::Null,
            profile: std::collections::HashMap::new(),
        })
    }

    async fn ft_hybrid(
        &self,
        _index: &str,
        _options: &crate::domain::entities::HybridSearchOptions,
    ) -> Result<crate::domain::entities::HybridSearchResult, CacheError> {
        Ok(crate::domain::entities::HybridSearchResult {
            total_results: 0,
            documents: vec![],
        })
    }

    async fn ft_aliasadd(&self, alias: &str, index: &str) -> Result<AliasResult, CacheError> {
        Ok(AliasResult {
            alias: alias.to_string(),
            index: index.to_string(),
            success: true,
        })
    }

    async fn ft_aliasdel(&self, alias: &str) -> Result<AliasResult, CacheError> {
        Ok(AliasResult {
            alias: alias.to_string(),
            index: String::new(),
            success: true,
        })
    }

    async fn ft_aliasupdate(&self, alias: &str, index: &str) -> Result<AliasResult, CacheError> {
        Ok(AliasResult {
            alias: alias.to_string(),
            index: index.to_string(),
            success: true,
        })
    }

    async fn ft_sugadd(
        &self,
        key: &str,
        _string: &str,
        _score: f64,
        _options: &SugAddOptions,
    ) -> Result<SugAddResult, CacheError> {
        Ok(SugAddResult {
            key: key.to_string(),
            size: 1,
        })
    }

    async fn ft_sugget(
        &self,
        _key: &str,
        _prefix: &str,
        _options: &SugGetOptions,
    ) -> Result<Vec<Suggestion>, CacheError> {
        Ok(vec![])
    }

    async fn ft_sugdel(&self, key: &str, _string: &str) -> Result<SugDelResult, CacheError> {
        Ok(SugDelResult {
            key: key.to_string(),
            deleted: true,
        })
    }

    async fn ft_suglen(&self, key: &str) -> Result<SugLenResult, CacheError> {
        Ok(SugLenResult {
            key: key.to_string(),
            size: 0,
        })
    }

    async fn ft_syndump(&self, _index: &str) -> Result<Vec<SynonymGroup>, CacheError> {
        Ok(vec![])
    }

    async fn ft_synupdate(
        &self,
        index: &str,
        group_id: &str,
        _skip_initial_scan: bool,
        _terms: &[String],
    ) -> Result<SynonymUpdateResult, CacheError> {
        Ok(SynonymUpdateResult {
            index: index.to_string(),
            group_id: group_id.to_string(),
            success: true,
        })
    }

    async fn ft_spellcheck(
        &self,
        _index: &str,
        _query: &str,
        _options: &SpellcheckOptions,
    ) -> Result<SpellcheckResult, CacheError> {
        Ok(SpellcheckResult { results: vec![] })
    }

    async fn ft_dictadd(&self, dict: &str, terms: &[String]) -> Result<DictResult, CacheError> {
        Ok(DictResult {
            dict: dict.to_string(),
            count: terms.len() as i64,
        })
    }

    async fn ft_dictdel(&self, dict: &str, terms: &[String]) -> Result<DictResult, CacheError> {
        Ok(DictResult {
            dict: dict.to_string(),
            count: terms.len() as i64,
        })
    }

    async fn ft_dictdump(&self, dict: &str) -> Result<DictDumpResult, CacheError> {
        Ok(DictDumpResult {
            dict: dict.to_string(),
            terms: vec![],
        })
    }

    async fn ft_config_get(
        &self,
        _option: &str,
    ) -> Result<std::collections::HashMap<String, String>, CacheError> {
        Ok(std::collections::HashMap::new())
    }

    async fn ft_config_set(&self, _option: &str, _value: &str) -> Result<bool, CacheError> {
        Ok(true)
    }

    async fn ft_cursor_read(
        &self,
        _index: &str,
        _cursor: u64,
        _count: Option<u64>,
    ) -> Result<crate::domain::entities::CursorReadResult, CacheError> {
        Ok(crate::domain::entities::CursorReadResult {
            cursor_id: 0,
            rows: vec![],
        })
    }

    async fn ft_cursor_del(&self, _index: &str, _cursor: u64) -> Result<bool, CacheError> {
        Ok(true)
    }
}

/// Mock Bloom Repository for testing
pub struct MockBloomRepository;

impl MockBloomRepository {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockBloomRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BloomRepository for MockBloomRepository {
    async fn bf_reserve(
        &self,
        key: &str,
        _options: BloomReserveOptions,
    ) -> Result<BloomReserveResult, CacheError> {
        Ok(BloomReserveResult {
            key: key.to_string(),
            success: true,
        })
    }

    async fn bf_add(&self, key: &str, _item: &str) -> Result<BloomAddResult, CacheError> {
        Ok(BloomAddResult {
            key: key.to_string(),
            results: vec![true],
        })
    }

    async fn bf_madd(&self, key: &str, items: Vec<String>) -> Result<BloomAddResult, CacheError> {
        Ok(BloomAddResult {
            key: key.to_string(),
            results: vec![true; items.len()],
        })
    }

    async fn bf_exists(&self, key: &str, _item: &str) -> Result<BloomExistsResult, CacheError> {
        Ok(BloomExistsResult {
            key: key.to_string(),
            results: vec![true],
        })
    }

    async fn bf_mexists(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<BloomExistsResult, CacheError> {
        Ok(BloomExistsResult {
            key: key.to_string(),
            results: vec![true; items.len()],
        })
    }

    async fn bf_insert(
        &self,
        key: &str,
        _options: BloomInsertOptions,
        items: Vec<String>,
    ) -> Result<BloomInsertResult, CacheError> {
        Ok(BloomInsertResult {
            key: key.to_string(),
            results: vec![true; items.len()],
        })
    }

    async fn bf_info(&self, _key: &str) -> Result<BloomInfo, CacheError> {
        Ok(BloomInfo {
            num_filters: 1,
            num_items_inserted: 100,
            capacity: 1000,
            size: 2048,
            expansion: Some(2),
        })
    }

    async fn bf_card(&self, key: &str) -> Result<BloomCardResult, CacheError> {
        Ok(BloomCardResult {
            key: key.to_string(),
            cardinality: 100,
        })
    }

    async fn bf_scandump(
        &self,
        _key: &str,
        iterator: u64,
    ) -> Result<BloomScanDumpResult, CacheError> {
        if iterator == 0 {
            Ok(BloomScanDumpResult {
                iterator: 1,
                data: Some("dGVzdA==".to_string()),
            })
        } else {
            Ok(BloomScanDumpResult {
                iterator: 0,
                data: None,
            })
        }
    }

    async fn bf_loadchunk(
        &self,
        key: &str,
        _iterator: u64,
        _data: &[u8],
    ) -> Result<BloomLoadChunkResult, CacheError> {
        Ok(BloomLoadChunkResult {
            key: key.to_string(),
            success: true,
        })
    }

    async fn cf_reserve(
        &self,
        key: &str,
        _options: CuckooReserveOptions,
    ) -> Result<CuckooReserveResult, CacheError> {
        Ok(CuckooReserveResult {
            key: key.to_string(),
            success: true,
        })
    }

    async fn cf_add(&self, key: &str, _item: &str) -> Result<CuckooAddResult, CacheError> {
        Ok(CuckooAddResult {
            key: key.to_string(),
            added: true,
        })
    }

    async fn cf_addnx(&self, key: &str, _item: &str) -> Result<CuckooAddResult, CacheError> {
        Ok(CuckooAddResult {
            key: key.to_string(),
            added: true,
        })
    }

    async fn cf_insert(
        &self,
        key: &str,
        _options: CuckooInsertOptions,
        items: Vec<String>,
    ) -> Result<CuckooInsertResult, CacheError> {
        Ok(CuckooInsertResult {
            key: key.to_string(),
            results: vec![true; items.len()],
        })
    }

    async fn cf_insertnx(
        &self,
        key: &str,
        _options: CuckooInsertOptions,
        items: Vec<String>,
    ) -> Result<CuckooInsertResult, CacheError> {
        Ok(CuckooInsertResult {
            key: key.to_string(),
            results: vec![true; items.len()],
        })
    }

    async fn cf_exists(&self, key: &str, _item: &str) -> Result<CuckooExistsResult, CacheError> {
        Ok(CuckooExistsResult {
            key: key.to_string(),
            results: vec![true],
        })
    }

    async fn cf_mexists(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<CuckooExistsResult, CacheError> {
        Ok(CuckooExistsResult {
            key: key.to_string(),
            results: vec![true; items.len()],
        })
    }

    async fn cf_del(&self, key: &str, _item: &str) -> Result<CuckooDelResult, CacheError> {
        Ok(CuckooDelResult {
            key: key.to_string(),
            deleted: true,
        })
    }

    async fn cf_count(&self, key: &str, _item: &str) -> Result<CuckooCountResult, CacheError> {
        Ok(CuckooCountResult {
            key: key.to_string(),
            count: 1,
        })
    }

    async fn cf_info(&self, _key: &str) -> Result<CuckooInfo, CacheError> {
        Ok(CuckooInfo {
            size: 4096,
            num_buckets: 512,
            num_filters: 1,
            num_items_inserted: 100,
            num_items_deleted: 5,
            bucket_size: 2,
            expansion_rate: 1,
            max_iterations: 20,
        })
    }

    async fn cf_scandump(
        &self,
        _key: &str,
        iterator: u64,
    ) -> Result<CuckooScanDumpResult, CacheError> {
        if iterator == 0 {
            Ok(CuckooScanDumpResult {
                iterator: 1,
                data: Some("dGVzdA==".to_string()),
            })
        } else {
            Ok(CuckooScanDumpResult {
                iterator: 0,
                data: None,
            })
        }
    }

    async fn cf_loadchunk(
        &self,
        key: &str,
        _iterator: u64,
        _data: &[u8],
    ) -> Result<CuckooLoadChunkResult, CacheError> {
        Ok(CuckooLoadChunkResult {
            key: key.to_string(),
            success: true,
        })
    }
}

/// Mock Probabilistic Repository for testing
pub struct MockProbabilisticRepository;

impl MockProbabilisticRepository {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockProbabilisticRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProbabilisticRepository for MockProbabilisticRepository {
    // Count-Min Sketch operations
    async fn cms_init_by_dim(
        &self,
        key: &str,
        _width: u64,
        _depth: u64,
    ) -> Result<CmsInitResult, CacheError> {
        Ok(CmsInitResult {
            key: key.to_string(),
            success: true,
        })
    }

    async fn cms_init_by_prob(
        &self,
        key: &str,
        _error: f64,
        _probability: f64,
    ) -> Result<CmsInitResult, CacheError> {
        Ok(CmsInitResult {
            key: key.to_string(),
            success: true,
        })
    }

    async fn cms_incr_by(
        &self,
        key: &str,
        items: Vec<(String, u64)>,
    ) -> Result<CmsIncrByResult, CacheError> {
        Ok(CmsIncrByResult {
            key: key.to_string(),
            counts: items.iter().map(|(_, count)| *count).collect(),
        })
    }

    async fn cms_query(&self, key: &str, items: Vec<String>) -> Result<CmsQueryResult, CacheError> {
        Ok(CmsQueryResult {
            key: key.to_string(),
            counts: vec![1; items.len()],
        })
    }

    async fn cms_merge(
        &self,
        dest: &str,
        _sources: Vec<String>,
        _weights: Option<Vec<u64>>,
    ) -> Result<CmsMergeResult, CacheError> {
        Ok(CmsMergeResult {
            key: dest.to_string(),
            success: true,
        })
    }

    async fn cms_info(&self, _key: &str) -> Result<CmsInfo, CacheError> {
        Ok(CmsInfo {
            width: 2000,
            depth: 5,
            count: 100,
        })
    }

    // Top-K operations
    async fn topk_reserve(
        &self,
        key: &str,
        _k: u64,
        _width: Option<u64>,
        _depth: Option<u64>,
        _decay: Option<f64>,
    ) -> Result<TopKReserveResult, CacheError> {
        Ok(TopKReserveResult {
            key: key.to_string(),
            success: true,
        })
    }

    async fn topk_add(&self, key: &str, items: Vec<String>) -> Result<TopKAddResult, CacheError> {
        Ok(TopKAddResult {
            key: key.to_string(),
            dropped: vec![None; items.len()],
        })
    }

    async fn topk_incr_by(
        &self,
        key: &str,
        items: Vec<(String, u64)>,
    ) -> Result<TopKIncrByResult, CacheError> {
        Ok(TopKIncrByResult {
            key: key.to_string(),
            dropped: vec![None; items.len()],
        })
    }

    async fn topk_query(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<TopKQueryResult, CacheError> {
        Ok(TopKQueryResult {
            key: key.to_string(),
            results: vec![true; items.len()],
        })
    }

    async fn topk_count(
        &self,
        key: &str,
        items: Vec<String>,
    ) -> Result<TopKCountResult, CacheError> {
        Ok(TopKCountResult {
            key: key.to_string(),
            counts: vec![10; items.len()],
        })
    }

    async fn topk_list(&self, key: &str, with_count: bool) -> Result<TopKListResult, CacheError> {
        Ok(TopKListResult {
            key: key.to_string(),
            items: vec![
                TopKItem {
                    item: "item1".to_string(),
                    count: if with_count { 100 } else { 0 },
                },
                TopKItem {
                    item: "item2".to_string(),
                    count: if with_count { 50 } else { 0 },
                },
            ],
        })
    }

    async fn topk_info(&self, _key: &str) -> Result<TopKInfo, CacheError> {
        Ok(TopKInfo {
            k: 10,
            width: 2000,
            depth: 7,
            decay: 0.9,
        })
    }

    // HyperLogLog operations
    async fn pf_add(&self, key: &str, _elements: Vec<String>) -> Result<PfAddResult, CacheError> {
        Ok(PfAddResult {
            key: key.to_string(),
            changed: true,
        })
    }

    async fn pf_count(&self, keys: Vec<String>) -> Result<PfCountResult, CacheError> {
        Ok(PfCountResult { keys, count: 1000 })
    }

    async fn pf_merge(
        &self,
        dest: &str,
        _sources: Vec<String>,
    ) -> Result<PfMergeResult, CacheError> {
        Ok(PfMergeResult {
            dest_key: dest.to_string(),
            success: true,
        })
    }

    // T-Digest operations
    async fn tdigest_create(
        &self,
        key: &str,
        _compression: Option<u64>,
    ) -> Result<TDigestAckResult, CacheError> {
        Ok(TDigestAckResult {
            key: key.to_string(),
            success: true,
        })
    }

    async fn tdigest_add(
        &self,
        key: &str,
        _values: Vec<f64>,
    ) -> Result<TDigestAckResult, CacheError> {
        Ok(TDigestAckResult {
            key: key.to_string(),
            success: true,
        })
    }

    async fn tdigest_quantile(
        &self,
        key: &str,
        quantiles: Vec<f64>,
    ) -> Result<TDigestValuesResult, CacheError> {
        Ok(TDigestValuesResult {
            key: key.to_string(),
            values: quantiles.into_iter().map(Some).collect(),
        })
    }

    async fn tdigest_cdf(
        &self,
        key: &str,
        values: Vec<f64>,
    ) -> Result<TDigestValuesResult, CacheError> {
        Ok(TDigestValuesResult {
            key: key.to_string(),
            values: vec![Some(0.5); values.len()],
        })
    }

    async fn tdigest_rank(
        &self,
        key: &str,
        values: Vec<f64>,
    ) -> Result<TDigestRanksResult, CacheError> {
        Ok(TDigestRanksResult {
            key: key.to_string(),
            ranks: vec![1; values.len()],
        })
    }

    async fn tdigest_revrank(
        &self,
        key: &str,
        values: Vec<f64>,
    ) -> Result<TDigestRanksResult, CacheError> {
        Ok(TDigestRanksResult {
            key: key.to_string(),
            ranks: vec![1; values.len()],
        })
    }

    async fn tdigest_byrank(
        &self,
        key: &str,
        ranks: Vec<u64>,
    ) -> Result<TDigestValuesResult, CacheError> {
        Ok(TDigestValuesResult {
            key: key.to_string(),
            values: vec![Some(10.0); ranks.len()],
        })
    }

    async fn tdigest_byrevrank(
        &self,
        key: &str,
        ranks: Vec<u64>,
    ) -> Result<TDigestValuesResult, CacheError> {
        Ok(TDigestValuesResult {
            key: key.to_string(),
            values: vec![Some(90.0); ranks.len()],
        })
    }

    async fn tdigest_min(&self, key: &str) -> Result<TDigestScalarResult, CacheError> {
        Ok(TDigestScalarResult {
            key: key.to_string(),
            value: Some(1.0),
        })
    }

    async fn tdigest_max(&self, key: &str) -> Result<TDigestScalarResult, CacheError> {
        Ok(TDigestScalarResult {
            key: key.to_string(),
            value: Some(99.0),
        })
    }

    async fn tdigest_info(&self, _key: &str) -> Result<TDigestInfo, CacheError> {
        Ok(TDigestInfo {
            compression: 100,
            capacity: 610,
            merged_nodes: 5,
            unmerged_nodes: 2,
            merged_weight: 100.0,
            unmerged_weight: 3.0,
            observations: 103,
            total_compressions: 1,
            memory_usage: 2048,
        })
    }

    async fn tdigest_merge(
        &self,
        dest: &str,
        _sources: Vec<String>,
        _compression: Option<u64>,
        _override_existing: bool,
    ) -> Result<TDigestAckResult, CacheError> {
        Ok(TDigestAckResult {
            key: dest.to_string(),
            success: true,
        })
    }

    async fn tdigest_reset(&self, key: &str) -> Result<TDigestAckResult, CacheError> {
        Ok(TDigestAckResult {
            key: key.to_string(),
            success: true,
        })
    }

    async fn tdigest_trimmed_mean(
        &self,
        key: &str,
        _low: f64,
        _high: f64,
    ) -> Result<TDigestScalarResult, CacheError> {
        Ok(TDigestScalarResult {
            key: key.to_string(),
            value: Some(50.0),
        })
    }
}

// ========== MockBitMapRepository ==========

#[derive(Default)]
pub struct MockBitMapRepository {
    // Store bitmaps as HashMap<key, Vec<u8>> - each byte represents 8 bits
    store: Mutex<HashMap<String, Vec<u8>>>,
}

impl MockBitMapRepository {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    fn get_bit(&self, bytes: &[u8], offset: u64) -> i64 {
        let byte_index = (offset / 8) as usize;
        let bit_index = 7 - (offset % 8) as u8;
        if byte_index < bytes.len() {
            ((bytes[byte_index] >> bit_index) & 1) as i64
        } else {
            0
        }
    }

    fn set_bit(&self, bytes: &mut Vec<u8>, offset: u64, value: bool) -> i64 {
        let byte_index = (offset / 8) as usize;
        let bit_index = 7 - (offset % 8) as u8;

        // Extend if needed
        while bytes.len() <= byte_index {
            bytes.push(0);
        }

        let old_bit = ((bytes[byte_index] >> bit_index) & 1) as i64;
        if value {
            bytes[byte_index] |= 1 << bit_index;
        } else {
            bytes[byte_index] &= !(1 << bit_index);
        }
        old_bit
    }
}

#[async_trait]
impl BitMapRepository for MockBitMapRepository {
    async fn setbit(&self, key: &str, offset: u64, value: bool) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let bytes = store.entry(key.to_string()).or_default();
        let old_value = self.set_bit(bytes, offset, value);
        Ok(old_value)
    }

    async fn getbit(&self, key: &str, offset: u64) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        let bytes = store.get(key).map(|v| v.as_slice()).unwrap_or(&[]);
        Ok(self.get_bit(bytes, offset))
    }

    async fn bitcount(
        &self,
        key: &str,
        start: Option<i64>,
        end: Option<i64>,
        _use_bit_index: bool,
    ) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        let bytes = store.get(key).map(|v| v.as_slice()).unwrap_or(&[]);

        if bytes.is_empty() {
            return Ok(0);
        }

        let (start_idx, end_idx) = match (start, end) {
            (Some(s), Some(e)) => {
                let len = bytes.len() as i64;
                let s = if s < 0 { (len + s).max(0) } else { s.min(len) } as usize;
                let e = if e < 0 {
                    (len + e).max(0)
                } else {
                    e.min(len.saturating_sub(1))
                } as usize;
                (s, e)
            }
            _ => (0, bytes.len().saturating_sub(1)),
        };

        if start_idx > end_idx {
            return Ok(0);
        }

        let count: i64 = bytes
            .iter()
            .skip(start_idx)
            .take(end_idx - start_idx + 1)
            .map(|b| b.count_ones() as i64)
            .sum();

        Ok(count)
    }

    async fn bitpos(
        &self,
        key: &str,
        bit: bool,
        start: Option<i64>,
        _end: Option<i64>,
        _use_bit_index: bool,
    ) -> Result<i64, CacheError> {
        let store = self.store.lock().expect("store lock");
        let bytes = store.get(key).map(|v| v.as_slice()).unwrap_or(&[]);

        if bytes.is_empty() {
            return Ok(if bit { -1 } else { 0 });
        }

        let start_byte = start.unwrap_or(0).max(0) as usize;
        let target = if bit { 1u8 } else { 0u8 };

        for (byte_idx, &byte) in bytes.iter().enumerate().skip(start_byte) {
            for bit_idx in 0..8 {
                let actual_bit = (byte >> (7 - bit_idx)) & 1;
                if actual_bit == target {
                    return Ok((byte_idx * 8 + bit_idx) as i64);
                }
            }
        }

        Ok(-1)
    }

    async fn bitop(
        &self,
        operation: BitOperation,
        dest_key: &str,
        keys: &[String],
    ) -> Result<i64, CacheError> {
        let mut store = self.store.lock().expect("store lock");

        // Get all source bitmaps
        let sources: Vec<Vec<u8>> = keys
            .iter()
            .map(|k| store.get(k).cloned().unwrap_or_default())
            .collect();

        if sources.is_empty() {
            return Ok(0);
        }

        let max_len = sources.iter().map(|s| s.len()).max().unwrap_or(0);
        let mut result = vec![0u8; max_len];

        match operation {
            BitOperation::And => {
                for i in 0..max_len {
                    result[i] = sources
                        .iter()
                        .map(|s| *s.get(i).unwrap_or(&0xff))
                        .fold(0xff, |acc, b| acc & b);
                }
            }
            BitOperation::Or => {
                for i in 0..max_len {
                    result[i] = sources
                        .iter()
                        .map(|s| *s.get(i).unwrap_or(&0))
                        .fold(0, |acc, b| acc | b);
                }
            }
            BitOperation::Xor => {
                for i in 0..max_len {
                    result[i] = sources
                        .iter()
                        .map(|s| *s.get(i).unwrap_or(&0))
                        .fold(0, |acc, b| acc ^ b);
                }
            }
            BitOperation::Not => {
                if let Some(source) = sources.first() {
                    for (i, &b) in source.iter().enumerate() {
                        result[i] = !b;
                    }
                }
            }
        }

        let len = result.len() as i64;
        store.insert(dest_key.to_string(), result);
        Ok(len)
    }

    async fn bitfield(
        &self,
        key: &str,
        commands: &[BitfieldCommand],
    ) -> Result<BitfieldResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let bytes = store.entry(key.to_string()).or_default();

        let mut results = Vec::new();

        for cmd in commands {
            match cmd {
                BitfieldCommand::Get { .. } => {
                    // Simplified: return 0 for GET operations
                    results.push(Some(0));
                }
                BitfieldCommand::Set { value, .. } => {
                    // Simplified: return previous value (0) and store the new value
                    results.push(Some(0));
                    // Extend bytes if needed
                    while bytes.len() < 8 {
                        bytes.push(0);
                    }
                    // Store first byte of value
                    bytes[0] = *value as u8;
                }
                BitfieldCommand::IncrBy { increment, .. } => {
                    results.push(Some(*increment));
                }
                BitfieldCommand::Overflow(_) => {
                    // OVERFLOW doesn't return a value
                }
            }
        }

        Ok(BitfieldResult { values: results })
    }

    async fn bitfield_ro(
        &self,
        _key: &str,
        commands: &[BitfieldCommand],
    ) -> Result<BitfieldResult, CacheError> {
        let results: Vec<Option<i64>> = commands
            .iter()
            .filter_map(|cmd| {
                if let BitfieldCommand::Get { .. } = cmd {
                    Some(Some(0))
                } else {
                    None
                }
            })
            .collect();

        Ok(BitfieldResult { values: results })
    }
}

// ========== MockGeoRepository ==========

use crate::domain::repositories::{
    GeoAddOptions, GeoAddResult, GeoMember, GeoPosition, GeoRepository, GeoSearchCenter,
    GeoSearchOptions, GeoSearchResult, GeoSearchShape, GeoSearchStoreResult, GeoUnit,
};

#[derive(Default)]
pub struct MockGeoRepository {
    store: Mutex<HashMap<String, Vec<GeoMember>>>,
}

impl MockGeoRepository {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    /// Calculate distance between two points using Haversine formula
    fn haversine_distance(pos1: &GeoPosition, pos2: &GeoPosition) -> f64 {
        let r = 6371000.0; // Earth's radius in meters
        let lat1 = pos1.latitude.to_radians();
        let lat2 = pos2.latitude.to_radians();
        let delta_lat = (pos2.latitude - pos1.latitude).to_radians();
        let delta_lon = (pos2.longitude - pos1.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();
        r * c
    }

    /// Convert distance to specified unit
    fn convert_distance(meters: f64, unit: &GeoUnit) -> f64 {
        match unit {
            GeoUnit::Meters => meters,
            GeoUnit::Kilometers => meters / 1000.0,
            GeoUnit::Miles => meters / 1609.344,
            GeoUnit::Feet => meters * 3.28084,
        }
    }
}

#[async_trait]
impl GeoRepository for MockGeoRepository {
    async fn geo_add(
        &self,
        key: &str,
        members: &[GeoMember],
        _options: GeoAddOptions,
    ) -> Result<GeoAddResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        let entry = store.entry(key.to_string()).or_default();
        let mut added = 0i64;

        for member in members {
            // Check if member already exists
            let existing = entry.iter().position(|m| m.member == member.member);
            if let Some(idx) = existing {
                // Update existing member's position
                entry[idx] = member.clone();
            } else {
                // Add new member
                entry.push(member.clone());
                added += 1;
            }
        }

        Ok(GeoAddResult {
            added,
            changed: None,
        })
    }

    async fn geo_pos(
        &self,
        key: &str,
        members: &[String],
    ) -> Result<Vec<Option<GeoPosition>>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let geo_members = store.get(key);

        let positions: Vec<Option<GeoPosition>> = members
            .iter()
            .map(|name| {
                geo_members.and_then(|gm| gm.iter().find(|m| &m.member == name).map(|m| m.position))
            })
            .collect();

        Ok(positions)
    }

    async fn geo_dist(
        &self,
        key: &str,
        member1: &str,
        member2: &str,
        unit: GeoUnit,
    ) -> Result<Option<f64>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let geo_members = match store.get(key) {
            Some(m) => m,
            None => return Ok(None),
        };

        let pos1 = geo_members.iter().find(|m| m.member == member1);
        let pos2 = geo_members.iter().find(|m| m.member == member2);

        match (pos1, pos2) {
            (Some(m1), Some(m2)) => {
                let meters = Self::haversine_distance(&m1.position, &m2.position);
                Ok(Some(Self::convert_distance(meters, &unit)))
            }
            _ => Ok(None),
        }
    }

    async fn geo_hash(
        &self,
        key: &str,
        members: &[String],
    ) -> Result<Vec<Option<String>>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let geo_members = store.get(key);

        let hashes: Vec<Option<String>> = members
            .iter()
            .map(|name| {
                geo_members.and_then(|gm| {
                    gm.iter()
                        .find(|m| &m.member == name)
                        .map(|_| "sn1h35gkzb0".to_string()) // Simplified mock geohash
                })
            })
            .collect();

        Ok(hashes)
    }

    async fn geo_search(
        &self,
        key: &str,
        center: GeoSearchCenter,
        shape: GeoSearchShape,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let geo_members = match store.get(key) {
            Some(m) => m,
            None => return Ok(Vec::new()),
        };

        // Get center position
        let center_pos = match &center {
            GeoSearchCenter::FromLonLat(pos) => *pos,
            GeoSearchCenter::FromMember(member_name) => {
                match geo_members.iter().find(|m| &m.member == member_name) {
                    Some(m) => m.position,
                    None => return Ok(Vec::new()),
                }
            }
        };

        // Filter members based on shape
        let radius_meters = match &shape {
            GeoSearchShape::ByRadius { radius, unit } => match unit {
                GeoUnit::Meters => *radius,
                GeoUnit::Kilometers => radius * 1000.0,
                GeoUnit::Miles => radius * 1609.344,
                GeoUnit::Feet => radius / 3.28084,
            },
            GeoSearchShape::ByBox {
                width,
                height,
                unit,
            } => {
                // Approximate: use half the diagonal as radius
                let w = match unit {
                    GeoUnit::Meters => *width,
                    GeoUnit::Kilometers => width * 1000.0,
                    GeoUnit::Miles => width * 1609.344,
                    GeoUnit::Feet => width / 3.28084,
                };
                let h = match unit {
                    GeoUnit::Meters => *height,
                    GeoUnit::Kilometers => height * 1000.0,
                    GeoUnit::Miles => height * 1609.344,
                    GeoUnit::Feet => height / 3.28084,
                };
                ((w / 2.0).powi(2) + (h / 2.0).powi(2)).sqrt()
            }
        };

        let mut results: Vec<GeoSearchResult> = geo_members
            .iter()
            .filter_map(|m| {
                let dist = Self::haversine_distance(&center_pos, &m.position);
                if dist <= radius_meters {
                    Some(GeoSearchResult {
                        member: m.member.clone(),
                        distance: if options.with_dist { Some(dist) } else { None },
                        position: if options.with_coord {
                            Some(m.position)
                        } else {
                            None
                        },
                        geohash: if options.with_hash {
                            Some(0) // Simplified mock
                        } else {
                            None
                        },
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by distance if requested
        if let Some(sort) = &options.sort {
            use crate::domain::repositories::GeoSortOrder;
            match sort {
                GeoSortOrder::Asc => {
                    results.sort_by(|a, b| {
                        let da = Self::haversine_distance(
                            &center_pos,
                            &geo_members
                                .iter()
                                .find(|m| m.member == a.member)
                                .unwrap()
                                .position,
                        );
                        let db = Self::haversine_distance(
                            &center_pos,
                            &geo_members
                                .iter()
                                .find(|m| m.member == b.member)
                                .unwrap()
                                .position,
                        );
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                GeoSortOrder::Desc => {
                    results.sort_by(|a, b| {
                        let da = Self::haversine_distance(
                            &center_pos,
                            &geo_members
                                .iter()
                                .find(|m| m.member == a.member)
                                .unwrap()
                                .position,
                        );
                        let db = Self::haversine_distance(
                            &center_pos,
                            &geo_members
                                .iter()
                                .find(|m| m.member == b.member)
                                .unwrap()
                                .position,
                        );
                        db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }

        // Apply count limit
        if let Some(count) = options.count {
            results.truncate(count);
        }

        Ok(results)
    }

    async fn geo_search_store(
        &self,
        dest_key: &str,
        source_key: &str,
        center: GeoSearchCenter,
        shape: GeoSearchShape,
        options: GeoSearchOptions,
        _store_dist: bool,
    ) -> Result<GeoSearchStoreResult, CacheError> {
        // Search for results
        let search_results = self.geo_search(source_key, center, shape, options).await?;

        // Store results in destination key
        let mut store = self.store.lock().expect("store lock");
        let source_members = store.get(source_key).cloned().unwrap_or_default();
        let dest_members: Vec<GeoMember> = search_results
            .iter()
            .filter_map(|r| {
                source_members
                    .iter()
                    .find(|m| m.member == r.member)
                    .cloned()
            })
            .collect();

        let stored = dest_members.len() as i64;
        store.insert(dest_key.to_string(), dest_members);

        Ok(GeoSearchStoreResult { stored })
    }

    async fn geo_radius(
        &self,
        key: &str,
        position: GeoPosition,
        radius: f64,
        unit: GeoUnit,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError> {
        self.geo_search(
            key,
            GeoSearchCenter::FromLonLat(position),
            GeoSearchShape::ByRadius { radius, unit },
            options,
        )
        .await
    }

    async fn geo_radius_by_member(
        &self,
        key: &str,
        member: &str,
        radius: f64,
        unit: GeoUnit,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError> {
        self.geo_search(
            key,
            GeoSearchCenter::FromMember(member.to_string()),
            GeoSearchShape::ByRadius { radius, unit },
            options,
        )
        .await
    }
}

// ========== Mock Pub/Sub Repository ==========

#[derive(Default)]
pub struct MockPubSubRepository {
    channels: Mutex<HashMap<String, i64>>,
    patterns: Mutex<i64>,
}

impl MockPubSubRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl PubSubRepository for MockPubSubRepository {
    async fn publish(&self, channel: &str, _message: &str) -> Result<PublishResult, CacheError> {
        let channels = self.channels.lock().expect("channels lock");
        let receivers = channels.get(channel).copied().unwrap_or(0);
        Ok(PublishResult {
            channel: channel.to_string(),
            receivers,
        })
    }

    async fn spublish(&self, channel: &str, _message: &str) -> Result<PublishResult, CacheError> {
        let channels = self.channels.lock().expect("channels lock");
        let receivers = channels.get(channel).copied().unwrap_or(0);
        Ok(PublishResult {
            channel: channel.to_string(),
            receivers,
        })
    }

    async fn pubsub_channels(&self, _pattern: Option<&str>) -> Result<Vec<String>, CacheError> {
        let channels = self.channels.lock().expect("channels lock");
        Ok(channels.keys().cloned().collect())
    }

    async fn pubsub_numsub(&self, channels: &[String]) -> Result<Vec<NumSubResult>, CacheError> {
        let store = self.channels.lock().expect("channels lock");
        Ok(channels
            .iter()
            .map(|ch| NumSubResult {
                channel: ch.clone(),
                subscribers: store.get(ch).copied().unwrap_or(0),
            })
            .collect())
    }

    async fn pubsub_numpat(&self) -> Result<i64, CacheError> {
        Ok(*self.patterns.lock().expect("patterns lock"))
    }

    async fn pubsub_shardchannels(
        &self,
        _pattern: Option<&str>,
    ) -> Result<Vec<String>, CacheError> {
        let channels = self.channels.lock().expect("channels lock");
        Ok(channels.keys().cloned().collect())
    }

    async fn pubsub_shardnumsub(
        &self,
        channels: &[String],
    ) -> Result<Vec<NumSubResult>, CacheError> {
        self.pubsub_numsub(channels).await
    }
}

// ==================== Mock Cluster Repository ====================

pub struct MockClusterRepository;

#[async_trait]
impl ClusterRepository for MockClusterRepository {
    async fn cluster_info(&self) -> Result<ClusterInfo, CacheError> {
        Err(CacheError::Internal("not in cluster mode".to_string()))
    }

    async fn cluster_nodes(&self) -> Result<Vec<ClusterNode>, CacheError> {
        Err(CacheError::Internal("not in cluster mode".to_string()))
    }

    async fn cluster_slots(&self) -> Result<Vec<ClusterSlotRange>, CacheError> {
        Err(CacheError::Internal("not in cluster mode".to_string()))
    }

    async fn cluster_shards(&self) -> Result<redis::Value, CacheError> {
        Err(CacheError::Internal("not in cluster mode".to_string()))
    }

    async fn cluster_keyslot(&self, _key: &str) -> Result<u16, CacheError> {
        Err(CacheError::Internal("not in cluster mode".to_string()))
    }

    async fn cluster_slot_stats(
        &self,
        _filter: crate::domain::repositories::ClusterSlotStatsFilter,
    ) -> Result<Vec<crate::domain::repositories::SlotStats>, CacheError> {
        Err(CacheError::Internal("not in cluster mode".to_string()))
    }
}

pub struct MockVectorRepository;

#[async_trait]
impl crate::domain::repositories::VectorRepository for MockVectorRepository {
    async fn vadd(
        &self,
        key: &str,
        items: Vec<(String, Vec<f32>)>,
    ) -> Result<crate::domain::entities::VectorAddResult, CacheError> {
        Ok(crate::domain::entities::VectorAddResult {
            key: key.to_string(),
            added_count: items.len() as u64,
        })
    }
    async fn vrem(&self, _key: &str, items: Vec<String>) -> Result<u64, CacheError> {
        Ok(items.len() as u64)
    }
    async fn vsim(
        &self,
        _key: &str,
        _vector: Vec<f32>,
        _k: u64,
    ) -> Result<crate::domain::entities::VectorSimResult, CacheError> {
        Ok(crate::domain::entities::VectorSimResult { items: vec![] })
    }
    async fn vcard(&self, _key: &str) -> Result<u64, CacheError> {
        Ok(42)
    }
    async fn vdim(&self, _key: &str) -> Result<u64, CacheError> {
        Ok(128)
    }
    async fn vemb(
        &self,
        _key: &str,
        items: Vec<String>,
    ) -> Result<Vec<Option<Vec<f32>>>, CacheError> {
        Ok(items.into_iter().map(|_| Some(vec![1.0, 2.0])).collect())
    }
    async fn vismember(&self, _key: &str, items: Vec<String>) -> Result<Vec<bool>, CacheError> {
        Ok(items.into_iter().map(|_| true).collect())
    }
    async fn vlinks(&self, _key: &str, _item: &str) -> Result<Vec<Vec<String>>, CacheError> {
        Ok(vec![vec!["neighbor".to_string()]])
    }
    async fn vrandmember(&self, _key: &str, _count: i64) -> Result<Vec<String>, CacheError> {
        Ok(vec!["member1".to_string()])
    }
    async fn vrange(
        &self,
        _key: &str,
        _start: &str,
        _end: &str,
        _count: Option<i64>,
    ) -> Result<crate::domain::entities::VectorRangeResult, CacheError> {
        Ok(crate::domain::entities::VectorRangeResult { items: vec![] })
    }
    async fn vinfo(&self, _key: &str) -> Result<crate::domain::entities::VectorInfo, CacheError> {
        Ok(crate::domain::entities::VectorInfo {
            dimension: 128,
            distance_metric: "L2".to_string(),
            data_type: "FLOAT32".to_string(),
            count: 10,
        })
    }
    async fn vgetattr(&self, _key: &str, _item: &str) -> Result<Option<String>, CacheError> {
        Ok(Some("{}".to_string()))
    }
    async fn vsetattr(
        &self,
        _key: &str,
        _item: &str,
        _attributes: &str,
    ) -> Result<bool, CacheError> {
        Ok(true)
    }
}

pub fn test_state_with_vector_repo() -> (AppState, Arc<MockVectorRepository>) {
    let vector_repo = Arc::new(MockVectorRepository);
    let mut state = test_state_with_repos(
        Arc::new(MockStringRepository::new()),
        Arc::new(MockKeyRepository::new()),
        Arc::new(MockAdminRepository),
    );
    state.vector_service = Arc::new(crate::application::services::VectorService::new(
        vector_repo.clone(),
    ));
    let mut caps = (*state.capabilities).clone();
    caps.features.vectors = true;
    caps.features.vector_range = true;
    state.capabilities = Arc::new(caps);
    (state, vector_repo)
}
