//! Integration tests for Phase 11 features (Redis 8.6+).
//!
//! These tests exercise the real Redis-talking repository code paths for
//! HOTKEYS, gracefully skipping when the running Redis predates 8.6 or when
//! Docker is unavailable on the host.

use redis_caching_service::application::services::AdminService;
use redis_caching_service::domain::entities::{HotkeysSlotRange, HotkeysStartOptions};
use redis_caching_service::domain::errors::CacheError;

use crate::docker_helper::create_pool_with_tag;

/// HOTKEYS landed in Redis 8.6 — pin the tag so the happy-path coverage on
/// `RedisAdminRepository::hotkeys_*` actually runs.
const HOTKEYS_REDIS_TAG: &str = "8.6";

/// Returns true when the error indicates the running Redis does not understand
/// the HOTKEYS family (typically Redis < 8.6). Mirrors the pattern used by the
/// hash-field-expiration integration tests.
fn is_unsupported_redis(err: &CacheError) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("unknown command") || msg.contains("unknown subcommand")
}

/// Redis 8.6 rejects `HOTKEYS START ... SLOTS ...` outside cluster mode, so
/// standalone containers will surface that as a `ResponseError`. The slot-range
/// integration test treats this as a successful proof that the SLOTS arguments
/// reached Redis correctly (i.e. the fix expanded the range into individual
/// slot numbers).
fn is_non_cluster_slots_error(err: &CacheError) -> bool {
    err.to_string().to_lowercase().contains("non-cluster mode")
}

#[tokio::test]
async fn test_hotkeys_lifecycle_against_real_redis() {
    let Some((_container, pool)) = create_pool_with_tag(HOTKEYS_REDIS_TAG).await else {
        return;
    };
    let service = AdminService::new(pool);

    // RESET first to make the test idempotent across reruns.
    let _ = service.hotkeys_reset().await;

    let start_result = service
        .hotkeys_start(HotkeysStartOptions {
            cpu: true,
            net: true,
            top_k: Some(5),
            duration_seconds: Some(60),
            sample_ratio: Some(100),
            slots: vec![],
        })
        .await;
    match start_result {
        Err(err) if is_unsupported_redis(&err) => return,
        Err(err) => panic!("unexpected HOTKEYS START error: {err}"),
        Ok(()) => {}
    }

    let get_result = service.hotkeys_get().await;
    match get_result {
        Err(err) if is_unsupported_redis(&err) => return,
        Err(err) => panic!("unexpected HOTKEYS GET error: {err}"),
        Ok(report) => {
            // Redis returns either a top-level map or an array of name/value
            // pairs depending on the build; both must surface as JSON.
            assert!(
                report.data.is_object() || report.data.is_array(),
                "HOTKEYS GET should return a structured reply, got {:?}",
                report.data
            );
        }
    }

    service.hotkeys_stop().await.expect("HOTKEYS STOP");
    service.hotkeys_reset().await.expect("HOTKEYS RESET");
}

#[tokio::test]
async fn test_hotkeys_start_with_slot_range_against_real_redis() {
    let Some((_container, pool)) = create_pool_with_tag(HOTKEYS_REDIS_TAG).await else {
        return;
    };
    let service = AdminService::new(pool);

    let _ = service.hotkeys_reset().await;

    // Slot ranges are expanded to individual slot numbers on the wire — this
    // exercises the fix for the SLOTS bug. Standalone Redis 8.6 rejects the
    // SLOTS argument entirely ("SLOTS parameter cannot be used in non-cluster
    // mode") which is itself proof that the expanded args reached Redis; the
    // happy path is exercised on a real cluster in tests/e2e/cluster_test.sh.
    let start_result = service
        .hotkeys_start(HotkeysStartOptions {
            cpu: true,
            net: false,
            top_k: Some(3),
            duration_seconds: Some(30),
            sample_ratio: Some(100),
            slots: vec![HotkeysSlotRange { start: 0, end: 31 }],
        })
        .await;
    match start_result {
        Err(err) if is_unsupported_redis(&err) => return,
        Err(err) if is_non_cluster_slots_error(&err) => {
            // Expected on standalone Redis 8.6 — the SLOTS args were accepted
            // by the parser and rejected at the cluster-mode check.
        }
        Err(err) => panic!("unexpected HOTKEYS START (slot range) error: {err}"),
        Ok(()) => {}
    }

    let _ = service.hotkeys_stop().await;
    let _ = service.hotkeys_reset().await;
}
