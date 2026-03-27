//! JSON Repository Trait
//!
//! Abstract interface for RedisJSON operations.

use async_trait::async_trait;
use serde_json::Value;

use crate::domain::entities::{
    JsonArrAppendResult, JsonArrIndexResult, JsonArrInsertResult, JsonArrLenResult,
    JsonArrPopResult, JsonArrTrimResult, JsonClearResult, JsonDebugMemoryResult, JsonDelResult,
    JsonGetResult, JsonMGetResult, JsonMSetItem, JsonNumResult, JsonObjKeysResult,
    JsonObjLenResult, JsonRespResult, JsonSetOptions, JsonSetResult, JsonStrAppendResult,
    JsonStrLenResult, JsonToggleResult, JsonTypeResult,
};
use crate::domain::errors::CacheError;

/// Repository trait for RedisJSON operations
#[async_trait]
pub trait JsonRepository: Send + Sync {
    // ==================== Core Operations ====================

    /// JSON.SET - Set JSON value at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression (e.g., "$", "$.field")
    /// * `value` - JSON value to set
    /// * `options` - Set options (NX, XX)
    async fn json_set(
        &self,
        key: &str,
        path: &str,
        value: Value,
        options: JsonSetOptions,
    ) -> Result<JsonSetResult, CacheError>;

    /// JSON.GET - Get JSON value at path(s)
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `paths` - One or more JSONPath expressions
    async fn json_get(
        &self,
        key: &str,
        paths: &[String],
    ) -> Result<Option<JsonGetResult>, CacheError>;

    /// JSON.MGET - Get values from multiple keys at a path
    ///
    /// # Arguments
    /// * `keys` - List of keys
    /// * `path` - JSONPath expression
    async fn json_mget(&self, keys: &[String], path: &str) -> Result<JsonMGetResult, CacheError>;

    /// JSON.MSET - Set multiple key-path-value triplets
    ///
    /// # Arguments
    /// * `items` - List of (key, path, value) items
    async fn json_mset(&self, items: &[JsonMSetItem]) -> Result<(), CacheError>;

    /// JSON.DEL - Delete value at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression (defaults to root)
    async fn json_del(&self, key: &str, path: &str) -> Result<JsonDelResult, CacheError>;

    /// JSON.TYPE - Get JSON type at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression
    async fn json_type(&self, key: &str, path: &str) -> Result<JsonTypeResult, CacheError>;

    // ==================== String Operations ====================

    /// JSON.STRLEN - Get length of JSON string at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to string value(s)
    async fn json_str_len(&self, key: &str, path: &str) -> Result<JsonStrLenResult, CacheError>;

    /// JSON.STRAPPEND - Append to JSON string at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to string value(s)
    /// * `value` - String to append
    async fn json_str_append(
        &self,
        key: &str,
        path: &str,
        value: &str,
    ) -> Result<JsonStrAppendResult, CacheError>;

    // ==================== Numeric Operations ====================

    /// JSON.NUMINCRBY - Increment numeric value at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to numeric value(s)
    /// * `value` - Amount to increment by (can be negative)
    async fn json_num_incr_by(
        &self,
        key: &str,
        path: &str,
        value: f64,
    ) -> Result<JsonNumResult, CacheError>;

    /// JSON.NUMMULTBY - Multiply numeric value at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to numeric value(s)
    /// * `value` - Multiplier
    async fn json_num_mult_by(
        &self,
        key: &str,
        path: &str,
        value: f64,
    ) -> Result<JsonNumResult, CacheError>;

    /// JSON.TOGGLE - Toggle boolean value at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to boolean value(s)
    async fn json_toggle(&self, key: &str, path: &str) -> Result<JsonToggleResult, CacheError>;

    /// JSON.CLEAR - Clear container (array/object) or set number to 0
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression
    async fn json_clear(&self, key: &str, path: &str) -> Result<JsonClearResult, CacheError>;

    // ==================== Array Operations ====================

    /// JSON.ARRLEN - Get length of JSON array at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to array(s)
    async fn json_arr_len(&self, key: &str, path: &str) -> Result<JsonArrLenResult, CacheError>;

    /// JSON.ARRAPPEND - Append values to JSON array at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to array(s)
    /// * `values` - Values to append
    async fn json_arr_append(
        &self,
        key: &str,
        path: &str,
        values: &[Value],
    ) -> Result<JsonArrAppendResult, CacheError>;

    /// JSON.ARRINDEX - Find index of element in array
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to array(s)
    /// * `value` - Value to search for
    /// * `start` - Start index (optional)
    /// * `stop` - Stop index (optional)
    async fn json_arr_index(
        &self,
        key: &str,
        path: &str,
        value: &Value,
        start: Option<i64>,
        stop: Option<i64>,
    ) -> Result<JsonArrIndexResult, CacheError>;

    /// JSON.ARRINSERT - Insert values at index in array
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to array(s)
    /// * `index` - Index to insert at
    /// * `values` - Values to insert
    async fn json_arr_insert(
        &self,
        key: &str,
        path: &str,
        index: i64,
        values: &[Value],
    ) -> Result<JsonArrInsertResult, CacheError>;

    /// JSON.ARRPOP - Pop element from array
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to array(s)
    /// * `index` - Index to pop from (default: -1, last element)
    async fn json_arr_pop(
        &self,
        key: &str,
        path: &str,
        index: Option<i64>,
    ) -> Result<JsonArrPopResult, CacheError>;

    /// JSON.ARRTRIM - Trim array to specified range
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to array(s)
    /// * `start` - Start index (inclusive)
    /// * `stop` - Stop index (inclusive)
    async fn json_arr_trim(
        &self,
        key: &str,
        path: &str,
        start: i64,
        stop: i64,
    ) -> Result<JsonArrTrimResult, CacheError>;

    // ==================== Object Operations ====================

    /// JSON.OBJLEN - Get number of keys in JSON object at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to object(s)
    async fn json_obj_len(&self, key: &str, path: &str) -> Result<JsonObjLenResult, CacheError>;

    /// JSON.OBJKEYS - Get keys of JSON object at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression pointing to object(s)
    async fn json_obj_keys(&self, key: &str, path: &str) -> Result<JsonObjKeysResult, CacheError>;

    // ==================== Debug Operations ====================

    /// JSON.DEBUG MEMORY - Get memory usage of JSON value at path
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression
    async fn json_debug_memory(
        &self,
        key: &str,
        path: &str,
    ) -> Result<JsonDebugMemoryResult, CacheError>;

    /// JSON.RESP - Get RESP representation of JSON value
    ///
    /// # Arguments
    /// * `key` - The key name
    /// * `path` - JSONPath expression
    async fn json_resp(&self, key: &str, path: &str) -> Result<JsonRespResult, CacheError>;
}
