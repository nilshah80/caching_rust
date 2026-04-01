//! Integration tests for error scenarios (8.2.7).

use redis_caching_service::application::services::{ListService, StringService};
use redis_caching_service::domain::errors::CacheError;
use redis_caching_service::infrastructure::redis::connection::InstrumentedPool;

use crate::docker_helper::create_pool;

#[tokio::test]
async fn test_connection_to_invalid_redis_fails() {
    // Attempt to create a pool pointing at a port where nothing is listening.
    let result = InstrumentedPool::new_for_tests_with_url("redis://127.0.0.1:1");

    match result {
        Ok(pool) => {
            // Pool creation may succeed (lazy connect); getting a connection should fail.
            let conn_result = pool.get().await;
            assert!(
                conn_result.is_err(),
                "Expected connection to invalid Redis to fail"
            );
        }
        Err(e) => {
            // Pool creation itself failed — also acceptable.
            assert!(
                matches!(
                    e,
                    CacheError::ConnectionFailed(_) | CacheError::PoolError(_)
                ),
                "Expected ConnectionFailed or PoolError, got: {:?}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_key_not_found_returns_none() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let service = StringService::new(pool);

    // GET a key that was never set — should return None, not an error.
    let result = service.get("nonexistent_key_12345").await.unwrap();
    assert!(
        result.is_none(),
        "Expected None for a non-existent key, got: {:?}",
        result
    );
}

#[tokio::test]
async fn test_invalid_command_on_wrong_type() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let string_svc = StringService::new(pool.clone());
    let list_svc = ListService::new(pool);

    // SET a string key
    string_svc
        .set(
            "wrongtype_key",
            "hello",
            None,
            None,
            false,
            false,
            false,
            false,
        )
        .await
        .unwrap();

    // Try LPUSH on a string key — Redis should return a WRONGTYPE error.
    let result = list_svc.lpush("wrongtype_key", vec!["value".into()]).await;

    assert!(
        result.is_err(),
        "Expected an error when LPUSHing to a string key"
    );
    let err = result.unwrap_err();
    // The error should be a RedisError (WRONGTYPE).
    assert!(
        matches!(err, CacheError::RedisError(_)),
        "Expected RedisError for type mismatch, got: {:?}",
        err
    );
}

#[tokio::test]
async fn test_expired_key_returns_none() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let service = StringService::new(pool);

    // SET with 1-second TTL
    service
        .set(
            "expiring_key",
            "temp_value",
            Some(1),
            None,
            false,
            false,
            false,
            false,
        )
        .await
        .unwrap();

    // Verify the key exists right now.
    let val = service.get("expiring_key").await.unwrap();
    assert!(val.is_some(), "Key should exist immediately after SET");

    // Wait 2 seconds for the key to expire.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // GET should return None after expiration.
    let val = service.get("expiring_key").await.unwrap();
    assert!(
        val.is_none(),
        "Expected None for expired key, got: {:?}",
        val
    );
}
