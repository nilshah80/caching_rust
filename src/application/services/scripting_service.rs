//! Scripting Service
//!
//! Business logic for Redis Lua scripting operations.
//! Provides secure execution of Lua scripts with proper argument handling.
//!
//! # Security Considerations
//!
//! - Scripts can execute arbitrary Redis commands
//! - Use EVAL_RO/EVALSHA_RO for read-only operations to prevent writes
//! - Consider implementing script allowlisting in production
//! - Maximum script execution time is controlled by Redis's lua-time-limit
//!
//! # Examples
//!
//! ## Basic Script Evaluation
//!
//! ```json
//! POST /api/v1/scripts/eval
//! {
//!   "script": "return redis.call('GET', KEYS[1])",
//!   "keys": ["user:1"]
//! }
//! ```
//!
//! ## Cached Script Execution
//!
//! ```json
//! POST /api/v1/scripts/evalsha
//! {
//!   "sha": "6b1bf486c81ceb7edf3c093f4a73d3e117c0b169",
//!   "keys": ["user:1"],
//!   "args": ["value1"]
//! }
//! ```

use std::sync::Arc;

use crate::api::http::schemas::scripting::{
    EvalRequest, EvalResponse, EvalShaRequest, FlushMode, ScriptDebugMode, ScriptDebugRequest,
    ScriptDebugResponse, ScriptExistsRequest, ScriptExistsResponse, ScriptExistsResult,
    ScriptFlushRequest, ScriptFlushResponse, ScriptKillResponse, ScriptLoadRequest,
    ScriptLoadResponse,
};
use crate::domain::errors::CacheError;
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Maximum number of keys allowed per script execution
const MAX_SCRIPT_KEYS: usize = 1000;

/// Maximum number of arguments allowed per script execution
const MAX_SCRIPT_ARGS: usize = 1000;

/// Maximum script size in bytes (1MB)
const MAX_SCRIPT_SIZE: usize = 1024 * 1024;

/// Service for Redis Lua scripting operations
pub struct ScriptingService {
    pool: Arc<InstrumentedPool>,
}

