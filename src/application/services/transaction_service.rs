//! Transaction Service
//!
//! Business logic for Redis transaction operations.
//! Implements the single-request bundled transaction model using MULTI/EXEC.
//!
//! # Overview
//!
//! This service provides atomic transaction execution for Redis operations.
//! All commands in a transaction are executed atomically using Redis MULTI/EXEC.
//!
//! # Limits
//!
//! - Maximum 100 commands per transaction
//! - Maximum 20 watch keys for optimistic locking
//! - 30 second execution timeout
//!
//! # Examples
//!
//! ## Basic Transaction
//!
//! ```json
//! POST /api/v1/transactions/execute
//! {
//!   "commands": [
//!     {"type": "SET", "key": "counter", "value": "0"},
//!     {"type": "INCR", "key": "counter"},
//!     {"type": "GET", "key": "counter"}
//!   ]
//! }
//! ```
//!
//! ## Transaction with WATCH (Optimistic Locking)
//!
//! ```json
//! POST /api/v1/transactions/execute
//! {
//!   "watch_keys": ["inventory:item:123"],
//!   "commands": [
//!     {"type": "GET", "key": "inventory:item:123"},
//!     {"type": "DECR", "key": "inventory:item:123"}
//!   ]
//! }
//! ```
//!
//! If the watched key is modified by another client before EXEC,
//! the transaction will fail with a 409 Conflict response.
//!
//! ## Compare-and-Set (String)
//!
//! ```json
//! POST /api/v1/transactions/cas
//! {
//!   "key": "version",
//!   "expected_value": "1",
//!   "new_value": "2"
//! }
//! ```
//!
//! ## Compare-and-Set (Hash Field)
//!
//! ```json
//! POST /api/v1/transactions/hcas
//! {
//!   "key": "user:1",
//!   "field": "version",
//!   "expected_value": "1",
//!   "new_value": "2"
//! }
//! ```

use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
#[cfg(test)]
use tokio::sync::Notify;
use tokio::time::Instant;

use crate::api::http::schemas::transactions::{
    CommandResult, CompareAndSetRequest, CompareAndSetResponse, HCompareAndSetRequest,
    RedisCommand, TransactionRequest, TransactionResponse,
};
use crate::domain::errors::CacheError;
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Maximum number of commands allowed in a single transaction
const MAX_TRANSACTION_COMMANDS: usize = 100;

/// Maximum number of keys that can be watched in a transaction
const MAX_WATCH_KEYS: usize = 20;

/// Default timeout for transaction execution (30 seconds)
const TRANSACTION_TIMEOUT_SECS: u64 = 30;

#[cfg(test)]
static EXECUTE_DELAY_MS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
struct WatchHooks {
    started: Arc<Notify>,
    proceed: Arc<Notify>,
}

#[cfg(test)]
static WATCH_HOOKS: Mutex<Option<WatchHooks>> = Mutex::new(None);

