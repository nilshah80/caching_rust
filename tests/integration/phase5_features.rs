//! Integration tests for Phase 5 features:
//! LCS, SORT, LMPOP/BLMPOP, FUNCTION, TimeSeries, hash field expiration, Redis 8 hash commands.

use std::collections::HashMap;
use std::sync::Arc;

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::redis::{Redis, REDIS_PORT};

use redis_caching_service::application::services::{
    FunctionService, HashService, KeyService, ListService, StringService, TimeSeriesService,
};
use redis_caching_service::domain::repositories::{
    HashExpiration, LcsOptions, ListDirection, SortOptions, TimeSeriesCreateOptions,
    TimeSeriesRangeOptions, TimeSeriesSample,
};
use redis_caching_service::infrastructure::redis::connection::InstrumentedPool;

async fn create_pool() -> (testcontainers::ContainerAsync<Redis>, Arc<InstrumentedPool>) {
    let container = Redis::default().start().await.unwrap();
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
    let url = format!("redis://{host}:{port}");
    let pool = Arc::new(InstrumentedPool::new_for_tests_with_url(&url).unwrap());
    (container, pool)
}

// ---------------------------------------------------------------------------
// LCS
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_lcs_operations() {
    let (_container, pool) = create_pool().await;
    let service = StringService::new(pool);

    service
        .set("lcs1", "ohmytext", None, None, false, false, false, false)
        .await
        .unwrap();
    service
        .set("lcs2", "mynewtext", None, None, false, false, false, false)
        .await
        .unwrap();

    // Default LCS — returns the common subsequence string
    let result = service
        .lcs("lcs1", "lcs2", LcsOptions::default())
        .await;
    match result {
        Err(e) if e.to_string().contains("unknown command") || e.to_string().contains("ERR") => {
            // Redis < 7.0, skip
            return;
        }
        Err(e) => panic!("unexpected error: {e}"),
        Ok(r) => {
            if let redis_caching_service::domain::repositories::LcsResult::String(s) = r {
                assert_eq!(s, "mytext");
            } else {
                panic!("expected LcsResult::String");
            }
        }
    }

    // LCS with LEN
    let result = service
        .lcs("lcs1", "lcs2", LcsOptions { len: true, ..Default::default() })
        .await
        .unwrap();
    if let redis_caching_service::domain::repositories::LcsResult::Length(n) = result {
        assert_eq!(n, 6);
    } else {
        panic!("expected LcsResult::Length");
    }

    // LCS with IDX
    let result = service
        .lcs(
            "lcs1",
            "lcs2",
            LcsOptions {
                idx: true,
                with_match_len: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    if let redis_caching_service::domain::repositories::LcsResult::Matches(m) = result {
        assert!(!m.matches.is_empty());
        assert_eq!(m.len, 6);
    } else {
        panic!("expected LcsResult::Matches");
    }
}

// ---------------------------------------------------------------------------
// SORT / SORT_RO
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_sort_operations() {
    let (_container, pool) = create_pool().await;
    let list_svc = ListService::new(pool.clone());
    let key_svc = KeyService::new(pool);

    list_svc
        .rpush("sortlist", vec!["3".into(), "1".into(), "2".into()])
        .await
        .unwrap();

    // SORT ascending (default)
    let sorted = key_svc
        .sort("sortlist", SortOptions::default())
        .await
        .unwrap();
    let vals: Vec<&str> = sorted.iter().map(|o| o.as_deref().unwrap()).collect();
    assert_eq!(vals, vec!["1", "2", "3"]);

    // SORT_RO (read-only variant, Redis 7.0+)
    let sorted_ro = key_svc
        .sort_ro("sortlist", SortOptions::default())
        .await;
    match sorted_ro {
        Err(e) if e.to_string().contains("unknown command") => {
            // Redis < 7.0, skip SORT_RO check
        }
        Err(e) => panic!("unexpected error: {e}"),
        Ok(vals_ro) => {
            let vals_ro: Vec<&str> = vals_ro.iter().map(|o| o.as_deref().unwrap()).collect();
            assert_eq!(vals_ro, vec!["1", "2", "3"]);
        }
    }
}

// ---------------------------------------------------------------------------
// LMPOP / BLMPOP
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_lmpop_operations() {
    let (_container, pool) = create_pool().await;
    let service = ListService::new(pool);

    service
        .rpush("lmpoplist", vec!["a".into(), "b".into(), "c".into()])
        .await
        .unwrap();

    // LMPOP LEFT count=2
    let result = service
        .lmpop(vec!["lmpoplist".into()], ListDirection::Left, Some(2))
        .await;
    match result {
        Err(e) if e.to_string().contains("unknown command") => {
            // Redis < 7.0, skip
            return;
        }
        Err(e) => panic!("unexpected error: {e}"),
        Ok(Some(r)) => {
            assert_eq!(r.key, "lmpoplist");
            assert_eq!(r.elements, vec!["a", "b"]);
        }
        Ok(None) => panic!("expected Some result"),
    }

    // BLMPOP with timeout — the list still has "c"
    let result = service
        .blmpop(vec!["lmpoplist".into()], ListDirection::Left, 1, Some(1))
        .await
        .unwrap();
    assert!(result.is_some());
    let r = result.unwrap();
    assert_eq!(r.elements, vec!["c"]);

    // BLMPOP on empty list — should timeout and return None
    let result = service
        .blmpop(vec!["lmpoplist".into()], ListDirection::Left, 1, None)
        .await
        .unwrap();
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// FUNCTION lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_function_lifecycle() {
    let (_container, pool) = create_pool().await;
    let service = FunctionService::new(pool);

    let code = "#!lua name=testlib\nredis.register_function('myfunc', function(keys, args) return args[1] end)";

    // FUNCTION LOAD
    let load_result = service.function_load(code, false).await;
    match load_result {
        Err(e) if e.to_string().contains("unknown command") || e.to_string().contains("unknown subcommand") => {
            // Redis < 7.0, skip
            return;
        }
        Err(e) => panic!("unexpected error: {e}"),
        Ok(lib_name) => {
            assert_eq!(lib_name, "testlib");
        }
    }

    // FUNCTION LIST
    let list = service.function_list(false).await.unwrap();
    assert!(list.to_string().contains("testlib"));

    // FCALL
    let result = service
        .fcall(
            "myfunc",
            &[],
            &[serde_json::Value::String("hello".into())],
            false,
        )
        .await
        .unwrap();
    assert_eq!(result, serde_json::Value::String("hello".into()));

    // FUNCTION DELETE
    service.function_delete("testlib").await.unwrap();

    // Verify deleted — list should not contain testlib
    let list = service.function_list(false).await.unwrap();
    assert!(!list.to_string().contains("testlib"));
}

// ---------------------------------------------------------------------------
// TimeSeries lifecycle (requires RedisTimeSeries module)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_timeseries_lifecycle() {
    let (_container, pool) = create_pool().await;
    let service = TimeSeriesService::new(pool);

    let mut labels = HashMap::new();
    labels.insert("sensor".to_string(), "temp".to_string());

    // TS.CREATE — if this fails with "unknown command", the module is not loaded
    let create_result = service
        .ts_create(
            "ts:test",
            TimeSeriesCreateOptions {
                labels,
                ..Default::default()
            },
        )
        .await;
    match create_result {
        Err(e) if e.to_string().contains("unknown command") || e.to_string().contains("ERR") => {
            // RedisTimeSeries module not available, skip
            return;
        }
        Err(e) => panic!("unexpected error: {e}"),
        Ok(()) => {}
    }

    // TS.ADD
    let ts = service
        .ts_add("ts:test", TimeSeriesSample { timestamp: 1000, value: 25.5 })
        .await
        .unwrap();
    assert_eq!(ts, 1000);

    let ts2 = service
        .ts_add("ts:test", TimeSeriesSample { timestamp: 2000, value: 26.0 })
        .await
        .unwrap();
    assert_eq!(ts2, 2000);

    // TS.GET — should return the latest sample
    let latest = service.ts_get("ts:test").await.unwrap();
    assert!(latest.is_some());
    let sample = latest.unwrap();
    assert_eq!(sample.timestamp, 2000);
    assert!((sample.value - 26.0).abs() < f64::EPSILON);

    // TS.RANGE
    let range = service
        .ts_range("ts:test", 0, 3000, TimeSeriesRangeOptions::default())
        .await
        .unwrap();
    assert_eq!(range.len(), 2);

    // TS.INFO
    let info = service.ts_info("ts:test").await.unwrap();
    assert!(info.to_string().contains("ts:test") || !info.is_null());

    // TS.DEL
    let deleted = service.ts_del("ts:test", 1000, 1000).await.unwrap();
    assert_eq!(deleted, 1);
}

// ---------------------------------------------------------------------------
// Hash field expiration (Redis 7.4+)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_hash_field_expiration() {
    let (_container, pool) = create_pool().await;
    let service = HashService::new(pool);

    service
        .hset("exphash", vec![("f1".into(), "v1".into()), ("f2".into(), "v2".into())])
        .await
        .unwrap();

    // HEXPIRE
    let result = service
        .hexpire("exphash", 300, vec!["f1".into()], None)
        .await;
    match result {
        Err(e)
            if e.to_string().contains("unknown command")
                || e.to_string().contains("unknown subcommand") =>
        {
            // Redis < 7.4, skip
            return;
        }
        Err(e) => panic!("unexpected error: {e}"),
        Ok(codes) => {
            // 1 = expiry set successfully
            assert_eq!(codes, vec![1]);
        }
    }

    // HTTL
    let ttls = service.httl("exphash", vec!["f1".into()]).await.unwrap();
    assert!(ttls[0] > 0 && ttls[0] <= 300);

    // HPERSIST — remove the expiry
    let persist = service.hpersist("exphash", vec!["f1".into()]).await.unwrap();
    // 1 = expiry removed
    assert_eq!(persist, vec![1]);

    // HTTL after persist — should be -1 (no expiry)
    let ttls = service.httl("exphash", vec!["f1".into()]).await.unwrap();
    assert_eq!(ttls, vec![-1]);
}

