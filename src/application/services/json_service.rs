//! JSON Service
//!
//! Business logic layer for RedisJSON operations.

use std::sync::Arc;

use serde_json::Value;

use crate::domain::entities::{
    JsonArrAppendResult, JsonArrIndexResult, JsonArrInsertResult, JsonArrLenResult,
    JsonArrPopResult, JsonArrTrimResult, JsonClearResult, JsonDebugMemoryResult, JsonDelResult,
    JsonGetResult, JsonMGetResult, JsonMSetItem, JsonNumResult, JsonObjKeysResult,
    JsonObjLenResult, JsonRespResult, JsonSetOptions, JsonSetResult, JsonStrAppendResult,
    JsonStrLenResult, JsonToggleResult, JsonTypeResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::JsonRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisJsonRepository;

/// Service for RedisJSON operations
pub struct JsonService {
    repository: Arc<dyn JsonRepository>,
}

impl JsonService {
    /// Create a new JsonService
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisJsonRepository::new(pool)))
    }

    /// Create a JsonService with a custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn JsonRepository>) -> Self {
        Self { repository }
    }

    // ==================== Core Operations ====================

    /// Set a JSON value at a path
    pub async fn json_set(
        &self,
        key: &str,
        path: &str,
        value: Value,
        nx: bool,
        xx: bool,
    ) -> Result<JsonSetResult, CacheError> {
        // Validate path
        self.validate_path(path)?;

        // NX and XX are mutually exclusive
        if nx && xx {
            return Err(CacheError::InvalidInput(
                "NX and XX options are mutually exclusive".to_string(),
            ));
        }

        let options = JsonSetOptions { nx, xx };
        self.repository.json_set(key, path, value, options).await
    }

    /// Get JSON value(s) at path(s)
    pub async fn json_get(
        &self,
        key: &str,
        paths: Vec<String>,
    ) -> Result<Option<JsonGetResult>, CacheError> {
        // Validate paths
        for path in &paths {
            self.validate_path(path)?;
        }
        self.repository.json_get(key, &paths).await
    }

    /// Get values from multiple keys at a path
    pub async fn json_mget(
        &self,
        keys: Vec<String>,
        path: &str,
    ) -> Result<JsonMGetResult, CacheError> {
        if keys.is_empty() {
            return Err(CacheError::InvalidInput(
                "Keys list cannot be empty".to_string(),
            ));
        }
        self.validate_path(path)?;
        self.repository.json_mget(&keys, path).await
    }

    /// Set multiple key-path-value triplets
    pub async fn json_mset(&self, items: Vec<JsonMSetItem>) -> Result<(), CacheError> {
        if items.is_empty() {
            return Err(CacheError::InvalidInput(
                "Items list cannot be empty".to_string(),
            ));
        }
        for item in &items {
            self.validate_path(&item.path)?;
        }
        self.repository.json_mset(&items).await
    }

    /// Delete value at path
    pub async fn json_del(&self, key: &str, path: &str) -> Result<JsonDelResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_del(key, path).await
    }

    /// Get JSON type at path
    pub async fn json_type(&self, key: &str, path: &str) -> Result<JsonTypeResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_type(key, path).await
    }

    // ==================== String Operations ====================

    /// Get length of JSON string at path
    pub async fn json_str_len(
        &self,
        key: &str,
        path: &str,
    ) -> Result<JsonStrLenResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_str_len(key, path).await
    }

    /// Append to JSON string at path
    pub async fn json_str_append(
        &self,
        key: &str,
        path: &str,
        value: &str,
    ) -> Result<JsonStrAppendResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_str_append(key, path, value).await
    }

    // ==================== Numeric Operations ====================

    /// Increment numeric value at path
    pub async fn json_num_incr_by(
        &self,
        key: &str,
        path: &str,
        value: f64,
    ) -> Result<JsonNumResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_num_incr_by(key, path, value).await
    }

    /// Multiply numeric value at path
    pub async fn json_num_mult_by(
        &self,
        key: &str,
        path: &str,
        value: f64,
    ) -> Result<JsonNumResult, CacheError> {
        self.validate_path(path)?;
        if value == 0.0 {
            // Multiplying by zero is valid but might be unintentional
            // Let it through but Redis will handle it
        }
        self.repository.json_num_mult_by(key, path, value).await
    }

    /// Toggle boolean value at path
    pub async fn json_toggle(&self, key: &str, path: &str) -> Result<JsonToggleResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_toggle(key, path).await
    }

    /// Clear container or set number to 0
    pub async fn json_clear(&self, key: &str, path: &str) -> Result<JsonClearResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_clear(key, path).await
    }

    // ==================== Array Operations ====================

    /// Get length of JSON array at path
    pub async fn json_arr_len(
        &self,
        key: &str,
        path: &str,
    ) -> Result<JsonArrLenResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_arr_len(key, path).await
    }

    /// Append values to JSON array at path
    pub async fn json_arr_append(
        &self,
        key: &str,
        path: &str,
        values: Vec<Value>,
    ) -> Result<JsonArrAppendResult, CacheError> {
        self.validate_path(path)?;
        if values.is_empty() {
            return Err(CacheError::InvalidInput(
                "Values list cannot be empty".to_string(),
            ));
        }
        self.repository.json_arr_append(key, path, &values).await
    }

    /// Find index of element in array
    pub async fn json_arr_index(
        &self,
        key: &str,
        path: &str,
        value: Value,
        start: Option<i64>,
        stop: Option<i64>,
    ) -> Result<JsonArrIndexResult, CacheError> {
        self.validate_path(path)?;
        self.repository
            .json_arr_index(key, path, &value, start, stop)
            .await
    }

    /// Insert values at index in array
    pub async fn json_arr_insert(
        &self,
        key: &str,
        path: &str,
        index: i64,
        values: Vec<Value>,
    ) -> Result<JsonArrInsertResult, CacheError> {
        self.validate_path(path)?;
        if values.is_empty() {
            return Err(CacheError::InvalidInput(
                "Values list cannot be empty".to_string(),
            ));
        }
        self.repository
            .json_arr_insert(key, path, index, &values)
            .await
    }

    /// Pop element from array
    pub async fn json_arr_pop(
        &self,
        key: &str,
        path: &str,
        index: Option<i64>,
    ) -> Result<JsonArrPopResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_arr_pop(key, path, index).await
    }

    /// Trim array to specified range
    pub async fn json_arr_trim(
        &self,
        key: &str,
        path: &str,
        start: i64,
        stop: i64,
    ) -> Result<JsonArrTrimResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_arr_trim(key, path, start, stop).await
    }

    // ==================== Object Operations ====================

    /// Get number of keys in JSON object at path
    pub async fn json_obj_len(
        &self,
        key: &str,
        path: &str,
    ) -> Result<JsonObjLenResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_obj_len(key, path).await
    }

    /// Get keys of JSON object at path
    pub async fn json_obj_keys(
        &self,
        key: &str,
        path: &str,
    ) -> Result<JsonObjKeysResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_obj_keys(key, path).await
    }

    // ==================== Debug Operations ====================

    /// Get memory usage of JSON value at path
    pub async fn json_debug_memory(
        &self,
        key: &str,
        path: &str,
    ) -> Result<JsonDebugMemoryResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_debug_memory(key, path).await
    }

    /// Get RESP representation of JSON value
    pub async fn json_resp(&self, key: &str, path: &str) -> Result<JsonRespResult, CacheError> {
        self.validate_path(path)?;
        self.repository.json_resp(key, path).await
    }

    // ==================== Validation Helpers ====================

    /// Validate JSONPath syntax (basic validation)
    ///
    /// RedisJSON paths must start with `$` (JSONPath syntax).
    /// The legacy dot-notation (e.g., `.foo`) is deprecated and not recommended.
    fn validate_path(&self, path: &str) -> Result<(), CacheError> {
        if path.is_empty() {
            return Err(CacheError::InvalidInput("Path cannot be empty".to_string()));
        }

        // RedisJSON expects paths to start with $
        if !path.starts_with('$') {
            return Err(CacheError::InvalidInput(format!(
                "Invalid JSONPath: path must start with '$', got: '{}'. Use '$' for root or '$.field' for nested paths.",
                path
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::redis::connection::InstrumentedPool;
    use crate::test_support::MockJsonRepository;
    use serde_json::json;

    #[test]
    fn test_json_service_new() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = JsonService::new(pool);
    }

    #[test]
    fn test_validate_path() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = JsonService::new(pool);

        // Valid paths (must start with $)
        assert!(service.validate_path("$").is_ok());
        assert!(service.validate_path("$.name").is_ok());
        assert!(service.validate_path("$..name").is_ok());
        assert!(service.validate_path("$.store.book[0].title").is_ok());
        assert!(service.validate_path("$[0]").is_ok());

        // Invalid paths
        assert!(service.validate_path("").is_err());
        assert!(service.validate_path("name").is_err());
        assert!(service.validate_path("[0]").is_err());
        // Dot-prefixed paths are rejected (legacy syntax)
        assert!(service.validate_path(".name").is_err());
        assert!(service.validate_path(".store.book").is_err());
    }

    #[tokio::test]
    async fn test_json_set_nx_xx_mutual_exclusion() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = JsonService::new(pool);

        let result = service
            .json_set("key", "$", Value::String("test".to_string()), true, true)
            .await;

        assert!(result.is_err());
        if let Err(CacheError::InvalidInput(msg)) = result {
            assert!(msg.contains("mutually exclusive"));
        }
    }

    #[tokio::test]
    async fn test_json_mget_empty_keys() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = JsonService::new(pool);

        let result = service.json_mget(vec![], "$").await;
        assert!(result.is_err());
        if let Err(CacheError::InvalidInput(msg)) = result {
            assert!(msg.contains("empty"));
        }
    }

    #[tokio::test]
    async fn test_json_mset_empty_items() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = JsonService::new(pool);

        let result = service.json_mset(vec![]).await;
        assert!(result.is_err());
        if let Err(CacheError::InvalidInput(msg)) = result {
            assert!(msg.contains("empty"));
        }
    }

    #[tokio::test]
    async fn test_json_arr_append_empty_values() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = JsonService::new(pool);

        let result = service.json_arr_append("key", "$", vec![]).await;
        assert!(result.is_err());
        if let Err(CacheError::InvalidInput(msg)) = result {
            assert!(msg.contains("empty"));
        }
    }

    #[tokio::test]
    async fn test_json_arr_insert_empty_values() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let service = JsonService::new(pool);

        let result = service.json_arr_insert("key", "$", 0, vec![]).await;
        assert!(result.is_err());
        if let Err(CacheError::InvalidInput(msg)) = result {
            assert!(msg.contains("empty"));
        }
    }

    fn service_with_mock() -> JsonService {
        JsonService::new_with_repository(Arc::new(MockJsonRepository::new()))
    }

    #[tokio::test]
    async fn test_json_mget_invalid_path() {
        let service = service_with_mock();
        let result = service.json_mget(vec!["key".to_string()], "invalid").await;
        assert!(matches!(result, Err(CacheError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_json_service_core_and_string_ops() {
        let service = service_with_mock();

        let set_result = service
            .json_set("key", "$", json!({"a": 1}), false, false)
            .await
            .unwrap();
        assert!(set_result.success);

        let get_result = service
            .json_get("key", vec!["$".to_string()])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(get_result.key, "key");

        let mget_result = service
            .json_mget(vec!["k1".to_string(), "k2".to_string()], "$")
            .await
            .unwrap();
        assert_eq!(mget_result.results.len(), 2);

        let items = vec![JsonMSetItem {
            key: "k1".to_string(),
            path: "$".to_string(),
            value: json!({"b": 2}),
        }];
        service.json_mset(items).await.unwrap();

        let del_result = service.json_del("key", "$").await.unwrap();
        assert_eq!(del_result.deleted_count, 1);

        let type_result = service.json_type("key", "$").await.unwrap();
        assert_eq!(type_result.types.len(), 1);

        let str_len_result = service.json_str_len("key", "$").await.unwrap();
        assert_eq!(str_len_result.lengths.len(), 1);

        let str_append_result = service.json_str_append("key", "$", "x").await.unwrap();
        assert_eq!(str_append_result.new_lengths.len(), 1);
    }

    #[tokio::test]
    async fn test_json_service_numeric_toggle_clear() {
        let service = service_with_mock();

        let incr_result = service.json_num_incr_by("key", "$", 1.0).await.unwrap();
        assert!(incr_result.values.is_array());

        let mult_result = service.json_num_mult_by("key", "$", 0.0).await.unwrap();
        assert!(mult_result.values.is_array());

        let toggle_result = service.json_toggle("key", "$").await.unwrap();
        assert_eq!(toggle_result.values.len(), 1);

        let clear_result = service.json_clear("key", "$").await.unwrap();
        assert_eq!(clear_result.cleared_count, 1);
    }

    #[tokio::test]
    async fn test_json_service_array_ops() {
        let service = service_with_mock();

        let len_result = service.json_arr_len("key", "$").await.unwrap();
        assert_eq!(len_result.lengths.len(), 1);

        let append_result = service
            .json_arr_append("key", "$", vec![json!(1)])
            .await
            .unwrap();
        assert_eq!(append_result.new_lengths.len(), 1);

        let index_result = service
            .json_arr_index("key", "$", json!(1), Some(0), Some(1))
            .await
            .unwrap();
        assert_eq!(index_result.indices.len(), 1);

        let insert_result = service
            .json_arr_insert("key", "$", 0, vec![json!(1)])
            .await
            .unwrap();
        assert_eq!(insert_result.new_lengths.len(), 1);

        let pop_result = service.json_arr_pop("key", "$", Some(0)).await.unwrap();
        assert_eq!(pop_result.values.len(), 1);

        let trim_result = service.json_arr_trim("key", "$", 0, 1).await.unwrap();
        assert_eq!(trim_result.new_lengths.len(), 1);
    }

    #[tokio::test]
    async fn test_json_service_object_and_debug_ops() {
        let service = service_with_mock();

        let obj_len_result = service.json_obj_len("key", "$").await.unwrap();
        assert_eq!(obj_len_result.lengths.len(), 1);

        let obj_keys_result = service.json_obj_keys("key", "$").await.unwrap();
        assert_eq!(obj_keys_result.keys.len(), 1);

        let memory_result = service.json_debug_memory("key", "$").await.unwrap();
        assert_eq!(memory_result.memory_bytes.len(), 1);

        let resp_result = service.json_resp("key", "$").await.unwrap();
        assert!(resp_result.resp.is_array());
    }
}
