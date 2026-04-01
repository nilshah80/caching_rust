use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    TimeSeriesCreateOptions, TimeSeriesMGetResult, TimeSeriesRangeOptions, TimeSeriesRangeResult,
    TimeSeriesRepository, TimeSeriesSample, TsAggregation,
};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisTimeSeriesRepository;

pub struct TimeSeriesService {
    repository: Arc<dyn TimeSeriesRepository>,
}

impl TimeSeriesService {
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisTimeSeriesRepository::new(pool)))
    }

    pub fn new_with_repository(repository: Arc<dyn TimeSeriesRepository>) -> Self {
        Self { repository }
    }

    pub async fn ts_create(
        &self,
        key: &str,
        options: TimeSeriesCreateOptions,
    ) -> Result<(), CacheError> {
        Self::validate_key(key)?;
        self.repository.ts_create(key, options).await
    }

    pub async fn ts_alter(
        &self,
        key: &str,
        options: TimeSeriesCreateOptions,
    ) -> Result<(), CacheError> {
        Self::validate_key(key)?;
        self.repository.ts_alter(key, options).await
    }

    pub async fn ts_add(&self, key: &str, sample: TimeSeriesSample) -> Result<i64, CacheError> {
        Self::validate_key(key)?;
        Self::validate_sample(&sample)?;
        self.repository.ts_add(key, sample).await
    }

    pub async fn ts_madd(
        &self,
        items: &[(String, TimeSeriesSample)],
    ) -> Result<Vec<i64>, CacheError> {
        if items.is_empty() {
            return Err(CacheError::InvalidInput(
                "At least one sample is required".to_string(),
            ));
        }
        for (key, sample) in items {
            Self::validate_key(key)?;
            Self::validate_sample(sample)?;
        }
        self.repository.ts_madd(items).await
    }

    pub async fn ts_incr_by(
        &self,
        key: &str,
        value: f64,
        timestamp: Option<i64>,
    ) -> Result<i64, CacheError> {
        Self::validate_key(key)?;
        Self::validate_numeric_value(value)?;
        if let Some(ts) = timestamp {
            Self::validate_timestamp(ts)?;
        }
        self.repository.ts_incr_by(key, value, timestamp).await
    }

    pub async fn ts_decr_by(
        &self,
        key: &str,
        value: f64,
        timestamp: Option<i64>,
    ) -> Result<i64, CacheError> {
        Self::validate_key(key)?;
        Self::validate_numeric_value(value)?;
        if let Some(ts) = timestamp {
            Self::validate_timestamp(ts)?;
        }
        self.repository.ts_decr_by(key, value, timestamp).await
    }

    pub async fn ts_del(&self, key: &str, from: i64, to: i64) -> Result<i64, CacheError> {
        Self::validate_key(key)?;
        Self::validate_range(from, to)?;
        self.repository.ts_del(key, from, to).await
    }

    pub async fn ts_get(&self, key: &str) -> Result<Option<TimeSeriesSample>, CacheError> {
        Self::validate_key(key)?;
        self.repository.ts_get(key).await
    }

    pub async fn ts_mget(
        &self,
        filters: &[String],
    ) -> Result<Vec<TimeSeriesMGetResult>, CacheError> {
        Self::validate_filters(filters)?;
        self.repository.ts_mget(filters).await
    }

    pub async fn ts_range(
        &self,
        key: &str,
        from: i64,
        to: i64,
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesSample>, CacheError> {
        Self::validate_key(key)?;
        Self::validate_range(from, to)?;
        Self::validate_range_options(&options)?;
        self.repository.ts_range(key, from, to, options).await
    }

    pub async fn ts_rev_range(
        &self,
        key: &str,
        from: i64,
        to: i64,
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesSample>, CacheError> {
        Self::validate_key(key)?;
        Self::validate_range(from, to)?;
        Self::validate_range_options(&options)?;
        self.repository.ts_rev_range(key, from, to, options).await
    }

    pub async fn ts_mrange(
        &self,
        from: i64,
        to: i64,
        filters: &[String],
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesRangeResult>, CacheError> {
        Self::validate_range(from, to)?;
        Self::validate_filters(filters)?;
        Self::validate_range_options(&options)?;
        self.repository.ts_mrange(from, to, filters, options).await
    }

    pub async fn ts_mrev_range(
        &self,
        from: i64,
        to: i64,
        filters: &[String],
        options: TimeSeriesRangeOptions,
    ) -> Result<Vec<TimeSeriesRangeResult>, CacheError> {
        Self::validate_range(from, to)?;
        Self::validate_filters(filters)?;
        Self::validate_range_options(&options)?;
        self.repository
            .ts_mrev_range(from, to, filters, options)
            .await
    }

    pub async fn ts_query_index(&self, filters: &[String]) -> Result<Vec<String>, CacheError> {
        Self::validate_filters(filters)?;
        self.repository.ts_query_index(filters).await
    }

    pub async fn ts_info(&self, key: &str) -> Result<serde_json::Value, CacheError> {
        Self::validate_key(key)?;
        self.repository.ts_info(key).await
    }

    pub async fn ts_create_rule(
        &self,
        source: &str,
        dest: &str,
        aggregation: TsAggregation,
        bucket_duration_ms: u64,
    ) -> Result<(), CacheError> {
        Self::validate_key(source)?;
        Self::validate_key(dest)?;
        if bucket_duration_ms == 0 {
            return Err(CacheError::InvalidInput(
                "Bucket duration must be positive".to_string(),
            ));
        }
        self.repository
            .ts_create_rule(source, dest, aggregation, bucket_duration_ms)
            .await
    }

    pub async fn ts_delete_rule(&self, source: &str, dest: &str) -> Result<(), CacheError> {
        Self::validate_key(source)?;
        Self::validate_key(dest)?;
        self.repository.ts_delete_rule(source, dest).await
    }

    fn validate_key(key: &str) -> Result<(), CacheError> {
        if key.trim().is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        Ok(())
    }

    fn validate_timestamp(timestamp: i64) -> Result<(), CacheError> {
        if timestamp < 0 {
            return Err(CacheError::InvalidInput(
                "Timestamp must be non-negative".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_numeric_value(value: f64) -> Result<(), CacheError> {
        if !value.is_finite() {
            return Err(CacheError::InvalidInput(
                "Value must be a finite number".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_sample(sample: &TimeSeriesSample) -> Result<(), CacheError> {
        Self::validate_timestamp(sample.timestamp)?;
        Self::validate_numeric_value(sample.value)
    }

    fn validate_range(from: i64, to: i64) -> Result<(), CacheError> {
        Self::validate_timestamp(from)?;
        Self::validate_timestamp(to)?;
        if from > to {
            return Err(CacheError::InvalidInput(
                "Range start must be less than or equal to end".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_filters(filters: &[String]) -> Result<(), CacheError> {
        if filters.is_empty() {
            return Err(CacheError::InvalidInput(
                "At least one filter is required".to_string(),
            ));
        }
        if filters.iter().any(|filter| filter.trim().is_empty()) {
            return Err(CacheError::InvalidInput(
                "Filters cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_range_options(options: &TimeSeriesRangeOptions) -> Result<(), CacheError> {
        if options.aggregation.is_some() && options.bucket_duration_ms.unwrap_or(0) == 0 {
            return Err(CacheError::InvalidInput(
                "Bucket duration is required when aggregation is set".to_string(),
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
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::Duration;
    use testcontainers::ContainerAsync;
    use testcontainers::GenericImage;
    use testcontainers::core::IntoContainerPort;

    #[derive(Default)]
    struct CaptureTimeSeriesRepo {
        created_key: Mutex<Option<String>>,
        added_key: Mutex<Option<String>>,
    }

    #[async_trait]
    impl TimeSeriesRepository for CaptureTimeSeriesRepo {
        async fn ts_create(
            &self,
            key: &str,
            _options: TimeSeriesCreateOptions,
        ) -> Result<(), CacheError> {
            *self.created_key.lock().expect("lock") = Some(key.to_string());
            Ok(())
        }

        async fn ts_alter(
            &self,
            _key: &str,
            _options: TimeSeriesCreateOptions,
        ) -> Result<(), CacheError> {
            Ok(())
        }

        async fn ts_add(&self, key: &str, _sample: TimeSeriesSample) -> Result<i64, CacheError> {
            *self.added_key.lock().expect("lock") = Some(key.to_string());
            Ok(123)
        }

        async fn ts_madd(
            &self,
            items: &[(String, TimeSeriesSample)],
        ) -> Result<Vec<i64>, CacheError> {
            Ok(items.iter().map(|(_, sample)| sample.timestamp).collect())
        }

        async fn ts_incr_by(
            &self,
            _key: &str,
            _value: f64,
            timestamp: Option<i64>,
        ) -> Result<i64, CacheError> {
            Ok(timestamp.unwrap_or(1))
        }

        async fn ts_decr_by(
            &self,
            _key: &str,
            _value: f64,
            timestamp: Option<i64>,
        ) -> Result<i64, CacheError> {
            Ok(timestamp.unwrap_or(1))
        }

        async fn ts_del(&self, _key: &str, _from: i64, _to: i64) -> Result<i64, CacheError> {
            Ok(1)
        }

        async fn ts_get(&self, _key: &str) -> Result<Option<TimeSeriesSample>, CacheError> {
            Ok(Some(TimeSeriesSample {
                timestamp: 1,
                value: 2.0,
            }))
        }

        async fn ts_mget(
            &self,
            filters: &[String],
        ) -> Result<Vec<TimeSeriesMGetResult>, CacheError> {
            Ok(vec![TimeSeriesMGetResult {
                key: filters[0].clone(),
                labels: HashMap::new(),
                sample: Some(TimeSeriesSample {
                    timestamp: 1,
                    value: 2.0,
                }),
            }])
        }

        async fn ts_range(
            &self,
            _key: &str,
            _from: i64,
            _to: i64,
            _options: TimeSeriesRangeOptions,
        ) -> Result<Vec<TimeSeriesSample>, CacheError> {
            Ok(vec![TimeSeriesSample {
                timestamp: 1,
                value: 2.0,
            }])
        }

        async fn ts_rev_range(
            &self,
            _key: &str,
            _from: i64,
            _to: i64,
            _options: TimeSeriesRangeOptions,
        ) -> Result<Vec<TimeSeriesSample>, CacheError> {
            Ok(vec![TimeSeriesSample {
                timestamp: 2,
                value: 3.0,
            }])
        }

        async fn ts_mrange(
            &self,
            _from: i64,
            _to: i64,
            filters: &[String],
            _options: TimeSeriesRangeOptions,
        ) -> Result<Vec<TimeSeriesRangeResult>, CacheError> {
            Ok(vec![TimeSeriesRangeResult {
                key: filters[0].clone(),
                labels: HashMap::new(),
                samples: vec![],
            }])
        }

        async fn ts_mrev_range(
            &self,
            _from: i64,
            _to: i64,
            filters: &[String],
            _options: TimeSeriesRangeOptions,
        ) -> Result<Vec<TimeSeriesRangeResult>, CacheError> {
            Ok(vec![TimeSeriesRangeResult {
                key: filters[0].clone(),
                labels: HashMap::new(),
                samples: vec![],
            }])
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
        ) -> Result<(), CacheError> {
            Ok(())
        }

        async fn ts_delete_rule(&self, _source: &str, _dest: &str) -> Result<(), CacheError> {
            Ok(())
        }
    }

    #[test]
    fn test_validate_key_empty() {
        assert!(matches!(
            TimeSeriesService::validate_key(""),
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_validate_sample_invalid_timestamp() {
        assert!(matches!(
            TimeSeriesService::validate_sample(&TimeSeriesSample {
                timestamp: -1,
                value: 1.0
            }),
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_validate_sample_invalid_value() {
        assert!(matches!(
            TimeSeriesService::validate_sample(&TimeSeriesSample {
                timestamp: 1,
                value: f64::NAN
            }),
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_validate_range_invalid() {
        assert!(matches!(
            TimeSeriesService::validate_range(10, 5),
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_validate_filters_empty() {
        assert!(matches!(
            TimeSeriesService::validate_filters(&[]),
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_validate_range_options_requires_bucket() {
        assert!(matches!(
            TimeSeriesService::validate_range_options(&TimeSeriesRangeOptions {
                count: None,
                aggregation: Some(TsAggregation::Avg),
                bucket_duration_ms: None,
            }),
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn test_ts_create_delegates() {
        let repo = Arc::new(CaptureTimeSeriesRepo::default());
        let service = TimeSeriesService::new_with_repository(repo.clone());
        service
            .ts_create("metrics", TimeSeriesCreateOptions::default())
            .await
            .expect("create");
        assert_eq!(
            *repo.created_key.lock().expect("lock"),
            Some("metrics".to_string())
        );
    }

    #[tokio::test]
    async fn test_ts_add_delegates() {
        let repo = Arc::new(CaptureTimeSeriesRepo::default());
        let service = TimeSeriesService::new_with_repository(repo.clone());
        let ts = service
            .ts_add(
                "metrics",
                TimeSeriesSample {
                    timestamp: 123,
                    value: 1.5,
                },
            )
            .await
            .expect("add");
        assert_eq!(ts, 123);
        assert_eq!(
            *repo.added_key.lock().expect("lock"),
            Some("metrics".to_string())
        );
    }

    #[tokio::test]
    async fn test_ts_madd_requires_items() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        assert!(matches!(
            service.ts_madd(&[]).await,
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn test_ts_alter_delegates() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        service
            .ts_alter("metrics", TimeSeriesCreateOptions::default())
            .await
            .expect("alter");
    }

    #[tokio::test]
    async fn test_ts_madd_validates_items() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        let items = vec![
            (
                "key1".to_string(),
                TimeSeriesSample {
                    timestamp: 100,
                    value: 1.0,
                },
            ),
            (
                "key2".to_string(),
                TimeSeriesSample {
                    timestamp: 200,
                    value: 2.0,
                },
            ),
        ];
        let timestamps = service.ts_madd(&items).await.expect("madd");
        assert_eq!(timestamps, vec![100, 200]);
    }

    #[tokio::test]
    async fn test_ts_incr_by_delegates() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        let ts = service.ts_incr_by("key", 5.0, None).await.expect("incr_by");
        assert_eq!(ts, 1);
    }

    #[tokio::test]
    async fn test_ts_incr_by_with_timestamp() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        let ts = service
            .ts_incr_by("key", 5.0, Some(999))
            .await
            .expect("incr_by");
        assert_eq!(ts, 999);
    }

    #[tokio::test]
    async fn test_ts_decr_by_delegates() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        let ts = service.ts_decr_by("key", 3.0, None).await.expect("decr_by");
        assert_eq!(ts, 1);
    }

    #[tokio::test]
    async fn test_ts_decr_by_invalid_value() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        assert!(matches!(
            service.ts_decr_by("key", f64::NAN, None).await,
            Err(CacheError::InvalidInput(_))
        ));
        assert!(matches!(
            service.ts_decr_by("key", f64::INFINITY, None).await,
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn test_ts_del_delegates() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        let deleted = service.ts_del("key", 0, 100).await.expect("del");
        assert_eq!(deleted, 1);
    }

    #[tokio::test]
    async fn test_ts_del_invalid_range() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        assert!(matches!(
            service.ts_del("key", 100, 50).await,
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn test_ts_rev_range_delegates() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        let samples = service
            .ts_rev_range("key", 0, 100, TimeSeriesRangeOptions::default())
            .await
            .expect("rev_range");
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].timestamp, 2);
    }

    #[tokio::test]
    async fn test_ts_mrev_range_delegates() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        let results = service
            .ts_mrev_range(
                0,
                100,
                &["filter1".to_string()],
                TimeSeriesRangeOptions::default(),
            )
            .await
            .expect("mrev_range");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "filter1");
    }

    #[tokio::test]
    async fn test_ts_create_rule_delegates() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        service
            .ts_create_rule("source", "dest", TsAggregation::Avg, 60000)
            .await
            .expect("create_rule");
    }

    #[tokio::test]
    async fn test_ts_create_rule_zero_bucket() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        assert!(matches!(
            service
                .ts_create_rule("source", "dest", TsAggregation::Avg, 0)
                .await,
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn test_ts_delete_rule_delegates() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        service
            .ts_delete_rule("source", "dest")
            .await
            .expect("delete_rule");
    }

    async fn start_redis_stack() -> Option<(ContainerAsync<GenericImage>, String)> {
        let image =
            GenericImage::new("redis/redis-stack-server", "latest").with_exposed_port(6379.tcp());
        start_generic_redis_image(image, 6379, Duration::from_secs(2), "redis stack").await
    }

    async fn service_with_timeseries() -> Option<(ContainerAsync<GenericImage>, TimeSeriesService)>
    {
        let (container, redis_url) = start_redis_stack().await?;
        let pool = Arc::new(InstrumentedPool::new_for_tests_with_url(&redis_url).expect("pool"));
        let service = TimeSeriesService::new(pool);
        Some((container, service))
    }

    #[tokio::test]
    async fn test_timeseries_create_add_get_range_integration() {
        let Some((_container, service)) = service_with_timeseries().await else {
            return;
        };
        let mut labels = HashMap::new();
        labels.insert("sensor".to_string(), "temp".to_string());
        service
            .ts_create(
                "ts:temp",
                TimeSeriesCreateOptions {
                    retention_ms: None,
                    chunk_size: None,
                    duplicate_policy: None,
                    labels,
                },
            )
            .await
            .expect("create");
        service
            .ts_add(
                "ts:temp",
                TimeSeriesSample {
                    timestamp: 1000,
                    value: 21.5,
                },
            )
            .await
            .expect("add1");
        service
            .ts_add(
                "ts:temp",
                TimeSeriesSample {
                    timestamp: 2000,
                    value: 22.0,
                },
            )
            .await
            .expect("add2");

        let latest = service
            .ts_get("ts:temp")
            .await
            .expect("get")
            .expect("sample");
        assert_eq!(latest.timestamp, 2000);

        let samples = service
            .ts_range("ts:temp", 0, 3000, TimeSeriesRangeOptions::default())
            .await
            .expect("range");
        assert_eq!(samples.len(), 2);
    }

    #[tokio::test]
    async fn test_timeseries_mget_mrange_queryindex_integration() {
        let Some((_container, service)) = service_with_timeseries().await else {
            return;
        };
        for key in ["ts:a", "ts:b"] {
            let mut labels = HashMap::new();
            labels.insert("sensor".to_string(), "temp".to_string());
            labels.insert("host".to_string(), key.to_string());
            service
                .ts_create(
                    key,
                    TimeSeriesCreateOptions {
                        retention_ms: None,
                        chunk_size: None,
                        duplicate_policy: None,
                        labels,
                    },
                )
                .await
                .expect("create");
            service
                .ts_add(
                    key,
                    TimeSeriesSample {
                        timestamp: 1000,
                        value: 1.0,
                    },
                )
                .await
                .expect("add");
        }

        let mget = service
            .ts_mget(&["sensor=temp".to_string()])
            .await
            .expect("mget");
        assert!(!mget.is_empty());

        let mrange = service
            .ts_mrange(
                0,
                2000,
                &["sensor=temp".to_string()],
                TimeSeriesRangeOptions::default(),
            )
            .await
            .expect("mrange");
        assert!(!mrange.is_empty());

        let keys = service
            .ts_query_index(&["sensor=temp".to_string()])
            .await
            .expect("queryindex");
        assert!(keys.iter().any(|key| key == "ts:a" || key == "ts:b"));
    }

    #[tokio::test]
    async fn test_timeseries_info_and_rule_commands_integration() {
        let Some((_container, service)) = service_with_timeseries().await else {
            return;
        };
        let mut labels = HashMap::new();
        labels.insert("sensor".to_string(), "temp".to_string());
        service
            .ts_create(
                "source",
                TimeSeriesCreateOptions {
                    retention_ms: None,
                    chunk_size: None,
                    duplicate_policy: None,
                    labels: labels.clone(),
                },
            )
            .await
            .expect("create source");
        service
            .ts_create(
                "dest",
                TimeSeriesCreateOptions {
                    retention_ms: None,
                    chunk_size: None,
                    duplicate_policy: None,
                    labels,
                },
            )
            .await
            .expect("create dest");

        let info = service.ts_info("source").await.expect("info");
        assert!(info.is_object() || info.is_array());

        service
            .ts_create_rule("source", "dest", TsAggregation::Avg, 60000)
            .await
            .expect("create rule");
        service
            .ts_delete_rule("source", "dest")
            .await
            .expect("delete rule");
    }

    #[tokio::test]
    async fn test_ts_incr_by_negative_timestamp_rejected() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        assert!(matches!(
            service.ts_incr_by("key", 1.0, Some(-1)).await,
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn test_ts_decr_by_negative_timestamp_rejected() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        assert!(matches!(
            service.ts_decr_by("key", 1.0, Some(-1)).await,
            Err(CacheError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn test_validate_filters_empty_string_in_array() {
        let service =
            TimeSeriesService::new_with_repository(Arc::new(CaptureTimeSeriesRepo::default()));
        assert!(matches!(
            service
                .ts_mget(&["valid".to_string(), " ".to_string()])
                .await,
            Err(CacheError::InvalidInput(_))
        ));
    }
}