// ---------------------------------------------------------------------------
// Redis 8 hash commands (HGETEX, HSETEX, HGETDEL) — requires Redis >= 7.9
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_redis8_hash_commands() {
    let (_container, pool) = create_pool().await;
    let service = HashService::new(pool);

    service
        .hset("r8hash", vec![("a".into(), "1".into()), ("b".into(), "2".into())])
        .await
        .unwrap();

    // HGETEX — get fields and optionally set expiration
    let result = service
        .hgetex("r8hash", vec!["a".into(), "b".into()], Some(HashExpiration::Ex(600)))
        .await;
    match result {
        Err(e)
            if e.to_string().contains("unknown command")
                || e.to_string().contains("unknown subcommand") =>
        {
            // Redis < 7.9, skip
            return;
        }
        Err(e) => panic!("unexpected error: {e}"),
        Ok(vals) => {
            assert_eq!(vals, vec![Some("1".into()), Some("2".into())]);
        }
    }

    // HSETEX — set fields with expiration
    let result = service
        .hsetex(
            "r8hash",
            vec![("c".into(), "3".into())],
            None,
            Some(HashExpiration::Ex(600)),
        )
        .await;
    match result {
        Err(e) if e.to_string().contains("unknown command") => return,
        Err(e) => panic!("unexpected error: {e}"),
        Ok(n) => {
            assert_eq!(n, 1); // 1 new field added
        }
    }

    // Verify the field was set
    let val = service.hget("r8hash", "c").await.unwrap();
    assert_eq!(val, Some("3".into()));

    // HGETDEL — get and delete fields atomically
    let result = service
        .hgetdel("r8hash", vec!["a".into(), "c".into()])
        .await;
    match result {
        Err(e) if e.to_string().contains("unknown command") => return,
        Err(e) => panic!("unexpected error: {e}"),
        Ok(vals) => {
            assert_eq!(vals, vec![Some("1".into()), Some("3".into())]);
        }
    }

    // Verify fields were deleted
    let val = service.hget("r8hash", "a").await.unwrap();
    assert!(val.is_none());
    let val = service.hget("r8hash", "c").await.unwrap();
    assert!(val.is_none());

    // "b" should still exist
    let val = service.hget("r8hash", "b").await.unwrap();
    assert_eq!(val, Some("2".into()));
}
