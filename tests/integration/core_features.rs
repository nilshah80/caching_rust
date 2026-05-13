//! Integration tests for core Redis operations (strings, hashes, lists, sorted sets, keys).

use redis_caching_service::application::services::{
    HashService, KeyService, ListService, SortedSetService, StringService,
};
use redis_caching_service::domain::repositories::ScoredMember;
use redis_caching_service::infrastructure::redis::connection::PoolStats;

use crate::docker_helper::create_pool;

#[tokio::test]
async fn test_string_set_get_cycle() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let service = StringService::new(pool);

    // SET key
    service
        .set(
            "hello", "world", None, None, false, false, false, false, None,
        )
        .await
        .unwrap();

    // GET key
    let val = service
        .get("hello")
        .await
        .unwrap()
        .expect("key should exist");
    assert_eq!(val.value, "world");
}

#[tokio::test]
async fn test_hash_set_get_cycle() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let service = HashService::new(pool);

    // HSET
    let pairs = vec![
        ("field1".to_string(), "value1".to_string()),
        ("field2".to_string(), "value2".to_string()),
    ];
    let added = service.hset("myhash", pairs).await.unwrap();
    assert_eq!(added, 2);

    // HGETALL
    let all = service.hgetall("myhash").await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all.get("field1").unwrap(), "value1");
    assert_eq!(all.get("field2").unwrap(), "value2");
}

#[tokio::test]
async fn test_list_push_pop_cycle() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let service = ListService::new(pool);

    // RPUSH
    let len = service
        .rpush("mylist", vec!["a".into(), "b".into(), "c".into()])
        .await
        .unwrap();
    assert_eq!(len, 3);

    // LPOP one element (count=None uses single-value LPOP, compatible with all Redis versions)
    let popped = service.lpop("mylist", None).await.unwrap();
    assert_eq!(popped, vec!["a".to_string()]);

    // LPOP another
    let popped = service.lpop("mylist", None).await.unwrap();
    assert_eq!(popped, vec!["b".to_string()]);
}

#[tokio::test]
async fn test_sorted_set_add_range() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let service = SortedSetService::new(pool);

    let members = vec![
        ScoredMember {
            member: "alice".into(),
            score: 100.0,
        },
        ScoredMember {
            member: "bob".into(),
            score: 200.0,
        },
        ScoredMember {
            member: "charlie".into(),
            score: 150.0,
        },
    ];

    let result = service.zadd("myzset", members, None).await.unwrap();
    assert_eq!(result.count, 3);

    // ZRANGE 0 -1 (all members, ascending by score)
    let range = service.zrange("myzset", 0, -1, None).await.unwrap();
    assert_eq!(range.len(), 3);
    assert_eq!(range[0].member, "alice");
    assert_eq!(range[1].member, "charlie");
    assert_eq!(range[2].member, "bob");
}

#[tokio::test]
async fn test_key_exists_delete() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let string_svc = StringService::new(pool.clone());
    let key_svc = KeyService::new(pool);

    // SET a key
    string_svc
        .set(
            "tempkey", "tempval", None, None, false, false, false, false, None,
        )
        .await
        .unwrap();

    // EXISTS
    let exists = key_svc.exists(vec!["tempkey".into()]).await.unwrap();
    assert_eq!(exists.count, 1);

    // DEL
    let deleted = key_svc.delete(vec!["tempkey".into()]).await.unwrap();
    assert_eq!(deleted.count, 1);

    // EXISTS again — should be 0
    let exists = key_svc.exists(vec!["tempkey".into()]).await.unwrap();
    assert_eq!(exists.count, 0);
}

// =========================================================================
// Pool Metrics Integration Tests (8.2.3)
// =========================================================================

#[tokio::test]
async fn test_pool_metrics_track_connections() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };

    // Get a connection — this should increment total_wait_count
    let _conn = pool.get().await.unwrap();

    let stats: PoolStats = pool.get_stats();
    assert!(
        stats.total_wait_count >= 1,
        "total_wait_count should be at least 1 after getting a connection, got {}",
        stats.total_wait_count
    );
    assert!(
        stats.size > 0,
        "pool size should be greater than 0, got {}",
        stats.size
    );
}

#[tokio::test]
async fn test_pool_get_stats() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };

    // Make a few connections to accumulate stats
    let _conn1 = pool.get().await.unwrap();
    drop(_conn1);
    let _conn2 = pool.get().await.unwrap();
    drop(_conn2);
    let _conn3 = pool.get().await.unwrap();

    let stats: PoolStats = pool.get_stats();

    // We made 3 checkout requests
    assert!(
        stats.total_wait_count >= 3,
        "total_wait_count should be at least 3, got {}",
        stats.total_wait_count
    );
    // max_size was set to 4 in new_for_tests_with_url
    assert_eq!(stats.max_size, 4);
    // avg_wait_ms should be non-negative
    assert!(
        stats.avg_wait_ms >= 0.0,
        "avg_wait_ms should be non-negative, got {}",
        stats.avg_wait_ms
    );
    // No failures expected
    assert_eq!(
        stats.failed_checkouts, 0,
        "failed_checkouts should be 0, got {}",
        stats.failed_checkouts
    );
    // current_waiting should be 0 since we are not blocking
    assert_eq!(
        stats.current_waiting, 0,
        "current_waiting should be 0, got {}",
        stats.current_waiting
    );
}
