//! Unified Pool Connection
//!
//! Wraps both standalone and cluster connections behind a single type
//! that repositories can use without knowing the topology.
//! Implements `redis::aio::ConnectionLike` by delegating to the inner connection.

use deadpool_redis::Connection as StandaloneConnection;
use redis::aio::ConnectionLike;
use redis::cluster_async::ClusterConnection;
use redis::{Cmd, Pipeline, RedisFuture, Value};

/// A connection that can be either standalone (from deadpool) or cluster-routed.
pub enum PoolConnection {
    Standalone(StandaloneConnection),
    Cluster(ClusterConnection),
}

impl ConnectionLike for PoolConnection {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        match self {
            Self::Standalone(c) => c.req_packed_command(cmd),
            Self::Cluster(c) => c.req_packed_command(cmd),
        }
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        match self {
            Self::Standalone(c) => c.req_packed_commands(cmd, offset, count),
            Self::Cluster(c) => c.req_packed_commands(cmd, offset, count),
        }
    }

    fn get_db(&self) -> i64 {
        match self {
            Self::Standalone(c) => c.get_db(),
            Self::Cluster(c) => c.get_db(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify PoolConnection implements ConnectionLike (compile-time check)
    fn _assert_connection_like(_: &dyn ConnectionLike) {}

    #[test]
    fn test_pool_connection_is_connection_like() {
        // This test verifies the trait implementation compiles.
        // We can't create real connections in unit tests.
    }
}