#[cfg(test)]
pub(crate) fn set_test_execute_delay_ms(ms: u64) {
    EXECUTE_DELAY_MS.store(ms, Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn set_watch_hooks(started: Arc<Notify>, proceed: Arc<Notify>) {
    *WATCH_HOOKS.lock().expect("watch hooks lock") = Some(WatchHooks { started, proceed });
}

#[cfg(test)]
pub(crate) fn clear_watch_hooks() {
    *WATCH_HOOKS.lock().expect("watch hooks lock") = None;
}

/// Service for transaction operations
pub struct TransactionService {
    pool: Arc<InstrumentedPool>,
}

impl TransactionService {
    /// Create a new TransactionService
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }

    /// Map Redis errors to appropriate CacheError types for script-based operations.
    /// Script execution errors (Lua errors) -> ScriptError (400)
    /// Connection/transport errors -> appropriate 5xx errors
    fn map_script_error(e: redis::RedisError) -> CacheError {
        use redis::ErrorKind;
        match e.kind() {
            // Script-specific errors -> 400
            ErrorKind::NoScriptError => {
                CacheError::ScriptError("Script not found in cache".to_string())
            }
            // Extension errors include Lua runtime errors
            ErrorKind::ExtensionError => CacheError::ScriptError(format!("Script error: {}", e)),
            // Response errors from Redis (including Lua errors)
            ErrorKind::ResponseError => {
                let msg = e.to_string();
                if msg.contains("NOSCRIPT")
                    || msg.contains("ERR Error")
                    || msg.contains("@user_script")
                {
                    CacheError::ScriptError(format!("Script error: {}", e))
                } else {
                    CacheError::RedisError(e)
                }
            }
            // Connection/transport errors -> 5xx
            ErrorKind::IoError => CacheError::ConnectionFailed(e.to_string()),
            ErrorKind::ClientError => CacheError::ConnectionFailed(e.to_string()),
            // Other errors use default RedisError mapping (500)
            _ => CacheError::RedisError(e),
        }
    }

    /// Execute multiple commands atomically in a single request.
    ///
    /// This method:
    /// 1. Validates command and watch key limits
    /// 2. Optionally WATCHes keys for optimistic locking
    /// 3. Wraps all commands in MULTI/EXEC for atomicity
    /// 4. Parses results and returns them in order
    ///
    /// If WATCH is used and a watched key is modified by another client
    /// before EXEC completes, the transaction will fail with TransactionAborted error (409).
    ///
    /// # Limits
    /// - Maximum 100 commands per transaction
    /// - Maximum 20 watch keys
    /// - 30 second execution timeout
    ///
    /// # Timeout Behavior
    /// If execution exceeds the timeout, a 504 Gateway Timeout is returned.
    /// The timeout is checked before acquiring a connection and after the Redis
    /// operation completes to avoid canceling in-flight operations and potentially
    /// corrupting pooled connections.
    pub async fn execute(
        &self,
        request: TransactionRequest,
    ) -> Result<TransactionResponse, CacheError> {
        let deadline = Instant::now() + Duration::from_secs(TRANSACTION_TIMEOUT_SECS);
        self.execute_with_deadline(request, deadline).await
    }

    async fn execute_with_deadline(
        &self,
        request: TransactionRequest,
        deadline: Instant,
    ) -> Result<TransactionResponse, CacheError> {
        // Validate command count
        if request.commands.is_empty() {
            return Err(CacheError::InvalidInput(
                "At least one command is required".to_string(),
            ));
        }

        if request.commands.len() > MAX_TRANSACTION_COMMANDS {
            return Err(CacheError::InvalidInput(format!(
                "Transaction cannot exceed {} commands (got {})",
                MAX_TRANSACTION_COMMANDS,
                request.commands.len()
            )));
        }

        // Validate watch keys
        if let Some(ref keys) = request.watch_keys
            && keys.len() > MAX_WATCH_KEYS
        {
            return Err(CacheError::InvalidInput(format!(
                "Cannot watch more than {} keys (got {})",
                MAX_WATCH_KEYS,
                keys.len()
            )));
        }

        // Check timeout before acquiring connection
        if Instant::now() >= deadline {
            return Err(CacheError::Timeout);
        }

        // Execute transaction (we let this complete even if it exceeds deadline
        // to avoid canceling mid-operation and corrupting pooled connections)
        let result = self.execute_internal(request).await;

        // Check timeout after completion - if we exceeded deadline, report timeout
        // even if the operation succeeded (client may have given up already)
        if Instant::now() >= deadline {
            return Err(CacheError::Timeout);
        }

        result
    }

    /// Internal transaction execution logic
    async fn execute_internal(
        &self,
        request: TransactionRequest,
    ) -> Result<TransactionResponse, CacheError> {
        let mut conn = self.pool.get().await?;

        #[cfg(test)]
        {
            let delay_ms = EXECUTE_DELAY_MS.load(Ordering::Relaxed);
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }

        // Track whether WATCH was issued (for cleanup on errors)
        let mut watched = false;

        // Optional WATCH for optimistic locking
        if let Some(ref keys) = request.watch_keys
            && !keys.is_empty()
        {
            // Validate watch keys before issuing WATCH
            for key in keys {
                if key.is_empty() {
                    return Err(CacheError::InvalidInput(
                        "Watch key cannot be empty".to_string(),
                    ));
                }
            }
            redis::cmd("WATCH")
                .arg(keys)
                .query_async::<()>(&mut *conn)
                .await?;
            watched = true;

            #[cfg(test)]
            {
                let hooks = WATCH_HOOKS
                    .lock()
                    .expect("watch hooks lock")
                    .as_ref()
                    .map(|h| (h.started.clone(), h.proceed.clone()));
                if let Some((started, proceed)) = hooks {
                    started.notify_one();
                    proceed.notified().await;
                }
            }
        }

        // Build pipeline with MULTI/EXEC
        let mut pipe = redis::pipe();
        pipe.atomic(); // Wraps in MULTI/EXEC

        for cmd in &request.commands {
            if let Err(e) = Self::add_command_to_pipe(&mut pipe, cmd) {
                // Clear WATCH state before returning error to prevent state leak
                if watched {
                    let _ = redis::cmd("UNWATCH").query_async::<()>(&mut *conn).await;
                }
                return Err(e);
            }
        }

        // Execute atomically
        let results: Vec<redis::Value> = match pipe.query_async(&mut *conn).await {
            Ok(r) => r,
            Err(e) => return Self::handle_exec_error(&mut conn, e).await,
        };

        // Check for WATCH abort: when a watched key is modified, EXEC returns nil
        // which the redis crate maps to an empty Vec (not an error).
        // If we were watching keys and got fewer results than expected, the transaction was aborted.
        if watched && results.len() != request.commands.len() {
            // No need to UNWATCH - EXEC already cleared the WATCH state
            return Err(CacheError::TransactionAborted);
        }

        // Parse results
        let parsed_results = Self::parse_results(&request.commands, results);

        Ok(TransactionResponse {
            success: parsed_results.iter().all(|r| r.success),
            results: parsed_results,
        })
    }

    async fn handle_exec_error(
        conn: &mut deadpool_redis::Connection,
        err: redis::RedisError,
    ) -> Result<TransactionResponse, CacheError> {
        // Check if this is a WATCH abort (nil response means EXEC returned nil)
        let err_str = err.to_string();
        if err_str.contains("nil") || err_str.contains("EXECABORT") {
            // Clear WATCH state before returning connection to pool
            let _ = redis::cmd("UNWATCH").query_async::<()>(&mut *conn).await;
            return Err(CacheError::TransactionAborted);
        }

        // For other Redis errors, propagate as RedisError (maps to 500)
        Err(CacheError::RedisError(err))
    }

    /// Compare-and-set operation for string values using Lua script.
    ///
    /// Atomically sets the value only if the current value matches the expected value.
    /// This is useful for implementing optimistic locking patterns.
    pub async fn compare_and_set(
        &self,
        request: CompareAndSetRequest,
    ) -> Result<CompareAndSetResponse, CacheError> {
        if request.key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }

        let mut conn = self.pool.get().await?;

        // Lua script for atomic compare-and-set
        // Returns: [swapped (0/1), current_value]
        let script = redis::Script::new(
            r#"
            local current = redis.call('GET', KEYS[1])
            if current == ARGV[1] then
                redis.call('SET', KEYS[1], ARGV[2])
                return {1, ARGV[2]}
            else
                return {0, current}
            end
            "#,
        );

        let result: Vec<redis::Value> = script
            .key(&request.key)
            .arg(&request.expected_value)
            .arg(&request.new_value)
            .invoke_async(&mut *conn)
            .await
            .map_err(Self::map_script_error)?;

        Ok(Self::parse_compare_and_set_result(&result))
    }

    /// Compare-and-set operation for hash field values using Lua script.
    ///
    /// Atomically sets the hash field only if the current value matches the expected value.
    pub async fn hcompare_and_set(
        &self,
        request: HCompareAndSetRequest,
    ) -> Result<CompareAndSetResponse, CacheError> {
        if request.key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if request.field.is_empty() {
            return Err(CacheError::InvalidInput(
                "Field cannot be empty".to_string(),
            ));
        }

        let mut conn = self.pool.get().await?;

        // Lua script for atomic hash compare-and-set
        // Returns: [swapped (0/1), current_value]
        let script = redis::Script::new(
            r#"
            local current = redis.call('HGET', KEYS[1], ARGV[1])
            if current == ARGV[2] then
                redis.call('HSET', KEYS[1], ARGV[1], ARGV[3])
                return {1, ARGV[3]}
            else
                return {0, current}
            end
            "#,
        );

        let result: Vec<redis::Value> = script
            .key(&request.key)
            .arg(&request.field)
            .arg(&request.expected_value)
            .arg(&request.new_value)
            .invoke_async(&mut *conn)
            .await
            .map_err(Self::map_script_error)?;

        Ok(Self::parse_compare_and_set_result(&result))
    }

    fn parse_compare_and_set_result(result: &[redis::Value]) -> CompareAndSetResponse {
        let swapped = matches!(result.first(), Some(redis::Value::Int(1)));

        let current_value = match result.get(1) {
            Some(redis::Value::BulkString(bytes)) => {
                Some(String::from_utf8_lossy(bytes).to_string())
            }
            Some(redis::Value::SimpleString(s)) => Some(s.clone()),
            _ => None,
        };

        CompareAndSetResponse {
            swapped,
            current_value,
        }
    }

    /// Add a RedisCommand to the pipeline
    fn add_command_to_pipe(
        pipe: &mut redis::Pipeline,
        cmd: &RedisCommand,
    ) -> Result<(), CacheError> {
        match cmd {
            // String commands
            RedisCommand::Get { key } => {
                pipe.get(key);
            }
            RedisCommand::Set {
                key,
                value,
                ttl_seconds,
            } => {
                if let Some(ttl) = ttl_seconds {
                    pipe.set_ex(key, value, *ttl);
                } else {
                    pipe.set(key, value);
                }
            }
            RedisCommand::Incr { key } => {
                pipe.incr(key, 1i64);
            }
            RedisCommand::IncrBy { key, delta } => {
                pipe.incr(key, *delta);
            }
            RedisCommand::Decr { key } => {
                pipe.decr(key, 1i64);
            }
            RedisCommand::DecrBy { key, delta } => {
                pipe.decr(key, *delta);
            }
            RedisCommand::Append { key, value } => {
                pipe.append(key, value);
            }
            RedisCommand::SetNx { key, value } => {
                pipe.set_nx(key, value);
            }
            RedisCommand::GetSet { key, value } => {
                pipe.getset(key, value);
            }
            RedisCommand::MGet { keys } => {
                pipe.mget(keys);
            }
            RedisCommand::MSet { entries } => {
                let pairs: Vec<(&str, &str)> = entries
                    .iter()
                    .map(|kv| (kv.key.as_str(), kv.value.as_str()))
                    .collect();
                pipe.mset(&pairs);
            }

            // Hash commands
            RedisCommand::HGet { key, field } => {
                pipe.hget(key, field);
            }
            RedisCommand::HSet { key, field, value } => {
                pipe.hset(key, field, value);
            }
            RedisCommand::HMSet { key, fields } => {
                let pairs: Vec<(&str, &str)> = fields
                    .iter()
                    .map(|fv| (fv.field.as_str(), fv.value.as_str()))
                    .collect();
                pipe.hset_multiple(key, &pairs);
            }
            RedisCommand::HMGet { key, fields } => {
                pipe.hget(key, fields);
            }
            RedisCommand::HIncrBy { key, field, delta } => {
                pipe.hincr(key, field, *delta);
            }
            RedisCommand::HIncrByFloat { key, field, delta } => {
                pipe.hincr(key, field, *delta);
            }
            RedisCommand::HDel { key, fields } => {
                pipe.hdel(key, fields);
            }
            RedisCommand::HExists { key, field } => {
                pipe.hexists(key, field);
            }
            RedisCommand::HGetAll { key } => {
                pipe.hgetall(key);
            }
            RedisCommand::HKeys { key } => {
                pipe.hkeys(key);
            }
            RedisCommand::HVals { key } => {
                pipe.hvals(key);
            }
            RedisCommand::HLen { key } => {
                pipe.hlen(key);
            }
            RedisCommand::HSetNx { key, field, value } => {
                pipe.hset_nx(key, field, value);
            }

            // List commands
            RedisCommand::LPush { key, values } => {
                pipe.lpush(key, values);
            }
            RedisCommand::RPush { key, values } => {
                pipe.rpush(key, values);
            }
            RedisCommand::LPop { key, count } => {
                if let Some(c) = count {
                    if *c == 0 {
                        return Err(CacheError::InvalidInput(
                            "LPOP count must be greater than 0".to_string(),
                        ));
                    }
                    pipe.lpop(key, std::num::NonZeroUsize::new(*c as usize));
                } else {
                    pipe.lpop(key, None);
                }
            }
            RedisCommand::RPop { key, count } => {
                if let Some(c) = count {
                    if *c == 0 {
                        return Err(CacheError::InvalidInput(
                            "RPOP count must be greater than 0".to_string(),
                        ));
                    }
                    pipe.rpop(key, std::num::NonZeroUsize::new(*c as usize));
                } else {
                    pipe.rpop(key, None);
                }
            }
            RedisCommand::LLen { key } => {
                pipe.llen(key);
            }
            RedisCommand::LIndex { key, index } => {
                pipe.lindex(key, *index as isize);
            }
            RedisCommand::LRange { key, start, stop } => {
                pipe.lrange(key, *start as isize, *stop as isize);
            }
            RedisCommand::LSet { key, index, value } => {
                pipe.lset(key, *index as isize, value);
            }
            RedisCommand::LTrim { key, start, stop } => {
                pipe.ltrim(key, *start as isize, *stop as isize);
            }
            RedisCommand::LRem { key, count, value } => {
                pipe.lrem(key, *count as isize, value);
            }

            // Set commands
            RedisCommand::SAdd { key, members } => {
                pipe.sadd(key, members);
            }
            RedisCommand::SRem { key, members } => {
                pipe.srem(key, members);
            }
            RedisCommand::SIsMember { key, member } => {
                pipe.sismember(key, member);
            }
            RedisCommand::SMembers { key } => {
                pipe.smembers(key);
            }
            RedisCommand::SCard { key } => {
                pipe.scard(key);
            }
            RedisCommand::SPop { key, count } => {
                let mut cmd = redis::cmd("SPOP");
                cmd.arg(key);
                if let Some(c) = count {
                    cmd.arg(*c);
                }
                pipe.add_command(cmd);
            }
            RedisCommand::SMove {
                source,
                destination,
                member,
            } => {
                pipe.smove(source, destination, member);
            }

            // Sorted set commands
            RedisCommand::ZAdd { key, members } => {
                let items: Vec<(f64, &str)> = members
                    .iter()
                    .map(|m| (m.score, m.member.as_str()))
                    .collect();
                pipe.zadd_multiple(key, &items);
            }
            RedisCommand::ZRem { key, members } => {
                pipe.zrem(key, members);
            }
            RedisCommand::ZIncrBy { key, delta, member } => {
                pipe.zincr(key, member, *delta);
            }
            RedisCommand::ZScore { key, member } => {
                pipe.zscore(key, member);
            }
            RedisCommand::ZRank { key, member } => {
                pipe.zrank(key, member);
            }
            RedisCommand::ZRevRank { key, member } => {
                pipe.zrevrank(key, member);
            }
            RedisCommand::ZCard { key } => {
                pipe.zcard(key);
            }
            RedisCommand::ZCount { key, min, max } => {
                pipe.zcount(key, min, max);
            }
            RedisCommand::ZRange {
                key,
                start,
                stop,
                with_scores,
            } => {
                let mut cmd = redis::cmd("ZRANGE");
                cmd.arg(key).arg(*start).arg(*stop);
                if *with_scores {
                    cmd.arg("WITHSCORES");
                }
                pipe.add_command(cmd);
            }
            RedisCommand::ZRevRange {
                key,
                start,
                stop,
                with_scores,
            } => {
                let mut cmd = redis::cmd("ZREVRANGE");
                cmd.arg(key).arg(*start).arg(*stop);
                if *with_scores {
                    cmd.arg("WITHSCORES");
                }
                pipe.add_command(cmd);
            }

            // Key commands
            RedisCommand::Del { keys } => {
                pipe.del(keys);
            }
            RedisCommand::Exists { keys } => {
                pipe.exists(keys);
            }
            RedisCommand::Expire { key, seconds } => {
                pipe.expire(key, *seconds as i64);
            }
            RedisCommand::PExpire { key, milliseconds } => {
                pipe.pexpire(key, *milliseconds as i64);
            }
            RedisCommand::Ttl { key } => {
                pipe.ttl(key);
            }
            RedisCommand::PTtl { key } => {
                pipe.pttl(key);
            }
            RedisCommand::Persist { key } => {
                pipe.persist(key);
            }
            RedisCommand::Rename { key, new_key } => {
                pipe.rename(key, new_key);
            }
            RedisCommand::RenameNx { key, new_key } => {
                pipe.rename_nx(key, new_key);
            }
            RedisCommand::Type { key } => {
                pipe.key_type(key);
            }
        }
        Ok(())
    }

    /// Parse Redis values into CommandResults
    fn parse_results(commands: &[RedisCommand], results: Vec<redis::Value>) -> Vec<CommandResult> {
        commands
            .iter()
            .enumerate()
            .map(|(index, _cmd)| {
                let value = results.get(index).cloned();
                match value {
                    Some(redis::Value::Nil) => CommandResult {
                        index,
                        success: true,
                        value: Some(serde_json::Value::Null),
                        error: None,
                    },
                    Some(redis::Value::Int(i)) => CommandResult {
                        index,
                        success: true,
                        value: Some(serde_json::Value::Number(i.into())),
                        error: None,
                    },
                    Some(redis::Value::BulkString(bytes)) => {
                        let s = String::from_utf8_lossy(&bytes).to_string();
                        CommandResult {
                            index,
                            success: true,
                            value: Some(serde_json::Value::String(s)),
                            error: None,
                        }
                    }
                    Some(redis::Value::SimpleString(s)) => CommandResult {
                        index,
                        success: true,
                        value: Some(serde_json::Value::String(s)),
                        error: None,
                    },
                    Some(redis::Value::Array(arr)) => {
                        let parsed = Self::parse_array_value(&arr);
                        CommandResult {
                            index,
                            success: true,
                            value: Some(parsed),
                            error: None,
                        }
                    }
                    Some(redis::Value::Double(d)) => CommandResult {
                        index,
                        success: true,
                        value: Some(serde_json::json!(d)),
                        error: None,
                    },
                    Some(redis::Value::Boolean(b)) => CommandResult {
                        index,
                        success: true,
                        value: Some(serde_json::Value::Bool(b)),
                        error: None,
                    },
                    Some(redis::Value::Okay) => CommandResult {
                        index,
                        success: true,
                        value: Some(serde_json::Value::String("OK".to_string())),
                        error: None,
                    },
                    Some(redis::Value::ServerError(err)) => CommandResult {
                        index,
                        success: false,
                        value: None,
                        error: Some(format!("{:?}", err)),
                    },
                    None => CommandResult {
                        index,
                        success: false,
                        value: None,
                        error: Some("No result returned".to_string()),
                    },
                    _ => CommandResult {
                        index,
                        success: true,
                        value: Some(serde_json::Value::Null),
                        error: None,
                    },
                }
            })
            .collect()
    }

    /// Parse a Redis array value into JSON
    fn parse_array_value(arr: &[redis::Value]) -> serde_json::Value {
        let parsed: Vec<serde_json::Value> = arr
            .iter()
            .map(|v| match v {
                redis::Value::Nil => serde_json::Value::Null,
                redis::Value::Int(i) => serde_json::Value::Number((*i).into()),
                redis::Value::BulkString(bytes) => {
                    serde_json::Value::String(String::from_utf8_lossy(bytes).to_string())
                }
                redis::Value::SimpleString(s) => serde_json::Value::String(s.clone()),
                redis::Value::Double(d) => serde_json::json!(*d),
                redis::Value::Boolean(b) => serde_json::Value::Bool(*b),
                redis::Value::Array(inner) => Self::parse_array_value(inner),
                redis::Value::Okay => serde_json::Value::String("OK".to_string()),
                _ => serde_json::Value::Null,
            })
            .collect();
        serde_json::Value::Array(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http::schemas::transactions::{FieldValue, KeyValue, ScoredMember};
    use crate::infrastructure::redis::connection::InstrumentedPool;
    use testcontainers::ContainerAsync;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::redis::{REDIS_PORT, Redis};
    use tokio::sync::Notify;

    async fn start_redis() -> (ContainerAsync<Redis>, String) {
        let container = Redis::default().start().await.unwrap();
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
        let url = format!("redis://{host}:{port}");
        (container, url)
    }

    async fn service_with_redis() -> (ContainerAsync<Redis>, TransactionService, redis::Client) {
        let (container, redis_url) = start_redis().await;
        let pool = InstrumentedPool::new_for_tests_with_url(&redis_url).unwrap();
        let service = TransactionService::new(Arc::new(pool));
        let client = redis::Client::open(redis_url.as_str()).unwrap();
        (container, service, client)
    }

    #[test]
    fn test_parse_array_value() {
        let arr = vec![
            redis::Value::Int(1),
            redis::Value::BulkString(b"hello".to_vec()),
            redis::Value::Nil,
        ];
        let result = TransactionService::parse_array_value(&arr);
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], serde_json::json!(1));
        assert_eq!(arr[1], serde_json::json!("hello"));
        assert!(arr[2].is_null());
    }

    #[test]
    fn test_parse_results_with_various_types() {
        let commands = vec![
            RedisCommand::Get {
                key: "test".to_string(),
            },
            RedisCommand::Incr {
                key: "counter".to_string(),
            },
            RedisCommand::Get {
                key: "missing".to_string(),
            },
        ];

        let results = vec![
            redis::Value::BulkString(b"value".to_vec()),
            redis::Value::Int(42),
            redis::Value::Nil,
        ];

        let parsed = TransactionService::parse_results(&commands, results);
        assert_eq!(parsed.len(), 3);

        assert!(parsed[0].success);
        assert_eq!(parsed[0].value, Some(serde_json::json!("value")));

        assert!(parsed[1].success);
        assert_eq!(parsed[1].value, Some(serde_json::json!(42)));

        assert!(parsed[2].success);
        assert!(parsed[2].value.as_ref().unwrap().is_null());
    }

    #[test]
    fn test_parse_results_additional_types() {
        let commands = vec![
            RedisCommand::Get {
                key: "a".to_string(),
            },
            RedisCommand::Get {
                key: "b".to_string(),
            },
            RedisCommand::Get {
                key: "c".to_string(),
            },
            RedisCommand::Get {
                key: "d".to_string(),
            },
            RedisCommand::Get {
                key: "e".to_string(),
            },
            RedisCommand::Get {
                key: "f".to_string(),
            },
        ];

        let server_error = redis::parse_redis_value(b"-ERR oops\r\n").unwrap();
        let results = vec![
            redis::Value::SimpleString("ok".to_string()),
            redis::Value::Double(1.5),
            redis::Value::Boolean(true),
            redis::Value::Okay,
            server_error,
        ];

        let parsed = TransactionService::parse_results(&commands, results);
        assert_eq!(parsed.len(), 6);
        assert_eq!(parsed[0].value, Some(serde_json::json!("ok")));
        assert_eq!(parsed[1].value, Some(serde_json::json!(1.5)));
        assert_eq!(parsed[2].value, Some(serde_json::json!(true)));
        assert_eq!(parsed[3].value, Some(serde_json::json!("OK")));
        assert!(!parsed[4].success);
        assert!(parsed[4].error.as_ref().unwrap().contains("ResponseError"));
        assert!(!parsed[5].success);
        assert!(parsed[5].error.as_ref().unwrap().contains("No result"));
    }

    #[test]
    fn test_parse_results_array_and_unknown() {
        let commands = vec![
            RedisCommand::Get {
                key: "a".to_string(),
            },
            RedisCommand::Get {
                key: "b".to_string(),
            },
        ];

        let results = vec![
            redis::Value::Array(vec![
                redis::Value::Int(1),
                redis::Value::BulkString(b"two".to_vec()),
            ]),
            redis::Value::Map(Vec::new()),
        ];

        let parsed = TransactionService::parse_results(&commands, results);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].value, Some(serde_json::json!([1, "two"])));
        assert!(parsed[1].value.as_ref().unwrap().is_null());
    }

    #[test]
    fn test_parse_array_value_additional_types() {
        let arr = vec![
            redis::Value::SimpleString("ok".to_string()),
            redis::Value::Double(1.5),
            redis::Value::Boolean(true),
            redis::Value::Array(vec![redis::Value::Int(3)]),
            redis::Value::Okay,
        ];

        let parsed = TransactionService::parse_array_value(&arr);
        assert_eq!(parsed, serde_json::json!(["ok", 1.5, true, [3], "OK"]));
    }

    #[test]
    fn test_add_command_to_pipe_all_variants() {
        let mut pipe = redis::pipe();
        let commands = vec![
            RedisCommand::Get {
                key: "k".to_string(),
            },
            RedisCommand::Set {
                key: "k".to_string(),
                value: "v".to_string(),
                ttl_seconds: Some(5),
            },
            RedisCommand::Set {
                key: "k2".to_string(),
                value: "v2".to_string(),
                ttl_seconds: None,
            },
            RedisCommand::Incr {
                key: "k".to_string(),
            },
            RedisCommand::IncrBy {
                key: "k".to_string(),
                delta: 2,
            },
            RedisCommand::Decr {
                key: "k".to_string(),
            },
            RedisCommand::DecrBy {
                key: "k".to_string(),
                delta: 3,
            },
            RedisCommand::Append {
                key: "k".to_string(),
                value: "v".to_string(),
            },
            RedisCommand::SetNx {
                key: "k".to_string(),
                value: "v".to_string(),
            },
            RedisCommand::GetSet {
                key: "k".to_string(),
                value: "v".to_string(),
            },
            RedisCommand::MGet {
                keys: vec!["k1".to_string(), "k2".to_string()],
            },
            RedisCommand::MSet {
                entries: vec![
                    KeyValue {
                        key: "k1".to_string(),
                        value: "v1".to_string(),
                    },
                    KeyValue {
                        key: "k2".to_string(),
                        value: "v2".to_string(),
                    },
                ],
            },
            RedisCommand::HGet {
                key: "h".to_string(),
                field: "f".to_string(),
            },
            RedisCommand::HSet {
                key: "h".to_string(),
                field: "f".to_string(),
                value: "v".to_string(),
            },
            RedisCommand::HMSet {
                key: "h".to_string(),
                fields: vec![FieldValue {
                    field: "f".to_string(),
                    value: "v".to_string(),
                }],
            },
            RedisCommand::HMGet {
                key: "h".to_string(),
                fields: vec!["f".to_string()],
            },
            RedisCommand::HIncrBy {
                key: "h".to_string(),
                field: "f".to_string(),
                delta: 1,
            },
            RedisCommand::HIncrByFloat {
                key: "h".to_string(),
                field: "f".to_string(),
                delta: 1.5,
            },
            RedisCommand::HDel {
                key: "h".to_string(),
                fields: vec!["f".to_string()],
            },
            RedisCommand::HExists {
                key: "h".to_string(),
                field: "f".to_string(),
            },
            RedisCommand::HGetAll {
                key: "h".to_string(),
            },
            RedisCommand::HKeys {
                key: "h".to_string(),
            },
            RedisCommand::HVals {
                key: "h".to_string(),
            },
            RedisCommand::HLen {
                key: "h".to_string(),
            },
            RedisCommand::HSetNx {
                key: "h".to_string(),
                field: "f".to_string(),
                value: "v".to_string(),
            },
            RedisCommand::LPush {
                key: "l".to_string(),
                values: vec!["v".to_string()],
            },
            RedisCommand::RPush {
                key: "l".to_string(),
                values: vec!["v".to_string()],
            },
            RedisCommand::LPop {
                key: "l".to_string(),
                count: None,
            },
            RedisCommand::LPop {
                key: "l".to_string(),
                count: Some(1),
            },
            RedisCommand::RPop {
                key: "l".to_string(),
                count: None,
            },
            RedisCommand::RPop {
                key: "l".to_string(),
                count: Some(1),
            },
            RedisCommand::LLen {
                key: "l".to_string(),
            },
            RedisCommand::LIndex {
                key: "l".to_string(),
                index: 0,
            },
            RedisCommand::LRange {
                key: "l".to_string(),
                start: 0,
                stop: -1,
            },
            RedisCommand::LSet {
                key: "l".to_string(),
                index: 0,
                value: "v".to_string(),
            },
            RedisCommand::LTrim {
                key: "l".to_string(),
                start: 0,
                stop: 1,
            },
            RedisCommand::LRem {
                key: "l".to_string(),
                count: 1,
                value: "v".to_string(),
            },
            RedisCommand::SAdd {
                key: "s".to_string(),
                members: vec!["m".to_string()],
            },
            RedisCommand::SRem {
                key: "s".to_string(),
                members: vec!["m".to_string()],
            },
            RedisCommand::SIsMember {
                key: "s".to_string(),
                member: "m".to_string(),
            },
            RedisCommand::SMembers {
                key: "s".to_string(),
            },
            RedisCommand::SCard {
                key: "s".to_string(),
            },
            RedisCommand::SPop {
                key: "s".to_string(),
                count: None,
            },
            RedisCommand::SPop {
                key: "s".to_string(),
                count: Some(2),
            },
            RedisCommand::SMove {
                source: "s1".to_string(),
                destination: "s2".to_string(),
                member: "m".to_string(),
            },
            RedisCommand::ZAdd {
                key: "z".to_string(),
                members: vec![ScoredMember {
                    score: 1.0,
                    member: "m".to_string(),
                }],
            },
            RedisCommand::ZRem {
                key: "z".to_string(),
                members: vec!["m".to_string()],
            },
            RedisCommand::ZIncrBy {
                key: "z".to_string(),
                delta: 1.0,
                member: "m".to_string(),
            },
            RedisCommand::ZScore {
                key: "z".to_string(),
                member: "m".to_string(),
            },
            RedisCommand::ZRank {
                key: "z".to_string(),
                member: "m".to_string(),
            },
            RedisCommand::ZRevRank {
                key: "z".to_string(),
                member: "m".to_string(),
            },
            RedisCommand::ZCard {
                key: "z".to_string(),
            },
            RedisCommand::ZCount {
                key: "z".to_string(),
                min: "-inf".to_string(),
                max: "+inf".to_string(),
            },
            RedisCommand::ZRange {
                key: "z".to_string(),
                start: 0,
                stop: -1,
                with_scores: false,
            },
            RedisCommand::ZRange {
                key: "z".to_string(),
                start: 0,
                stop: -1,
                with_scores: true,
            },
            RedisCommand::ZRevRange {
                key: "z".to_string(),
                start: 0,
                stop: -1,
                with_scores: false,
            },
            RedisCommand::ZRevRange {
                key: "z".to_string(),
                start: 0,
                stop: -1,
                with_scores: true,
            },
            RedisCommand::Del {
                keys: vec!["k".to_string()],
            },
            RedisCommand::Exists {
                keys: vec!["k".to_string()],
            },
            RedisCommand::Expire {
                key: "k".to_string(),
                seconds: 10,
            },
            RedisCommand::PExpire {
                key: "k".to_string(),
                milliseconds: 1000,
            },
            RedisCommand::Ttl {
                key: "k".to_string(),
            },
            RedisCommand::PTtl {
                key: "k".to_string(),
            },
            RedisCommand::Persist {
                key: "k".to_string(),
            },
            RedisCommand::Rename {
                key: "k".to_string(),
                new_key: "k2".to_string(),
            },
            RedisCommand::RenameNx {
                key: "k".to_string(),
                new_key: "k3".to_string(),
            },
            RedisCommand::Type {
                key: "k".to_string(),
            },
        ];

        for cmd in commands {
            assert!(TransactionService::add_command_to_pipe(&mut pipe, &cmd).is_ok());
        }
    }

    #[test]
    fn test_add_command_to_pipe_invalid_pop_counts() {
        let mut pipe = redis::pipe();
        let lpop = RedisCommand::LPop {
            key: "l".to_string(),
            count: Some(0),
        };
        let rpop = RedisCommand::RPop {
            key: "l".to_string(),
            count: Some(0),
        };
        let err = TransactionService::add_command_to_pipe(&mut pipe, &lpop).unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
        let err = TransactionService::add_command_to_pipe(&mut pipe, &rpop).unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[test]
    fn test_parse_compare_and_set_result_simple_string() {
        let result = vec![
            redis::Value::Int(1),
            redis::Value::SimpleString("OK".to_string()),
        ];
        let parsed = TransactionService::parse_compare_and_set_result(&result);
        assert!(parsed.swapped);
        assert_eq!(parsed.current_value.as_deref(), Some("OK"));
    }

    #[test]
    fn test_map_script_error_variants() {
        let noscript = redis::RedisError::from((redis::ErrorKind::NoScriptError, "NOSCRIPT"));
        assert!(matches!(
            TransactionService::map_script_error(noscript),
            CacheError::ScriptError(_)
        ));

        let extension = redis::RedisError::from((redis::ErrorKind::ExtensionError, "ERR"));
        assert!(matches!(
            TransactionService::map_script_error(extension),
            CacheError::ScriptError(_)
        ));

        let response = redis::RedisError::from((
            redis::ErrorKind::ResponseError,
            "ERR",
            "NOSCRIPT missing".to_string(),
        ));
        assert!(matches!(
            TransactionService::map_script_error(response),
            CacheError::ScriptError(_)
        ));

        let other = redis::RedisError::from((redis::ErrorKind::ResponseError, "WRONGTYPE"));
        assert!(matches!(
            TransactionService::map_script_error(other),
            CacheError::RedisError(_)
        ));

        let io_err = redis::RedisError::from((redis::ErrorKind::IoError, "io"));
        assert!(matches!(
            TransactionService::map_script_error(io_err),
            CacheError::ConnectionFailed(_)
        ));
    }

    #[test]
    fn test_transaction_service_creation() {
        use crate::infrastructure::redis::connection::InstrumentedPool;
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = TransactionService::new(pool);
    }

    #[tokio::test]
    async fn test_transaction_command_limit_validation() {
        use crate::infrastructure::redis::connection::InstrumentedPool;
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = TransactionService::new(pool);

        // Create request with too many commands (> 100)
        let commands: Vec<RedisCommand> = (0..101)
            .map(|i| RedisCommand::Get {
                key: format!("key{}", i),
            })
            .collect();

        let request = TransactionRequest {
            watch_keys: None,
            commands,
        };

        let result = service.execute(request).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CacheError::InvalidInput(msg) => {
                assert!(msg.contains("100"));
                assert!(msg.contains("101"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[tokio::test]
    async fn test_transaction_watch_key_limit_validation() {
        use crate::infrastructure::redis::connection::InstrumentedPool;
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = TransactionService::new(pool);

        // Create request with too many watch keys (> 20)
        let watch_keys: Vec<String> = (0..21).map(|i| format!("watch{}", i)).collect();

        let request = TransactionRequest {
            watch_keys: Some(watch_keys),
            commands: vec![RedisCommand::Get {
                key: "test".to_string(),
            }],
        };

        let result = service.execute(request).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CacheError::InvalidInput(msg) => {
                assert!(msg.contains("20"));
                assert!(msg.contains("21"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[tokio::test]
    async fn test_transaction_empty_commands_validation() {
        use crate::infrastructure::redis::connection::InstrumentedPool;
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = TransactionService::new(pool);

        let request = TransactionRequest {
            watch_keys: None,
            commands: vec![],
        };

        let result = service.execute(request).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CacheError::InvalidInput(msg) => {
                assert!(msg.contains("At least one command"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[tokio::test]
    async fn test_execute_timeout_precheck() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = TransactionService::new(pool);
        let request = TransactionRequest {
            watch_keys: None,
            commands: vec![RedisCommand::Get {
                key: "k".to_string(),
            }],
        };
        let result = service
            .execute_with_deadline(request, Instant::now() - Duration::from_secs(1))
            .await;
        assert!(matches!(result, Err(CacheError::Timeout)));
    }

    #[tokio::test]
    async fn test_execute_timeout_postcheck() {
        let (_container, service, _client) = service_with_redis().await;
        set_test_execute_delay_ms(20);
        let request = TransactionRequest {
            watch_keys: None,
            commands: vec![RedisCommand::Get {
                key: "k".to_string(),
            }],
        };
        let result = service
            .execute_with_deadline(request, Instant::now() + Duration::from_millis(5))
            .await;
        set_test_execute_delay_ms(0);
        assert!(matches!(result, Err(CacheError::Timeout)));
    }

    #[tokio::test]
    async fn test_execute_watch_key_empty() {
        let (_container, service, _client) = service_with_redis().await;
        let request = TransactionRequest {
            watch_keys: Some(vec!["".to_string()]),
            commands: vec![RedisCommand::Get {
                key: "k".to_string(),
            }],
        };
        let result = service.execute(request).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_execute_unwatch_on_add_command_error() {
        let (_container, service, _client) = service_with_redis().await;
        let request = TransactionRequest {
            watch_keys: Some(vec!["watched".to_string()]),
            commands: vec![RedisCommand::LPop {
                key: "list".to_string(),
                count: Some(0),
            }],
        };
        let result = service.execute(request).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_execute_watch_abort() {
        let (_container, service, client) = service_with_redis().await;
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: () = redis::cmd("SET")
            .arg("watched")
            .arg("1")
            .query_async(&mut conn)
            .await
            .unwrap();

        let started = Arc::new(Notify::new());
        let proceed = Arc::new(Notify::new());
        set_watch_hooks(started.clone(), proceed.clone());

        let request = TransactionRequest {
            watch_keys: Some(vec!["watched".to_string()]),
            commands: vec![RedisCommand::Get {
                key: "watched".to_string(),
            }],
        };
        let service = Arc::new(service);
        let handle = tokio::spawn({
            let service = service.clone();
            async move { service.execute(request).await }
        });

        started.notified().await;
        let _: () = redis::cmd("SET")
            .arg("watched")
            .arg("2")
            .query_async(&mut conn)
            .await
            .unwrap();
        proceed.notify_one();

        let result = handle.await.unwrap();
        clear_watch_hooks();
        assert!(matches!(result, Err(CacheError::TransactionAborted)));
    }

    #[tokio::test]
    async fn test_handle_exec_error_execabort() {
        let (_container, service, _client) = service_with_redis().await;
        let mut conn = service.pool.get().await.unwrap();
        let err = redis::RedisError::from((
            redis::ErrorKind::ResponseError,
            "EXECABORT Transaction discarded because of previous errors.",
        ));
        let result = TransactionService::handle_exec_error(&mut conn, err).await;
        assert!(matches!(result, Err(CacheError::TransactionAborted)));
    }

    #[tokio::test]
    async fn test_execute_execabort_error() {
        let (_container, service, _client) = service_with_redis().await;
        let request = TransactionRequest {
            watch_keys: None,
            commands: vec![RedisCommand::MSet { entries: vec![] }],
        };
        let result = service.execute(request).await;
        match result {
            Err(CacheError::TransactionAborted) => {}
            Err(CacheError::RedisError(err)) => {
                let message = err.to_string();
                assert!(
                    message.contains("EXECABORT") || message.contains("wrong number of arguments"),
                    "unexpected redis error: {message}"
                );
            }
            other => panic!("expected transaction abort or redis error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_compare_and_set_variants() {
        let (_container, service, client) = service_with_redis().await;
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: () = redis::cmd("SET")
            .arg("version")
            .arg("1")
            .query_async(&mut conn)
            .await
            .unwrap();

        let request = CompareAndSetRequest {
            key: "version".to_string(),
            expected_value: "1".to_string(),
            new_value: "2".to_string(),
        };
        let result = service.compare_and_set(request).await.unwrap();
        assert!(result.swapped);
        assert_eq!(result.current_value.as_deref(), Some("2"));

        let request = CompareAndSetRequest {
            key: "version".to_string(),
            expected_value: "nope".to_string(),
            new_value: "3".to_string(),
        };
        let result = service.compare_and_set(request).await.unwrap();
        assert!(!result.swapped);
        assert_eq!(result.current_value.as_deref(), Some("2"));

        let request = CompareAndSetRequest {
            key: "missing".to_string(),
            expected_value: "x".to_string(),
            new_value: "y".to_string(),
        };
        let result = service.compare_and_set(request).await.unwrap();
        assert!(!result.swapped);
        assert!(result.current_value.is_none());
    }

    #[tokio::test]
    async fn test_compare_and_set_empty_key() {
        let (_container, service, _client) = service_with_redis().await;
        let request = CompareAndSetRequest {
            key: "".to_string(),
            expected_value: "1".to_string(),
            new_value: "2".to_string(),
        };
        let result = service.compare_and_set(request).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_hcompare_and_set_variants() {
        let (_container, service, client) = service_with_redis().await;
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: () = redis::cmd("HSET")
            .arg("user:1")
            .arg("version")
            .arg("1")
            .query_async(&mut conn)
            .await
            .unwrap();

        let request = HCompareAndSetRequest {
            key: "user:1".to_string(),
            field: "version".to_string(),
            expected_value: "1".to_string(),
            new_value: "2".to_string(),
        };
        let result = service.hcompare_and_set(request).await.unwrap();
        assert!(result.swapped);
        assert_eq!(result.current_value.as_deref(), Some("2"));

        let request = HCompareAndSetRequest {
            key: "user:1".to_string(),
            field: "version".to_string(),
            expected_value: "nope".to_string(),
            new_value: "3".to_string(),
        };
        let result = service.hcompare_and_set(request).await.unwrap();
        assert!(!result.swapped);
        assert_eq!(result.current_value.as_deref(), Some("2"));

        let request = HCompareAndSetRequest {
            key: "user:1".to_string(),
            field: "missing".to_string(),
            expected_value: "x".to_string(),
            new_value: "y".to_string(),
        };
        let result = service.hcompare_and_set(request).await.unwrap();
        assert!(!result.swapped);
        assert!(result.current_value.is_none());
    }

    #[tokio::test]
    async fn test_hcompare_and_set_empty_key() {
        let (_container, service, _client) = service_with_redis().await;
        let request = HCompareAndSetRequest {
            key: "".to_string(),
            field: "f".to_string(),
            expected_value: "1".to_string(),
            new_value: "2".to_string(),
        };
        let result = service.hcompare_and_set(request).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_hcompare_and_set_empty_field() {
        let (_container, service, _client) = service_with_redis().await;
        let request = HCompareAndSetRequest {
            key: "k".to_string(),
            field: "".to_string(),
            expected_value: "1".to_string(),
            new_value: "2".to_string(),
        };
        let result = service.hcompare_and_set(request).await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::api::http::schemas::transactions::{
        CompareAndSetRequest, HCompareAndSetRequest, RedisCommand, TransactionRequest,
    };
    use crate::infrastructure::redis::connection::InstrumentedPool;
    use testcontainers::ContainerAsync;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::redis::{REDIS_PORT, Redis};

    async fn start_redis() -> (ContainerAsync<Redis>, String) {
        let container = Redis::default().start().await.unwrap();
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
        let url = format!("redis://{host}:{port}");
        (container, url)
    }

    async fn service_with_redis() -> (ContainerAsync<Redis>, TransactionService, redis::Client) {
        let (container, redis_url) = start_redis().await;
        let pool = InstrumentedPool::new_for_tests_with_url(&redis_url).unwrap();
        let service = TransactionService::new(Arc::new(pool));
        let client = redis::Client::open(redis_url.as_str()).unwrap();
        (container, service, client)
    }

    #[tokio::test]
    async fn test_execute_simple_transaction() {
        let (_container, service, _client) = service_with_redis().await;

        let request = TransactionRequest {
            commands: vec![
                RedisCommand::Set {
                    key: "txn_key".to_string(),
                    value: "hello".to_string(),
                    ttl_seconds: None,
                },
                RedisCommand::Get {
                    key: "txn_key".to_string(),
                },
            ],
            watch_keys: None,
        };

        let response = service.execute(request).await.unwrap();
        assert!(response.success);
        assert_eq!(response.results.len(), 2);

        // SET returns OK
        assert!(response.results[0].success);

        // GET returns the value we set
        assert!(response.results[1].success);
        assert_eq!(response.results[1].value, Some(serde_json::json!("hello")));
    }

    #[tokio::test]
    async fn test_compare_and_set_success() {
        let (_container, service, client) = service_with_redis().await;

        // Pre-set the key using a direct Redis connection
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: () = redis::cmd("SET")
            .arg("cas_key")
            .arg("old_value")
            .query_async(&mut conn)
            .await
            .unwrap();

        // CAS with matching expected value should succeed
        let request = CompareAndSetRequest {
            key: "cas_key".to_string(),
            expected_value: "old_value".to_string(),
            new_value: "new_value".to_string(),
        };
        let result = service.compare_and_set(request).await.unwrap();
        assert!(result.swapped);
        assert_eq!(result.current_value.as_deref(), Some("new_value"));
    }

    #[tokio::test]
    async fn test_compare_and_set_mismatch() {
        let (_container, service, client) = service_with_redis().await;

        // Pre-set the key
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: () = redis::cmd("SET")
            .arg("cas_key2")
            .arg("actual_value")
            .query_async(&mut conn)
            .await
            .unwrap();

        // CAS with wrong expected value should fail
        let request = CompareAndSetRequest {
            key: "cas_key2".to_string(),
            expected_value: "wrong_value".to_string(),
            new_value: "new_value".to_string(),
        };
        let result = service.compare_and_set(request).await.unwrap();
        assert!(!result.swapped);
        assert_eq!(result.current_value.as_deref(), Some("actual_value"));
    }

    #[tokio::test]
    async fn test_execute_with_watch_keys_success() {
        let (_container, service, client) = service_with_redis().await;

        // Pre-set the key directly
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: () = redis::cmd("SET")
            .arg("watched_key")
            .arg("initial")
            .query_async(&mut conn)
            .await
            .unwrap();

        // Execute a transaction that WATCHes the key and does GET + SET.
        // Since no one modifies the watched key between WATCH and EXEC,
        // the transaction should succeed.
        let request = TransactionRequest {
            watch_keys: Some(vec!["watched_key".to_string()]),
            commands: vec![
                RedisCommand::Get {
                    key: "watched_key".to_string(),
                },
                RedisCommand::Set {
                    key: "watched_key".to_string(),
                    value: "updated".to_string(),
                    ttl_seconds: None,
                },
            ],
        };

        let response = service.execute(request).await.unwrap();
        assert!(response.success);
        assert_eq!(response.results.len(), 2);

        // GET should return the initial value
        assert!(response.results[0].success);
        assert_eq!(
            response.results[0].value,
            Some(serde_json::json!("initial"))
        );

        // SET should succeed
        assert!(response.results[1].success);
    }

    #[tokio::test]
    async fn test_execute_multi_command_types() {
        let (_container, service, _client) = service_with_redis().await;

        // Execute a transaction with multiple command types: SET, LPUSH, SADD
        let request = TransactionRequest {
            watch_keys: None,
            commands: vec![
                RedisCommand::Set {
                    key: "multi_str".to_string(),
                    value: "hello".to_string(),
                    ttl_seconds: None,
                },
                RedisCommand::LPush {
                    key: "multi_list".to_string(),
                    values: vec!["a".to_string(), "b".to_string()],
                },
                RedisCommand::SAdd {
                    key: "multi_set".to_string(),
                    members: vec!["x".to_string(), "y".to_string(), "z".to_string()],
                },
            ],
        };

        let response = service.execute(request).await.unwrap();
        assert!(response.success);
        assert_eq!(response.results.len(), 3);

        // All commands should succeed
        assert!(response.results[0].success); // SET -> OK
        assert!(response.results[1].success); // LPUSH -> count of elements
        assert!(response.results[2].success); // SADD -> count of elements added
    }

    #[tokio::test]
    async fn test_hcompare_and_set_success() {
        let (_container, service, client) = service_with_redis().await;

        // Pre-set a hash field directly
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: () = redis::cmd("HSET")
            .arg("hcas_hash")
            .arg("field1")
            .arg("old_val")
            .query_async(&mut conn)
            .await
            .unwrap();

        // hcompare_and_set with matching expected value should succeed
        let request = HCompareAndSetRequest {
            key: "hcas_hash".to_string(),
            field: "field1".to_string(),
            expected_value: "old_val".to_string(),
            new_value: "new_val".to_string(),
        };
        let result = service.hcompare_and_set(request).await.unwrap();
        assert!(result.swapped);
        assert_eq!(result.current_value.as_deref(), Some("new_val"));
    }

    #[tokio::test]
    async fn test_hcompare_and_set_mismatch() {
        let (_container, service, client) = service_with_redis().await;

        // Pre-set a hash field directly
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: () = redis::cmd("HSET")
            .arg("hcas_hash2")
            .arg("field1")
            .arg("actual_val")
            .query_async(&mut conn)
            .await
            .unwrap();

        // hcompare_and_set with wrong expected value should fail
        let request = HCompareAndSetRequest {
            key: "hcas_hash2".to_string(),
            field: "field1".to_string(),
            expected_value: "wrong_val".to_string(),
            new_value: "new_val".to_string(),
        };
        let result = service.hcompare_and_set(request).await.unwrap();
        assert!(!result.swapped);
        assert_eq!(result.current_value.as_deref(), Some("actual_val"));
    }
}
