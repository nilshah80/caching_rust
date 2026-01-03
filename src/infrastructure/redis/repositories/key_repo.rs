//! Redis Key Repository Implementation
//!
//! Concrete implementation of KeyRepository using Redis.

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use redis::AsyncCommands;
use std::sync::Arc;

use crate::domain::entities::{
    CopyOptions, CopyResult, DeleteResult, DumpResult, ExistsResult, ExpireOptions,
    ExpireResult, KeyInfo, PersistResult, RandomKeyResult, RenameResult,
    ScanResult, TouchResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::KeyRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Redis implementation of KeyRepository
pub struct RedisKeyRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisKeyRepository {
    /// Create a new RedisKeyRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl KeyRepository for RedisKeyRepository {
    async fn delete(&self, keys: &[String]) -> Result<DeleteResult, CacheError> {
        if keys.is_empty() {
            return Ok(DeleteResult {
                deleted: vec![],
                not_found: vec![],
                count: 0,
            });
        }

        let mut conn = self.pool.get().await?;

        // Check each key individually for accurate results
        let mut existing = Vec::new();
        let mut not_found = Vec::new();
        for key in keys {
            let exists: i64 = conn.exists(key).await.unwrap_or(0);
            if exists > 0 {
                existing.push(key.clone());
            } else {
                not_found.push(key.clone());
            }
        }

        // Delete all keys
        let count: i64 = conn.del(keys).await?;

        Ok(DeleteResult {
            deleted: existing,
            not_found,
            count: count as usize,
        })
    }

    async fn exists(&self, keys: &[String]) -> Result<ExistsResult, CacheError> {
        if keys.is_empty() {
            return Ok(ExistsResult {
                existing: vec![],
                missing: vec![],
                count: 0,
            });
        }

        let mut conn = self.pool.get().await?;

        let mut existing = Vec::new();
        let mut missing = Vec::new();

        // Check each key individually
        for key in keys {
            let exists: i64 = conn.exists(key).await?;
            if exists > 0 {
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

    async fn expire(&self, key: &str, seconds: i64, options: ExpireOptions) -> Result<ExpireResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("EXPIRE");
        cmd.arg(key).arg(seconds);

        if options.nx {
            cmd.arg("NX");
        }
        if options.xx {
            cmd.arg("XX");
        }
        if options.gt {
            cmd.arg("GT");
        }
        if options.lt {
            cmd.arg("LT");
        }

        let result: i64 = cmd.query_async(&mut conn).await?;

        Ok(ExpireResult {
            key: key.to_string(),
            success: result == 1,
            new_ttl: if result == 1 { Some(seconds) } else { None },
        })
    }

    async fn expire_at(&self, key: &str, timestamp: i64, options: ExpireOptions) -> Result<ExpireResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("EXPIREAT");
        cmd.arg(key).arg(timestamp);

        if options.nx {
            cmd.arg("NX");
        }
        if options.xx {
            cmd.arg("XX");
        }
        if options.gt {
            cmd.arg("GT");
        }
        if options.lt {
            cmd.arg("LT");
        }

        let result: i64 = cmd.query_async(&mut conn).await?;

        Ok(ExpireResult {
            key: key.to_string(),
            success: result == 1,
            new_ttl: None,
        })
    }

    async fn pexpire(&self, key: &str, milliseconds: i64, options: ExpireOptions) -> Result<ExpireResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("PEXPIRE");
        cmd.arg(key).arg(milliseconds);

        if options.nx {
            cmd.arg("NX");
        }
        if options.xx {
            cmd.arg("XX");
        }
        if options.gt {
            cmd.arg("GT");
        }
        if options.lt {
            cmd.arg("LT");
        }

        let result: i64 = cmd.query_async(&mut conn).await?;

        Ok(ExpireResult {
            key: key.to_string(),
            success: result == 1,
            new_ttl: if result == 1 { Some(milliseconds / 1000) } else { None },
        })
    }

    async fn pexpire_at(&self, key: &str, timestamp: i64, options: ExpireOptions) -> Result<ExpireResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("PEXPIREAT");
        cmd.arg(key).arg(timestamp);

        if options.nx {
            cmd.arg("NX");
        }
        if options.xx {
            cmd.arg("XX");
        }
        if options.gt {
            cmd.arg("GT");
        }
        if options.lt {
            cmd.arg("LT");
        }

        let result: i64 = cmd.query_async(&mut conn).await?;

        Ok(ExpireResult {
            key: key.to_string(),
            success: result == 1,
            new_ttl: None,
        })
    }

    async fn ttl(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let ttl: i64 = conn.ttl(key).await?;
        Ok(ttl)
    }

    async fn pttl(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let pttl: i64 = conn.pttl(key).await?;
        Ok(pttl)
    }

    async fn persist(&self, key: &str) -> Result<PersistResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = conn.persist(key).await?;

        Ok(PersistResult {
            key: key.to_string(),
            success: result == 1,
        })
    }

    async fn key_type(&self, key: &str) -> Result<String, CacheError> {
        let mut conn = self.pool.get().await?;
        let key_type: String = redis::cmd("TYPE")
            .arg(key)
            .query_async(&mut conn)
            .await?;
        Ok(key_type)
    }

    async fn rename(&self, key: &str, new_key: &str) -> Result<RenameResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: Result<(), redis::RedisError> = conn.rename(key, new_key).await;

        match result {
            Ok(()) => Ok(RenameResult {
                old_key: key.to_string(),
                new_key: new_key.to_string(),
                success: true,
            }),
            Err(e) => {
                if e.to_string().contains("no such key") {
                    Err(CacheError::KeyNotFound(key.to_string()))
                } else {
                    Err(CacheError::from(e))
                }
            }
        }
    }

    async fn rename_nx(&self, key: &str, new_key: &str) -> Result<RenameResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: i64 = conn.rename_nx(key, new_key).await.map_err(|e| {
            if e.to_string().contains("no such key") {
                CacheError::KeyNotFound(key.to_string())
            } else {
                CacheError::from(e)
            }
        })?;

        Ok(RenameResult {
            old_key: key.to_string(),
            new_key: new_key.to_string(),
            success: result == 1,
        })
    }

    async fn copy(&self, source: &str, destination: &str, options: CopyOptions) -> Result<CopyResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("COPY");
        cmd.arg(source).arg(destination);

        if let Some(db) = options.db {
            cmd.arg("DB").arg(db);
        }
        if options.replace {
            cmd.arg("REPLACE");
        }

        let result: i64 = cmd.query_async(&mut conn).await?;

        Ok(CopyResult {
            source: source.to_string(),
            destination: destination.to_string(),
            success: result == 1,
        })
    }

    async fn scan(&self, cursor: u64, pattern: Option<&str>, count: Option<u64>, key_type: Option<&str>) -> Result<ScanResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("SCAN");
        cmd.arg(cursor);

        if let Some(p) = pattern {
            cmd.arg("MATCH").arg(p);
        }
        if let Some(c) = count {
            cmd.arg("COUNT").arg(c);
        }
        if let Some(t) = key_type {
            cmd.arg("TYPE").arg(t);
        }

        let (new_cursor, keys): (u64, Vec<String>) = cmd.query_async(&mut conn).await?;

        Ok(ScanResult {
            cursor: new_cursor,
            count: keys.len(),
            keys,
        })
    }

    async fn keys(&self, pattern: &str) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let keys: Vec<String> = conn.keys(pattern).await?;
        Ok(keys)
    }

    async fn random_key(&self) -> Result<RandomKeyResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let key: Option<String> = redis::cmd("RANDOMKEY")
            .query_async(&mut conn)
            .await?;

        Ok(RandomKeyResult { key })
    }

    async fn touch(&self, keys: &[String]) -> Result<TouchResult, CacheError> {
        if keys.is_empty() {
            return Ok(TouchResult { count: 0 });
        }

        let mut conn = self.pool.get().await?;
        let count: i64 = redis::cmd("TOUCH")
            .arg(keys)
            .query_async(&mut conn)
            .await?;

        Ok(TouchResult { count: count as usize })
    }

    async fn unlink(&self, keys: &[String]) -> Result<DeleteResult, CacheError> {
        if keys.is_empty() {
            return Ok(DeleteResult {
                deleted: vec![],
                not_found: vec![],
                count: 0,
            });
        }

        let mut conn = self.pool.get().await?;

        // Check which keys exist before unlinking
        let mut existing = Vec::new();
        let mut not_found = Vec::new();
        for key in keys {
            let exists: i64 = conn.exists(key).await.unwrap_or(0);
            if exists > 0 {
                existing.push(key.clone());
            } else {
                not_found.push(key.clone());
            }
        }

        let count: i64 = redis::cmd("UNLINK")
            .arg(keys)
            .query_async(&mut conn)
            .await?;

        Ok(DeleteResult {
            deleted: existing,
            not_found,
            count: count as usize,
        })
    }

    async fn dump(&self, key: &str) -> Result<DumpResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let data: Option<Vec<u8>> = redis::cmd("DUMP")
            .arg(key)
            .query_async(&mut conn)
            .await?;

        Ok(DumpResult {
            key: key.to_string(),
            data: data.map(|d| BASE64.encode(&d)),
        })
    }

    async fn restore(&self, key: &str, ttl: i64, data: &[u8], replace: bool) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("RESTORE");
        cmd.arg(key).arg(ttl).arg(data);

        if replace {
            cmd.arg("REPLACE");
        }

        let result: Result<(), redis::RedisError> = cmd.query_async(&mut conn).await;

        match result {
            Ok(()) => Ok(true),
            Err(e) => {
                if e.to_string().contains("BUSYKEY") {
                    Ok(false)
                } else {
                    Err(CacheError::from(e))
                }
            }
        }
    }

    async fn object_encoding(&self, key: &str) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let encoding: Option<String> = redis::cmd("OBJECT")
            .arg("ENCODING")
            .arg(key)
            .query_async(&mut conn)
            .await?;
        Ok(encoding)
    }

    async fn object_idletime(&self, key: &str) -> Result<Option<u64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let idle: Option<u64> = redis::cmd("OBJECT")
            .arg("IDLETIME")
            .arg(key)
            .query_async(&mut conn)
            .await?;
        Ok(idle)
    }

    async fn object_refcount(&self, key: &str) -> Result<Option<u64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let count: Option<u64> = redis::cmd("OBJECT")
            .arg("REFCOUNT")
            .arg(key)
            .query_async(&mut conn)
            .await?;
        Ok(count)
    }

    async fn object_freq(&self, key: &str) -> Result<Option<u64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let freq: Option<u64> = redis::cmd("OBJECT")
            .arg("FREQ")
            .arg(key)
            .query_async(&mut conn)
            .await
            .ok();
        Ok(freq)
    }

    async fn key_info(&self, key: &str) -> Result<KeyInfo, CacheError> {
        let mut conn = self.pool.get().await?;

        // Check if key exists first
        let exists: i64 = conn.exists(key).await?;
        if exists == 0 {
            return Ok(KeyInfo::not_found(key.to_string()));
        }

        // Get type, TTL, PTTL in a pipeline
        let (key_type, ttl, pttl): (String, i64, i64) = redis::pipe()
            .cmd("TYPE").arg(key)
            .cmd("TTL").arg(key)
            .cmd("PTTL").arg(key)
            .query_async(&mut conn)
            .await?;

        let mut info = KeyInfo::new(key.to_string(), key_type, ttl)
            .with_pttl(pttl);

        // Try to get OBJECT info (may fail for some types)
        if let Ok(encoding) = self.object_encoding(key).await {
            if let Some(enc) = encoding {
                info = info.with_encoding(enc);
            }
        }

        if let Ok(idle) = self.object_idletime(key).await {
            if let Some(i) = idle {
                info = info.with_idle_time(i);
            }
        }

        if let Ok(refcount) = self.object_refcount(key).await {
            if let Some(rc) = refcount {
                info = info.with_ref_count(rc);
            }
        }

        // Try to get memory usage (Redis 4.0+)
        let memory: Option<i64> = redis::cmd("MEMORY")
            .arg("USAGE")
            .arg(key)
            .query_async(&mut conn)
            .await
            .ok();
        if let Some(mem) = memory {
            info = info.with_memory_usage(mem);
        }

        Ok(info)
    }

    async fn expire_time(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let time: i64 = redis::cmd("EXPIRETIME")
            .arg(key)
            .query_async(&mut conn)
            .await?;
        Ok(time)
    }

    async fn pexpire_time(&self, key: &str) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let time: i64 = redis::cmd("PEXPIRETIME")
            .arg(key)
            .query_async(&mut conn)
            .await?;
        Ok(time)
    }
}
