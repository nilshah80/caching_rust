use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{FunctionFlushMode, FunctionRepository, FunctionRestorePolicy};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisFunctionRepository;

const MAX_FUNCTION_CODE_SIZE: usize = 1024 * 1024;
const MAX_FUNCTION_KEYS: usize = 1000;
const MAX_FUNCTION_ARGS: usize = 1000;

pub struct FunctionService {
    repository: Arc<dyn FunctionRepository>,
}

impl FunctionService {
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisFunctionRepository::new(pool)))
    }

    pub fn new_with_repository(repository: Arc<dyn FunctionRepository>) -> Self {
        Self { repository }
    }

    pub async fn function_load(&self, code: &str, replace: bool) -> Result<String, CacheError> {
        Self::validate_code(code)?;
        self.repository.function_load(code, replace).await
    }

    pub async fn function_delete(&self, name: &str) -> Result<(), CacheError> {
        Self::validate_name(name)?;
        self.repository.function_delete(name).await
    }

    pub async fn function_flush(&self, mode: Option<FunctionFlushMode>) -> Result<(), CacheError> {
        self.repository.function_flush(mode).await
    }

    pub async fn function_dump(&self) -> Result<Vec<u8>, CacheError> {
        self.repository.function_dump().await
    }

    pub async fn function_restore(
        &self,
        payload: &[u8],
        policy: Option<FunctionRestorePolicy>,
    ) -> Result<(), CacheError> {
        if payload.is_empty() {
            return Err(CacheError::InvalidInput(
                "Restore payload cannot be empty".to_string(),
            ));
        }
        self.repository.function_restore(payload, policy).await
    }

    pub async fn function_list(&self, with_code: bool) -> Result<serde_json::Value, CacheError> {
        self.repository.function_list(with_code).await
    }

    pub async fn function_stats(&self) -> Result<serde_json::Value, CacheError> {
        self.repository.function_stats().await
    }

    pub async fn function_kill(&self) -> Result<(), CacheError> {
        self.repository.function_kill().await
    }

    pub async fn fcall(
        &self,
        name: &str,
        keys: &[String],
        args: &[serde_json::Value],
        readonly: bool,
    ) -> Result<serde_json::Value, CacheError> {
        Self::validate_name(name)?;
        Self::validate_keys(keys)?;
        Self::validate_args(args)?;
        self.repository.fcall(name, keys, args, readonly).await
    }

    fn validate_name(name: &str) -> Result<(), CacheError> {
        if name.trim().is_empty() {
            return Err(CacheError::InvalidInput(
                "Function name cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_code(code: &str) -> Result<(), CacheError> {
        if code.trim().is_empty() {
            return Err(CacheError::InvalidInput(
                "Function code cannot be empty".to_string(),
            ));
        }
        if code.len() > MAX_FUNCTION_CODE_SIZE {
            return Err(CacheError::InvalidInput(
                "Function code exceeds 1MB limit".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_keys(keys: &[String]) -> Result<(), CacheError> {
        if keys.len() > MAX_FUNCTION_KEYS {
            return Err(CacheError::InvalidInput(
                "Too many keys for function call".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_args(args: &[serde_json::Value]) -> Result<(), CacheError> {
        if args.len() > MAX_FUNCTION_ARGS {
            return Err(CacheError::InvalidInput(
                "Too many arguments for function call".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::start_generic_redis_image;
    use async_trait::async_trait;
    use std::sync::Mutex;
    use std::time::Duration;
    use testcontainers::ContainerAsync;
    use testcontainers::GenericImage;
    use testcontainers::core::IntoContainerPort;

    #[derive(Default)]
    struct CaptureFunctionRepo {
        load_replace: Mutex<Option<bool>>,
        deleted_name: Mutex<Option<String>>,
        call_name: Mutex<Option<String>>,
        call_readonly: Mutex<Option<bool>>,
    }

    #[async_trait]
    impl FunctionRepository for CaptureFunctionRepo {
        async fn function_load(&self, _code: &str, replace: bool) -> Result<String, CacheError> {
            *self.load_replace.lock().expect("lock") = Some(replace);
            Ok("lib".to_string())
        }

        async fn function_delete(&self, name: &str) -> Result<(), CacheError> {
            *self.deleted_name.lock().expect("lock") = Some(name.to_string());
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

        async fn function_list(&self, _with_code: bool) -> Result<serde_json::Value, CacheError> {
            Ok(serde_json::json!([{"library_name": "lib"}]))
        }

        async fn function_stats(&self) -> Result<serde_json::Value, CacheError> {
            Ok(serde_json::json!({"running_script": null}))
        }

        async fn function_kill(&self) -> Result<(), CacheError> {
            Ok(())
        }

        async fn fcall(
            &self,
            name: &str,
            _keys: &[String],
            _args: &[serde_json::Value],
            readonly: bool,
        ) -> Result<serde_json::Value, CacheError> {
            *self.call_name.lock().expect("lock") = Some(name.to_string());
            *self.call_readonly.lock().expect("lock") = Some(readonly);
            Ok(serde_json::json!("ok"))
        }
    }

    #[test]
    fn test_validate_code_empty() {
        assert!(matches!(
            FunctionService::validate_code(""),
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_validate_code_too_large() {
        assert!(matches!(
            FunctionService::validate_code(&"x".repeat(MAX_FUNCTION_CODE_SIZE + 1)),
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_validate_name_empty() {
        assert!(matches!(
            FunctionService::validate_name(" "),
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_validate_keys_limit() {
        let keys: Vec<String> = (0..=MAX_FUNCTION_KEYS).map(|i| format!("k{i}")).collect();
        assert!(matches!(
            FunctionService::validate_keys(&keys),
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_validate_args_limit() {
        let args: Vec<serde_json::Value> = (0..=MAX_FUNCTION_ARGS)
            .map(|i| serde_json::json!(i))
            .collect();
        assert!(matches!(
            FunctionService::validate_args(&args),
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn test_function_load_delegates() {
        let repo = Arc::new(CaptureFunctionRepo::default());
        let service = FunctionService::new_with_repository(repo.clone());
        let library = service
            .function_load(
                "#!lua name=lib\nredis.register_function('x', function() return 1 end)",
                true,
            )
            .await
            .expect("load");
        assert_eq!(library, "lib");
        assert_eq!(*repo.load_replace.lock().expect("lock"), Some(true));
    }

    #[tokio::test]
    async fn test_function_delete_delegates() {
        let repo = Arc::new(CaptureFunctionRepo::default());
        let service = FunctionService::new_with_repository(repo.clone());
        service.function_delete("lib").await.expect("delete");
        assert_eq!(
            *repo.deleted_name.lock().expect("lock"),
            Some("lib".to_string())
        );
    }

    #[tokio::test]
    async fn test_function_restore_empty_payload() {
        let service =
            FunctionService::new_with_repository(Arc::new(CaptureFunctionRepo::default()));
        assert!(matches!(
            service.function_restore(&[], None).await,
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn test_function_stats_delegates() {
        let service =
            FunctionService::new_with_repository(Arc::new(CaptureFunctionRepo::default()));
        let stats = service.function_stats().await.expect("stats");
        assert_eq!(stats, serde_json::json!({"running_script": null}));
    }

    #[tokio::test]
    async fn test_function_kill_delegates() {
        let service =
            FunctionService::new_with_repository(Arc::new(CaptureFunctionRepo::default()));
        service.function_kill().await.expect("kill");
    }

    #[tokio::test]
    async fn test_fcall_delegates() {
        let repo = Arc::new(CaptureFunctionRepo::default());
        let service = FunctionService::new_with_repository(repo.clone());
        let value = service
            .fcall(
                "lib.echo",
                &["k1".to_string()],
                &[serde_json::json!("v1")],
                true,
            )
            .await
            .expect("fcall");
        assert_eq!(value, serde_json::json!("ok"));
        assert_eq!(
            *repo.call_name.lock().expect("lock"),
            Some("lib.echo".to_string())
        );
        assert_eq!(*repo.call_readonly.lock().expect("lock"), Some(true));
    }

    async fn start_redis() -> Option<(ContainerAsync<GenericImage>, String)> {
        let image = GenericImage::new("redis", "7.4").with_exposed_port(6379.tcp());
        start_generic_redis_image(image, 6379, Duration::from_secs(2), "redis").await
    }

    async fn service_with_redis() -> Option<(
        ContainerAsync<GenericImage>,
        Arc<InstrumentedPool>,
        FunctionService,
    )> {
        let (container, redis_url) = start_redis().await?;
        let pool = Arc::new(InstrumentedPool::new_for_tests_with_url(&redis_url).expect("pool"));
        let service = FunctionService::new(pool.clone());
        Some((container, pool, service))
    }

    #[tokio::test]
    async fn test_function_load_list_and_fcall_integration() {
        let Some((_container, pool, service)) = service_with_redis().await else {
            return;
        };
        let code = "#!lua name=lib\nredis.register_function('echo', function(keys, args) return args[1] end)\nredis.register_function{function_name='get_value', callback=function(keys, args) return redis.call('GET', keys[1]) end, flags={'no-writes'}}";

        let library = service.function_load(code, false).await.expect("load");
        assert_eq!(library, "lib");

        let libraries = service.function_list(false).await.expect("list");
        assert!(libraries.is_array());

        let echo = service
            .fcall("echo", &[], &[serde_json::json!("hello")], false)
            .await
            .expect("fcall");
        assert_eq!(echo, serde_json::json!("hello"));

        let mut conn = pool.get().await.expect("conn");
        let _: () = redis::cmd("SET")
            .arg("mykey")
            .arg("value")
            .query_async(&mut conn)
            .await
            .expect("set");
        let get_value = service
            .fcall("get_value", &["mykey".to_string()], &[], true)
            .await
            .expect("fcall_ro");
        assert_eq!(get_value, serde_json::json!("value"));
    }

    #[tokio::test]
    async fn test_function_dump_flush_restore_integration() {
        let Some((_container, _pool, service)) = service_with_redis().await else {
            return;
        };
        let code = "#!lua name=lib\nredis.register_function('echo', function(keys, args) return args[1] end)";
        service.function_load(code, false).await.expect("load");
        let dump = service.function_dump().await.expect("dump");
        assert!(!dump.is_empty());

        service
            .function_flush(Some(FunctionFlushMode::Sync))
            .await
            .expect("flush");
        service
            .function_restore(&dump, Some(FunctionRestorePolicy::Append))
            .await
            .expect("restore");

        let value = service
            .fcall("echo", &[], &[serde_json::json!("restored")], false)
            .await
            .expect("fcall");
        assert_eq!(value, serde_json::json!("restored"));
    }

    #[tokio::test]
    async fn test_function_stats_and_delete_integration() {
        let Some((_container, _pool, service)) = service_with_redis().await else {
            return;
        };
        let code = "#!lua name=lib\nredis.register_function('echo', function(keys, args) return args[1] end)";
        service.function_load(code, false).await.expect("load");
        let stats = service.function_stats().await.expect("stats");
        assert!(stats.is_object() || stats.is_array());

        service.function_delete("lib").await.expect("delete");
        let result = service
            .fcall("echo", &[], &[serde_json::json!("gone")], false)
            .await;
        assert!(result.is_err());
    }
}