impl ScriptingService {
    /// Create a new ScriptingService with the given connection pool
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }

    /// Map Redis errors to appropriate CacheError types.
    /// Script execution errors (Lua errors, NOSCRIPT, NOTBUSY, UNKILLABLE) -> ScriptError (400)
    /// Connection/transport errors -> appropriate 5xx errors
    fn map_redis_error(e: redis::RedisError) -> CacheError {
        use redis::ErrorKind;
        match e.kind() {
            // Script-specific errors -> 400
            ErrorKind::NoScriptError => CacheError::ScriptError(
                "Script not found in cache. Use SCRIPT LOAD first.".to_string(),
            ),
            // NotBusy error from SCRIPT KILL when no script is running
            ErrorKind::NotBusy => {
                CacheError::ScriptError("No script is currently running".to_string())
            }
            // Extension errors include Lua runtime errors
            ErrorKind::ExtensionError => CacheError::ScriptError(format!("Script error: {}", e)),
            // Response errors from Redis (including Lua errors returned by Redis)
            ErrorKind::ResponseError => {
                let msg = e.to_string();
                // Check if it's a script-related error
                if msg.contains("NOSCRIPT")
                    || msg.contains("ERR Error")
                    || msg.contains("@user_script")
                {
                    CacheError::ScriptError(format!("Script error: {}", e))
                } else if msg.contains("NOTBUSY") || msg.contains("No scripts in execution") {
                    CacheError::ScriptError("No script is currently running".to_string())
                } else if msg.contains("UNKILLABLE") {
                    CacheError::ScriptError(
                        "Cannot kill script that has performed writes".to_string(),
                    )
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

    /// Evaluate a Lua script.
    ///
    /// Executes the provided Lua script with the given keys and arguments.
    /// If `readonly` is true, uses EVAL_RO which prevents write operations.
    ///
    /// # Arguments
    /// * `request` - The eval request containing script, keys, args, and readonly flag
    ///
    /// # Returns
    /// The script execution result as a JSON value
    ///
    /// # Errors
    /// - `CacheError::InvalidInput` - Invalid script, too many keys/args
    /// - `CacheError::ScriptError` - Lua script execution error
    /// - `CacheError::PoolError` - Connection pool error
    pub async fn eval(&self, request: EvalRequest) -> Result<EvalResponse, CacheError> {
        // Validate input
        self.validate_script(&request.script)?;
        self.validate_keys(&request.keys)?;
        self.validate_args(&request.args)?;

        let mut conn = self.pool.get().await?;

        // Build the command based on readonly flag
        let cmd_name = if request.readonly { "EVAL_RO" } else { "EVAL" };

        let mut cmd = redis::cmd(cmd_name);
        cmd.arg(&request.script).arg(request.keys.len());

        // Add keys
        for key in &request.keys {
            cmd.arg(key);
        }

        // Add args (convert JSON values to Redis-compatible strings)
        for arg in &request.args {
            cmd.arg(Self::json_to_redis_arg(arg));
        }

        let result: redis::Value = cmd
            .query_async(&mut *conn)
            .await
            .map_err(Self::map_redis_error)?;

        Ok(EvalResponse {
            result: Self::redis_value_to_json(result),
        })
    }

    /// Evaluate a cached script by SHA1 hash.
    ///
    /// Executes a previously loaded script identified by its SHA1 hash.
    /// More efficient than EVAL as the script doesn't need to be transmitted.
    ///
    /// # Arguments
    /// * `request` - The evalsha request containing SHA, keys, args, and readonly flag
    ///
    /// # Returns
    /// The script execution result as a JSON value
    ///
    /// # Errors
    /// - `CacheError::InvalidInput` - Invalid SHA format
    /// - `CacheError::ScriptError` - Script not found or execution error
    pub async fn evalsha(&self, request: EvalShaRequest) -> Result<EvalResponse, CacheError> {
        // Validate SHA format
        self.validate_sha(&request.sha)?;
        self.validate_keys(&request.keys)?;
        self.validate_args(&request.args)?;

        let mut conn = self.pool.get().await?;

        // Build the command based on readonly flag
        let cmd_name = if request.readonly {
            "EVALSHA_RO"
        } else {
            "EVALSHA"
        };

        let mut cmd = redis::cmd(cmd_name);
        cmd.arg(&request.sha).arg(request.keys.len());

        // Add keys
        for key in &request.keys {
            cmd.arg(key);
        }

        // Add args
        for arg in &request.args {
            cmd.arg(Self::json_to_redis_arg(arg));
        }

        let result: redis::Value = cmd
            .query_async(&mut *conn)
            .await
            .map_err(Self::map_redis_error)?;

        Ok(EvalResponse {
            result: Self::redis_value_to_json(result),
        })
    }

    /// Load a script into the script cache.
    ///
    /// Caches the script on the Redis server and returns its SHA1 hash.
    /// The script can then be executed using EVALSHA with this hash.
    ///
    /// # Arguments
    /// * `request` - The script load request containing the script
    ///
    /// # Returns
    /// The SHA1 hash of the cached script
    pub async fn script_load(
        &self,
        request: ScriptLoadRequest,
    ) -> Result<ScriptLoadResponse, CacheError> {
        self.validate_script(&request.script)?;

        let mut conn = self.pool.get().await?;

        let sha: String = redis::cmd("SCRIPT")
            .arg("LOAD")
            .arg(&request.script)
            .query_async(&mut *conn)
            .await
            .map_err(Self::map_redis_error)?;

        Ok(ScriptLoadResponse { sha })
    }

    /// Check if scripts exist in the script cache.
    ///
    /// Returns existence status for each provided SHA1 hash.
    ///
    /// # Arguments
    /// * `request` - The request containing SHA hashes to check
    ///
    /// # Returns
    /// Existence status for each SHA
    pub async fn script_exists(
        &self,
        request: ScriptExistsRequest,
    ) -> Result<ScriptExistsResponse, CacheError> {
        if request.shas.is_empty() {
            return Err(CacheError::InvalidInput(
                "At least one SHA is required".to_string(),
            ));
        }

        // Validate all SHAs
        for sha in &request.shas {
            self.validate_sha(sha)?;
        }

        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("SCRIPT");
        cmd.arg("EXISTS");
        for sha in &request.shas {
            cmd.arg(sha);
        }

        let results: Vec<i64> = cmd
            .query_async(&mut *conn)
            .await
            .map_err(Self::map_redis_error)?;

        let response_results: Vec<ScriptExistsResult> = request
            .shas
            .iter()
            .zip(results.iter())
            .map(|(sha, &exists)| ScriptExistsResult {
                sha: sha.clone(),
                exists: exists == 1,
            })
            .collect();

        Ok(ScriptExistsResponse {
            results: response_results,
        })
    }

    /// Flush all scripts from the script cache.
    ///
    /// Removes all cached scripts from the Redis server.
    /// When no mode is specified, Redis uses its default behavior (SYNC for Redis 6.2+).
    /// Explicitly specify ASYNC for non-blocking or SYNC for blocking flush.
    ///
    /// # Arguments
    /// * `request` - Optional flush mode (ASYNC or SYNC). If not specified, Redis default is used.
    pub async fn script_flush(
        &self,
        request: ScriptFlushRequest,
    ) -> Result<ScriptFlushResponse, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("SCRIPT");
        cmd.arg("FLUSH");

        // Only add mode argument if explicitly specified, otherwise let Redis use its default
        if let Some(mode) = request.mode {
            match mode {
                FlushMode::Sync => cmd.arg("SYNC"),
                FlushMode::Async => cmd.arg("ASYNC"),
            };
        }

        cmd.query_async::<()>(&mut *conn)
            .await
            .map_err(Self::map_redis_error)?;

        Ok(ScriptFlushResponse { success: true })
    }

    /// Kill the currently executing script.
    ///
    /// Terminates a Lua script that is currently running.
    /// Only works if the script has not yet performed any writes.
    ///
    /// # Returns
    /// Success status
    ///
    /// # Errors
    /// - `CacheError::ScriptError` - No script running or script already performed writes
    pub async fn script_kill(&self) -> Result<ScriptKillResponse, CacheError> {
        let mut conn = self.pool.get().await?;

        redis::cmd("SCRIPT")
            .arg("KILL")
            .query_async::<()>(&mut *conn)
            .await
            .map_err(Self::map_redis_error)?;

        Ok(ScriptKillResponse { success: true })
    }

    /// Set the script debug mode.
    ///
    /// Controls the Lua script debugger for development/debugging purposes.
    /// This command should only be used in development environments.
    ///
    /// # Arguments
    /// * `request` - The debug mode to set (YES, SYNC, or NO)
    ///
    /// # Warning
    /// This is primarily for development use. In production, debugging should be disabled.
    pub async fn script_debug(
        &self,
        request: ScriptDebugRequest,
    ) -> Result<ScriptDebugResponse, CacheError> {
        let mut conn = self.pool.get().await?;

        let mode_arg = match request.mode {
            ScriptDebugMode::Yes => "YES",
            ScriptDebugMode::Sync => "SYNC",
            ScriptDebugMode::No => "NO",
        };

        redis::cmd("SCRIPT")
            .arg("DEBUG")
            .arg(mode_arg)
            .query_async::<()>(&mut *conn)
            .await
            .map_err(Self::map_redis_error)?;

        Ok(ScriptDebugResponse { success: true })
    }

    // ========== Helper Methods ==========

    /// Validate script content
    fn validate_script(&self, script: &str) -> Result<(), CacheError> {
        if script.is_empty() {
            return Err(CacheError::InvalidInput(
                "Script cannot be empty".to_string(),
            ));
        }
        if script.len() > MAX_SCRIPT_SIZE {
            return Err(CacheError::InvalidInput(format!(
                "Script size ({} bytes) exceeds maximum ({} bytes)",
                script.len(),
                MAX_SCRIPT_SIZE
            )));
        }
        Ok(())
    }

    /// Validate SHA1 format
    fn validate_sha(&self, sha: &str) -> Result<(), CacheError> {
        if sha.len() != 40 {
            return Err(CacheError::InvalidInput(format!(
                "SHA must be 40 characters, got {}",
                sha.len()
            )));
        }
        if !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(CacheError::InvalidInput(
                "SHA must contain only hexadecimal characters".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate keys count
    fn validate_keys(&self, keys: &[String]) -> Result<(), CacheError> {
        if keys.len() > MAX_SCRIPT_KEYS {
            return Err(CacheError::InvalidInput(format!(
                "Too many keys ({}) - maximum is {}",
                keys.len(),
                MAX_SCRIPT_KEYS
            )));
        }
        Ok(())
    }

    /// Validate args count
    fn validate_args(&self, args: &[serde_json::Value]) -> Result<(), CacheError> {
        if args.len() > MAX_SCRIPT_ARGS {
            return Err(CacheError::InvalidInput(format!(
                "Too many arguments ({}) - maximum is {}",
                args.len(),
                MAX_SCRIPT_ARGS
            )));
        }
        Ok(())
    }

    /// Convert JSON value to Redis argument string
    fn json_to_redis_arg(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => String::new(),
            serde_json::Value::Bool(b) => {
                if *b {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                serde_json::to_string(value).unwrap_or_default()
            }
        }
    }

    /// Convert a Redis value to a string key for use in JSON objects.
    /// Handles all Redis value types to prevent data loss when converting RESP3 maps.
    fn redis_value_to_string_key(value: redis::Value) -> String {
        match value {
            redis::Value::BulkString(bytes) => String::from_utf8_lossy(&bytes).to_string(),
            redis::Value::SimpleString(s) => s,
            redis::Value::Int(i) => i.to_string(),
            redis::Value::Double(d) => d.to_string(),
            redis::Value::BigNumber(bn) => bn.to_string(),
            redis::Value::Boolean(b) => b.to_string(),
            redis::Value::Nil => "null".to_string(),
            redis::Value::VerbatimString { format: _, text } => text,
            // For complex types, use JSON representation
            other => serde_json::to_string(&Self::redis_value_to_json(other))
                .unwrap_or_else(|_| "<complex>".to_string()),
        }
    }

    /// Convert Redis value to JSON
    fn redis_value_to_json(value: redis::Value) -> serde_json::Value {
        match value {
            redis::Value::Nil => serde_json::Value::Null,
            redis::Value::Int(i) => serde_json::Value::Number(i.into()),
            redis::Value::BulkString(bytes) => {
                serde_json::Value::String(String::from_utf8_lossy(&bytes).to_string())
            }
            redis::Value::SimpleString(s) => serde_json::Value::String(s),
            redis::Value::Array(arr) => {
                let items: Vec<serde_json::Value> =
                    arr.into_iter().map(Self::redis_value_to_json).collect();
                serde_json::Value::Array(items)
            }
            redis::Value::Double(d) => serde_json::json!(d),
            redis::Value::Boolean(b) => serde_json::Value::Bool(b),
            redis::Value::Okay => serde_json::Value::String("OK".to_string()),
            redis::Value::Map(m) => {
                let obj: serde_json::Map<String, serde_json::Value> = m
                    .into_iter()
                    .map(|(k, v)| {
                        // Convert any Redis value to a string key to avoid data loss
                        let key = Self::redis_value_to_string_key(k);
                        (key, Self::redis_value_to_json(v))
                    })
                    .collect();
                serde_json::Value::Object(obj)
            }
            redis::Value::Set(s) => {
                let items: Vec<serde_json::Value> =
                    s.into_iter().map(Self::redis_value_to_json).collect();
                serde_json::Value::Array(items)
            }
            redis::Value::Attribute {
                data,
                attributes: _,
            } => Self::redis_value_to_json(*data),
            redis::Value::Push { kind: _, data } => {
                let items: Vec<serde_json::Value> =
                    data.into_iter().map(Self::redis_value_to_json).collect();
                serde_json::Value::Array(items)
            }
            redis::Value::BigNumber(bn) => serde_json::Value::String(bn.to_string()),
            redis::Value::VerbatimString { format: _, text } => serde_json::Value::String(text),
            redis::Value::ServerError(e) => {
                serde_json::json!({"error": format!("{:?}", e)})
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::redis::connection::InstrumentedPool;
    use serde_json::json;
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

    async fn service_with_redis() -> (ContainerAsync<Redis>, ScriptingService) {
        let (container, redis_url) = start_redis().await;
        let pool = InstrumentedPool::new_for_tests_with_url(&redis_url).unwrap();
        let service = ScriptingService::new(Arc::new(pool));
        (container, service)
    }

    #[test]
    fn test_scripting_service_creation() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = ScriptingService::new(pool);
    }

    #[test]
    fn test_validate_script_empty() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = ScriptingService::new(pool);
        let result = service.validate_script("");
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[test]
    fn test_validate_script_valid() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = ScriptingService::new(pool);
        let result = service.validate_script("return 1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_script_too_large() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = ScriptingService::new(pool);
        let large_script = "x".repeat(MAX_SCRIPT_SIZE + 1);
        let result = service.validate_script(&large_script);
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[test]
    fn test_validate_sha_valid() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = ScriptingService::new(pool);
        let result = service.validate_sha("6b1bf486c81ceb7edf3c093f4a73d3e117c0b169");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_sha_invalid_length() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = ScriptingService::new(pool);
        let result = service.validate_sha("abc123");
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[test]
    fn test_validate_sha_invalid_chars() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = ScriptingService::new(pool);
        let result = service.validate_sha("gggggggggggggggggggggggggggggggggggggggg");
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[test]
    fn test_validate_keys_within_limit() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = ScriptingService::new(pool);
        let keys: Vec<String> = (0..100).map(|i| format!("key:{}", i)).collect();
        let result = service.validate_keys(&keys);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_keys_exceeds_limit() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = ScriptingService::new(pool);
        let keys: Vec<String> = (0..MAX_SCRIPT_KEYS + 1)
            .map(|i| format!("key:{}", i))
            .collect();
        let result = service.validate_keys(&keys);
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[test]
    fn test_validate_args_within_limit() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = ScriptingService::new(pool);
        let args: Vec<serde_json::Value> = (0..100).map(|i| serde_json::json!(i)).collect();
        let result = service.validate_args(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_args_exceeds_limit() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = ScriptingService::new(pool);
        let args: Vec<serde_json::Value> = (0..MAX_SCRIPT_ARGS + 1)
            .map(|i| serde_json::json!(i))
            .collect();
        let result = service.validate_args(&args);
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[test]
    fn test_json_to_redis_arg_null() {
        assert_eq!(
            ScriptingService::json_to_redis_arg(&serde_json::Value::Null),
            ""
        );
    }

    #[test]
    fn test_json_to_redis_arg_bool() {
        assert_eq!(
            ScriptingService::json_to_redis_arg(&serde_json::json!(true)),
            "1"
        );
        assert_eq!(
            ScriptingService::json_to_redis_arg(&serde_json::json!(false)),
            "0"
        );
    }

    #[test]
    fn test_json_to_redis_arg_number() {
        assert_eq!(
            ScriptingService::json_to_redis_arg(&serde_json::json!(42)),
            "42"
        );
        assert_eq!(
            ScriptingService::json_to_redis_arg(&serde_json::json!(3.14)),
            "3.14"
        );
    }

    #[test]
    fn test_json_to_redis_arg_string() {
        assert_eq!(
            ScriptingService::json_to_redis_arg(&serde_json::json!("hello")),
            "hello"
        );
    }

    #[test]
    fn test_json_to_redis_arg_array() {
        let result = ScriptingService::json_to_redis_arg(&serde_json::json!([1, 2, 3]));
        assert_eq!(result, "[1,2,3]");
    }

    #[test]
    fn test_json_to_redis_arg_object() {
        let result = ScriptingService::json_to_redis_arg(&serde_json::json!({"key": "value"}));
        assert!(result.contains("key"));
        assert!(result.contains("value"));
    }

    #[test]
    fn test_redis_value_to_json_nil() {
        assert_eq!(
            ScriptingService::redis_value_to_json(redis::Value::Nil),
            serde_json::Value::Null
        );
    }

    #[test]
    fn test_redis_value_to_json_int() {
        assert_eq!(
            ScriptingService::redis_value_to_json(redis::Value::Int(42)),
            serde_json::json!(42)
        );
    }

    #[test]
    fn test_redis_value_to_json_bulk_string() {
        assert_eq!(
            ScriptingService::redis_value_to_json(redis::Value::BulkString(b"hello".to_vec())),
            serde_json::json!("hello")
        );
    }

    #[test]
    fn test_redis_value_to_json_simple_string() {
        assert_eq!(
            ScriptingService::redis_value_to_json(redis::Value::SimpleString("OK".to_string())),
            serde_json::json!("OK")
        );
    }

    #[test]
    fn test_redis_value_to_json_boolean() {
        assert_eq!(
            ScriptingService::redis_value_to_json(redis::Value::Boolean(true)),
            serde_json::json!(true)
        );
    }

    #[test]
    fn test_redis_value_to_json_double() {
        let result = ScriptingService::redis_value_to_json(redis::Value::Double(3.14));
        assert_eq!(result, serde_json::json!(3.14));
    }

    #[test]
    fn test_redis_value_to_json_okay() {
        assert_eq!(
            ScriptingService::redis_value_to_json(redis::Value::Okay),
            serde_json::json!("OK")
        );
    }

    #[test]
    fn test_redis_value_to_json_array() {
        let arr = vec![
            redis::Value::Int(1),
            redis::Value::BulkString(b"hello".to_vec()),
            redis::Value::Nil,
        ];
        let result = ScriptingService::redis_value_to_json(redis::Value::Array(arr));
        assert_eq!(result, serde_json::json!([1, "hello", null]));
    }

    #[test]
    fn test_redis_value_to_json_nested_array() {
        let arr = vec![
            redis::Value::Array(vec![redis::Value::Int(1), redis::Value::Int(2)]),
            redis::Value::Array(vec![redis::Value::Int(3), redis::Value::Int(4)]),
        ];
        let result = ScriptingService::redis_value_to_json(redis::Value::Array(arr));
        assert_eq!(result, serde_json::json!([[1, 2], [3, 4]]));
    }

    #[test]
    fn test_map_redis_error_variants() {
        let noscript = redis::RedisError::from((redis::ErrorKind::NoScriptError, "NOSCRIPT"));
        assert!(matches!(
            ScriptingService::map_redis_error(noscript),
            CacheError::ScriptError(_)
        ));

        let notbusy = redis::RedisError::from((redis::ErrorKind::NotBusy, "NOTBUSY"));
        assert!(matches!(
            ScriptingService::map_redis_error(notbusy),
            CacheError::ScriptError(_)
        ));

        let extension = redis::RedisError::from((redis::ErrorKind::ExtensionError, "ERR"));
        assert!(matches!(
            ScriptingService::map_redis_error(extension),
            CacheError::ScriptError(_)
        ));

        let response_script = redis::RedisError::from((
            redis::ErrorKind::ResponseError,
            "ERR",
            "NOSCRIPT missing".to_string(),
        ));
        assert!(matches!(
            ScriptingService::map_redis_error(response_script),
            CacheError::ScriptError(_)
        ));

        let response_notbusy = redis::RedisError::from((
            redis::ErrorKind::ResponseError,
            "ERR",
            "No scripts in execution".to_string(),
        ));
        assert!(matches!(
            ScriptingService::map_redis_error(response_notbusy),
            CacheError::ScriptError(_)
        ));

        let response_unkillable = redis::RedisError::from((
            redis::ErrorKind::ResponseError,
            "ERR",
            "UNKILLABLE".to_string(),
        ));
        assert!(matches!(
            ScriptingService::map_redis_error(response_unkillable),
            CacheError::ScriptError(_)
        ));

        let response_other =
            redis::RedisError::from((redis::ErrorKind::ResponseError, "WRONGTYPE"));
        assert!(matches!(
            ScriptingService::map_redis_error(response_other),
            CacheError::RedisError(_)
        ));

        let io_err = redis::RedisError::from((redis::ErrorKind::IoError, "io"));
        assert!(matches!(
            ScriptingService::map_redis_error(io_err),
            CacheError::ConnectionFailed(_)
        ));

        let client_err = redis::RedisError::from((redis::ErrorKind::ClientError, "client"));
        assert!(matches!(
            ScriptingService::map_redis_error(client_err),
            CacheError::ConnectionFailed(_)
        ));
    }

    #[test]
    fn test_redis_value_to_json_map_and_set() {
        let map_value = redis::Value::Map(vec![
            (
                redis::Value::Int(1),
                redis::Value::SimpleString("one".to_string()),
            ),
            (redis::Value::Boolean(true), redis::Value::Int(2)),
            (
                redis::Value::Array(vec![redis::Value::Int(3)]),
                redis::Value::Nil,
            ),
        ]);
        let result = ScriptingService::redis_value_to_json(map_value);
        let obj = result.as_object().unwrap();
        assert_eq!(obj.get("1"), Some(&json!("one")));
        assert_eq!(obj.get("true"), Some(&json!(2)));
        assert_eq!(obj.get("[3]"), Some(&serde_json::Value::Null));

        let set_value = redis::Value::Set(vec![
            redis::Value::BulkString(b"a".to_vec()),
            redis::Value::BulkString(b"b".to_vec()),
        ]);
        let result = ScriptingService::redis_value_to_json(set_value);
        assert_eq!(result, serde_json::json!(["a", "b"]));
    }

    #[test]
    fn test_redis_value_to_json_attribute_push_and_specials() {
        let attr_value = redis::Value::Attribute {
            data: Box::new(redis::Value::SimpleString("ok".to_string())),
            attributes: Vec::new(),
        };
        assert_eq!(
            ScriptingService::redis_value_to_json(attr_value),
            serde_json::json!("ok")
        );

        let push_value = redis::Value::Push {
            kind: redis::PushKind::Message,
            data: vec![
                redis::Value::Int(1),
                redis::Value::BulkString(b"x".to_vec()),
            ],
        };
        assert_eq!(
            ScriptingService::redis_value_to_json(push_value),
            serde_json::json!([1, "x"])
        );

        let big = redis::Value::BigNumber(num_bigint::BigInt::from(123456));
        assert_eq!(
            ScriptingService::redis_value_to_json(big),
            serde_json::json!("123456")
        );

        let verbatim = redis::Value::VerbatimString {
            format: redis::VerbatimFormat::Text,
            text: "text".to_string(),
        };
        assert_eq!(
            ScriptingService::redis_value_to_json(verbatim),
            serde_json::json!("text")
        );

        let server_error = redis::parse_redis_value(b"-ERR boom\r\n").unwrap();
        let json_value = ScriptingService::redis_value_to_json(server_error);
        assert!(json_value.get("error").is_some());
    }

    #[tokio::test]
    async fn test_eval_evalsha_and_script_management() {
        let (_container, service) = service_with_redis().await;

        let eval_request = EvalRequest {
            script: "return {KEYS[1], ARGV[1]}".to_string(),
            keys: vec!["k1".to_string()],
            args: vec![json!("v1")],
            readonly: false,
        };
        let eval_response = service.eval(eval_request).await.unwrap();
        assert_eq!(eval_response.result, json!(["k1", "v1"]));

        let load_response = service
            .script_load(ScriptLoadRequest {
                script: "return ARGV[1]".to_string(),
            })
            .await
            .unwrap();

        let evalsha_response = service
            .evalsha(EvalShaRequest {
                sha: load_response.sha.clone(),
                keys: Vec::new(),
                args: vec![json!("ok")],
                readonly: false,
            })
            .await
            .unwrap();
        assert_eq!(evalsha_response.result, json!("ok"));

        let exists_response = service
            .script_exists(ScriptExistsRequest {
                shas: vec![
                    load_response.sha,
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                ],
            })
            .await
            .unwrap();
        assert_eq!(exists_response.results.len(), 2);

        let flush_sync = service
            .script_flush(ScriptFlushRequest {
                mode: Some(FlushMode::Sync),
            })
            .await;
        assert!(matches!(flush_sync, Ok(_) | Err(CacheError::RedisError(_))));

        let flush_async = service
            .script_flush(ScriptFlushRequest {
                mode: Some(FlushMode::Async),
            })
            .await;
        assert!(matches!(
            flush_async,
            Ok(_) | Err(CacheError::RedisError(_))
        ));
    }

    #[tokio::test]
    async fn test_script_kill_and_debug() {
        let (_container, service) = service_with_redis().await;

        let result = service.script_kill().await;
        assert!(matches!(result, Err(CacheError::ScriptError(_))));

        let debug_response = service
            .script_debug(ScriptDebugRequest {
                mode: ScriptDebugMode::No,
            })
            .await
            .unwrap();
        assert!(debug_response.success);
    }
}
