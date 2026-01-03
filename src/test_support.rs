#![cfg(test)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use crate::application::services::{AdminService, HashService, KeyService, StringService};
use crate::domain::entities::{
    AclLogEntry, BgRewriteAofResult, BgSaveResult, ClientInfo, ClientKillOptions,
    ClientPauseOptions, CopyKeyOptions, CopyOptions, CopyResult, DeleteResult, DumpResult,
    ExistsResult, ExpireOptions, ExpireResult, FlushOptions, FlushResult, KeyInfo, LatencyEvent,
    MemoryStats, MemoryUsage, MoveKeyOptions, PersistResult, RandomKeyResult, RenameResult,
    ScanResult, ServerInfo, ServerTime, SlowlogEntry, TouchResult,
    AppendResult, GetExOptions, MGetResult, RangeResult, SetOptions, SetRangeResult,
    SetResult, StringValue,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::{AdminRepository, HashRepository, KeyRepository, StringRepository};
use crate::infrastructure::config::Settings;
use crate::infrastructure::redis::capabilities::RedisCapabilities;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::shared::app_state::AppState;

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
        self.store.lock().expect("store lock").insert(key.to_string(), value.to_string());
    }
}

#[async_trait]
impl StringRepository for MockStringRepository {
    async fn get(&self, key: &str) -> Result<Option<StringValue>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).map(|value| StringValue::new(key.to_string(), value.clone())))
    }

    async fn set(&self, key: &str, value: &str, options: SetOptions) -> Result<SetResult, CacheError> {
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

    async fn set_nx(&self, key: &str, value: &str, _ttl: Option<Duration>) -> Result<bool, CacheError> {
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
        let start = if start < 0 { (len + start).max(0) } else { start };
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

    async fn set_range(&self, key: &str, offset: i64, value: &str) -> Result<SetRangeResult, CacheError> {
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

    async fn get_ex(&self, key: &str, _options: GetExOptions) -> Result<Option<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(store.get(key).cloned())
    }

    async fn get_del(&self, key: &str) -> Result<Option<String>, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        Ok(store.remove(key))
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

    async fn acl_log(&self, _count: Option<i64>, _reset: bool) -> Result<Vec<AclLogEntry>, CacheError> {
        Ok(vec![AclLogEntry::default()])
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
        self.store.lock().expect("store lock").insert(key.to_string(), value.to_string());
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
        Ok(DeleteResult { deleted, not_found, count })
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
        Ok(ExistsResult { existing, missing, count })
    }

    async fn expire(&self, key: &str, _seconds: i64, _options: ExpireOptions) -> Result<ExpireResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(ExpireResult { key: key.to_string(), success: store.contains_key(key), new_ttl: None })
    }

    async fn expire_at(&self, key: &str, _timestamp: i64, _options: ExpireOptions) -> Result<ExpireResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(ExpireResult { key: key.to_string(), success: store.contains_key(key), new_ttl: None })
    }

    async fn pexpire(&self, key: &str, _milliseconds: i64, _options: ExpireOptions) -> Result<ExpireResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(ExpireResult { key: key.to_string(), success: store.contains_key(key), new_ttl: None })
    }

    async fn pexpire_at(&self, key: &str, _timestamp: i64, _options: ExpireOptions) -> Result<ExpireResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(ExpireResult { key: key.to_string(), success: store.contains_key(key), new_ttl: None })
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
        Ok(PersistResult { key: key.to_string(), success: store.contains_key(key) })
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
            Ok(RenameResult { old_key: key.to_string(), new_key: new_key.to_string(), success: true })
        } else {
            Err(CacheError::KeyNotFound(key.to_string()))
        }
    }

    async fn rename_nx(&self, key: &str, new_key: &str) -> Result<RenameResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if store.contains_key(new_key) {
            return Ok(RenameResult { old_key: key.to_string(), new_key: new_key.to_string(), success: false });
        }
        if let Some(value) = store.remove(key) {
            store.insert(new_key.to_string(), value);
            Ok(RenameResult { old_key: key.to_string(), new_key: new_key.to_string(), success: true })
        } else {
            Err(CacheError::KeyNotFound(key.to_string()))
        }
    }

    async fn copy(&self, source: &str, destination: &str, options: CopyOptions) -> Result<CopyResult, CacheError> {
        let mut store = self.store.lock().expect("store lock");
        if !options.replace && store.contains_key(destination) {
            return Ok(CopyResult { source: source.to_string(), destination: destination.to_string(), success: false });
        }
        if let Some(value) = store.get(source).cloned() {
            store.insert(destination.to_string(), value);
            Ok(CopyResult { source: source.to_string(), destination: destination.to_string(), success: true })
        } else {
            Ok(CopyResult { source: source.to_string(), destination: destination.to_string(), success: false })
        }
    }

    async fn scan(&self, cursor: u64, pattern: Option<&str>, count: Option<u64>, _key_type: Option<&str>) -> Result<ScanResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        let keys: Vec<String> = store.keys()
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
        let next_cursor = if cursor == 0 && !keys.is_empty() { 0 } else { 0 };
        Ok(ScanResult { cursor: next_cursor, keys, count: key_count })
    }

    async fn keys(&self, pattern: &str) -> Result<Vec<String>, CacheError> {
        let store = self.store.lock().expect("store lock");
        let pat = pattern.replace('*', "");
        Ok(store.keys()
            .filter(|k| pat.is_empty() || k.contains(&pat))
            .cloned()
            .collect())
    }

    async fn random_key(&self) -> Result<RandomKeyResult, CacheError> {
        let store = self.store.lock().expect("store lock");
        Ok(RandomKeyResult { key: store.keys().next().cloned() })
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
            Ok(DumpResult { key: key.to_string(), data: Some(value.clone()) })
        } else {
            Ok(DumpResult { key: key.to_string(), data: None })
        }
    }

    async fn restore(&self, key: &str, _ttl: i64, data: &[u8], replace: bool) -> Result<bool, CacheError> {
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
        Ok(store
            .get(key)
            .map_or(false, |map| map.contains_key(field)))
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

    async fn hrand_field(&self, key: &str, count: Option<i64>, with_values: bool) -> Result<Vec<String>, CacheError> {
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

    async fn hscan(&self, key: &str, _cursor: u64, pattern: Option<String>, count: Option<u64>) -> Result<(u64, Vec<String>), CacheError> {
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
            if let Some(limit) = count {
                if (result.len() / 2) as u64 >= limit {
                    break;
                }
            }
        }
        Ok((0, result))
    }
}

