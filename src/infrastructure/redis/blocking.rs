//! Helpers for Redis commands that intentionally block server-side.

use std::time::Duration;

use crate::domain::errors::CacheError;
use crate::infrastructure::redis::pool_connection::PoolConnection;

/// Extra client-side slack beyond the Redis server-side blocking timeout.
///
/// This keeps redis-rs response timeouts from racing Redis' own timeout while
/// still restoring the normal pool timeout immediately after the blocking call
/// in standalone mode. Cluster mode uses the same grace in its client-wide
/// response timeout because redis-rs does not expose per-command cluster
/// timeout mutation.
pub const BLOCKING_RESPONSE_TIMEOUT_GRACE: Duration = Duration::from_secs(1);

/// Run a blocking Redis command with a response timeout long enough for Redis
/// to return its own timeout response.
///
/// `reset_to_response_timeout` is the pool default to restore after the call.
/// redis-rs does not expose the connection's current timeout, so callers must
/// pass the explicit reset target instead of relying on save/restore semantics.
pub async fn query_with_blocking_timeout<T: redis::FromRedisValue>(
    conn: &mut PoolConnection,
    cmd: &mut redis::Cmd,
    blocking_timeout: Duration,
    reset_to_response_timeout: Option<Duration>,
) -> Result<T, CacheError> {
    let mut guard = ResponseTimeoutGuard::new(conn, blocking_timeout, reset_to_response_timeout);
    let result: Result<T, redis::RedisError> = cmd.query_async(guard.connection()).await;
    result.map_err(CacheError::RedisError)
}

fn response_timeout_for_blocking_command(blocking_timeout: Duration) -> Duration {
    blocking_timeout.saturating_add(BLOCKING_RESPONSE_TIMEOUT_GRACE)
}

struct ResponseTimeoutGuard<'a> {
    conn: &'a mut PoolConnection,
    reset_to: Option<Duration>,
}

impl<'a> ResponseTimeoutGuard<'a> {
    fn new(
        conn: &'a mut PoolConnection,
        blocking_timeout: Duration,
        reset_to: Option<Duration>,
    ) -> Self {
        let reset_to = reset_to.filter(|_| {
            conn.set_response_timeout(response_timeout_for_blocking_command(blocking_timeout))
        });

        Self { conn, reset_to }
    }

    fn connection(&mut self) -> &mut PoolConnection {
        self.conn
    }
}

impl Drop for ResponseTimeoutGuard<'_> {
    fn drop(&mut self) {
        if let Some(timeout) = self.reset_to {
            let _ = self.conn.set_response_timeout(timeout);
        }
    }
}
