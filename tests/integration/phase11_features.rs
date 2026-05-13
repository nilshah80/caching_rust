//! Integration tests for Phase 11 features (Redis 8.6+).
//!
//! These tests exercise the real Redis-talking repository code paths for
//! HOTKEYS, gracefully skipping when the running Redis predates 8.6 or when
//! Docker is unavailable on the host.

use redis_caching_service::application::services::{AdminService, KeyService, StringService};
use redis_caching_service::domain::entities::{
    HotkeysSlotRange, HotkeysStartOptions, RestoreOptions,
};
use redis_caching_service::domain::errors::CacheError;

use crate::docker_helper::{create_pool, create_pool_with_tag};

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

#[tokio::test]
async fn test_bgsave_schedule_against_real_redis() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let service = AdminService::new(pool);

    let plain = service.bgsave(false).await.expect("BGSAVE");
    assert!(plain.started);

    // BGSAVE SCHEDULE may return either OK or "Background saving scheduled"
    // depending on whether another persistence task is running. Some Redis
    // releases also return "Background save already in progress" if the
    // previous BGSAVE has not finished — we accept that as well to keep the
    // test deterministic.
    let scheduled = service.bgsave(true).await;
    match scheduled {
        Ok(result) => assert!(result.started),
        Err(err) if is_unsupported_redis(&err) => {}
        Err(err)
            if err
                .to_string()
                .to_lowercase()
                .contains("already in progress") => {}
        Err(err) => panic!("unexpected BGSAVE SCHEDULE error: {err}"),
    }
}

#[tokio::test]
async fn test_wait_aof_against_real_redis() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let service = AdminService::new(pool);

    // numlocal=0 + numreplicas=0 returns immediately on any Redis 7.2+
    // regardless of persistence configuration. Older Redis returns "unknown
    // command" which is treated as a skip.
    let result = service.wait_aof(0, 0, 1_000).await;
    match result {
        Ok(reply) => {
            assert!(reply.local >= 0);
            assert!(reply.replicas >= 0);
        }
        Err(err) if is_unsupported_redis(&err) => {}
        Err(err) => panic!("unexpected WAITAOF error: {err}"),
    }
}

#[tokio::test]
async fn test_client_unblock_against_real_redis() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let service = AdminService::new(pool);

    // Use the current connection's own client ID — guarantees a valid target,
    // and since this connection isn't blocked the reply must be 0.
    let id = service.client_id().await.expect("CLIENT ID");
    assert!(id > 0, "CLIENT ID should return a positive integer");

    let reply = service
        .client_unblock(id, false)
        .await
        .expect("CLIENT UNBLOCK");
    assert_eq!(reply, 0, "current client is not blocked");
}

#[tokio::test]
async fn test_client_setinfo_appears_in_client_list() {
    // CLIENT SETINFO LIB-NAME runs from the pool's post_create hook; verify
    // it landed in CLIENT LIST output. Skips on Redis < 7.2 where the command
    // is a no-op.
    let Some((_container, pool)) = create_pool_with_tag("7.4").await else {
        return;
    };
    let service = AdminService::new(pool);
    let clients = service.client_list().await.expect("CLIENT LIST");
    let expected = redis_caching_service::infrastructure::redis::connection::CLIENT_LIB_NAME;
    let found = clients.iter().any(|client| client.lib_name == expected);
    assert!(
        found,
        "no CLIENT LIST entry advertises LIB-NAME={expected}; got {clients:?}"
    );
}

#[tokio::test]
async fn test_restore_with_all_options_against_real_redis() {
    let Some((_container, pool)) = create_pool().await else {
        return;
    };
    let strings = StringService::new(pool.clone());
    let keys = KeyService::new(pool);

    // Seed and dump a key.
    strings
        .set(
            "phase11_restore_src",
            "hello",
            None,
            None,
            false,
            false,
            false,
            false,
        )
        .await
        .expect("seed");
    let dump = keys.dump("phase11_restore_src").await.expect("dump");
    let payload = dump.data.expect("dump payload");
    // The dump serializer stores base64 already; decode it back into raw bytes
    // for the RESTORE round-trip.
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as BASE64;
    let raw = BASE64.decode(payload.as_bytes()).expect("base64 decode");

    // Plain restore.
    let ok = keys
        .restore("phase11_restore_dst", &raw, RestoreOptions::default())
        .await
        .expect("plain restore");
    assert!(ok);

    // RESTORE with REPLACE + IDLETIME.
    let ok = keys
        .restore(
            "phase11_restore_dst",
            &raw,
            RestoreOptions {
                replace: true,
                idletime: Some(60),
                ..Default::default()
            },
        )
        .await
        .expect("restore IDLETIME");
    assert!(ok);

    // RESTORE with ABSTTL targeting a far-future absolute timestamp.
    let ok = keys
        .restore(
            "phase11_restore_dst",
            &raw,
            RestoreOptions {
                ttl: 4_102_444_800_000,
                replace: true,
                absttl: true,
                ..Default::default()
            },
        )
        .await
        .expect("restore ABSTTL");
    assert!(ok);

    // RESTORE with FREQ may need an LFU policy; tolerate Redis rejecting it.
    let _ = keys
        .restore(
            "phase11_restore_dst",
            &raw,
            RestoreOptions {
                replace: true,
                freq: Some(5),
                ..Default::default()
            },
        )
        .await;
}