pub fn test_state_with_repos(
    string_repo: Arc<MockStringRepository>,
    key_repo: Arc<MockKeyRepository>,
    admin_repo: Arc<MockAdminRepository>,
) -> AppState {
    let hash_repo = Arc::new(MockHashRepository::new());
    test_state_with_all_repos(string_repo, hash_repo, key_repo, admin_repo)
}

pub fn test_state_with_all_repos(
    string_repo: Arc<MockStringRepository>,
    hash_repo: Arc<MockHashRepository>,
    key_repo: Arc<MockKeyRepository>,
    admin_repo: Arc<MockAdminRepository>,
) -> AppState {
    let pool = Arc::new(InstrumentedPool::new_for_tests());
    let config = Arc::new(Settings::default());
    let capabilities = Arc::new(RedisCapabilities::default_capabilities());
    let string_service = Arc::new(StringService::new_with_repository(string_repo));
    let hash_service = Arc::new(HashService::new_with_repository(hash_repo));
    let key_service = Arc::new(KeyService::new_with_repository(key_repo));
    let admin_service = Arc::new(AdminService::new_with_repository(admin_repo));

    AppState::new_with_services(pool, config, capabilities, string_service, hash_service, key_service, admin_service)
}

pub fn test_state() -> (AppState, Arc<MockStringRepository>, Arc<MockKeyRepository>, Arc<MockAdminRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository::default());
    let state = test_state_with_repos(string_repo.clone(), key_repo.clone(), admin_repo.clone());
    (state, string_repo, key_repo, admin_repo)
}

pub fn test_state_with_hash_repo() -> (AppState, Arc<MockHashRepository>) {
    let string_repo = Arc::new(MockStringRepository::new());
    let key_repo = Arc::new(MockKeyRepository::new());
    let admin_repo = Arc::new(MockAdminRepository::default());
    let hash_repo = Arc::new(MockHashRepository::new());
    let state = test_state_with_all_repos(string_repo, hash_repo.clone(), key_repo, admin_repo);
    (state, hash_repo)
}
