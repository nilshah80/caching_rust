//! Integration tests for core Redis operations (strings, hashes, lists, sorted sets, keys).

use std::sync::Arc;

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::redis::{Redis, REDIS_PORT};

use redis_caching_service::application::services::{
    HashService, KeyService, ListService, SortedSetService, StringService,
};
use redis_caching_service::domain::repositories::ScoredMember;
use redis_caching_service::infrastructure::redis::connection::InstrumentedPool;

async fn create_pool() -> (testcontainers::ContainerAsync<Redis>, Arc<InstrumentedPool>) {
    let container = Redis::default().start().await.unwrap();
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
    let url = format!("redis://{host}:{port}");
    let pool = Arc::new(InstrumentedPool::new_for_tests_with_url(&url).unwrap());
    (container, pool)
}

#[tokio::test]
async fn test_string_set_get_cycle() {
    let (_container, pool) = create_pool().await;
    let service = StringService::new(pool);

    // SET key
    service
        .set("hello", "world", None, None, false, false, false, false)
        .await
        .unwrap();

    // GET key
    let val = service.get("hello").await.unwrap().expect("key should exist");
    assert_eq!(val.value, "world");
}

#[tokio::test]
async fn test_hash_set_get_cycle() {
    let (_container, pool) = create_pool().await;
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
    let (_container, pool) = create_pool().await;
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
    let (_container, pool) = create_pool().await;
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
    let (_container, pool) = create_pool().await;
    let string_svc = StringService::new(pool.clone());
    let key_svc = KeyService::new(pool);

    // SET a key
    string_svc
        .set("tempkey", "tempval", None, None, false, false, false, false)
        .await
        .unwrap();

    // EXISTS
    let exists = key_svc
        .exists(vec!["tempkey".into()])
        .await
        .unwrap();
    assert_eq!(exists.count, 1);

    // DEL
    let deleted = key_svc
        .delete(vec!["tempkey".into()])
        .await
        .unwrap();
    assert_eq!(deleted.count, 1);

    // EXISTS again — should be 0
    let exists = key_svc
        .exists(vec!["tempkey".into()])
        .await
        .unwrap();
    assert_eq!(exists.count, 0);
}
