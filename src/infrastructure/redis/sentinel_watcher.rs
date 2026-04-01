//! Sentinel Failover Watcher
//!
//! Background task that periodically queries Sentinel for the current master address.
//! If the master changes (e.g. after failover), the watcher:
//! 1. Creates a new deadpool-redis Pool pointing at the new master
//! 2. Swaps it into the InstrumentedPool atomically via `swap_pool()`
//! 3. Logs the failover event
//!
//! The old pool's connections drain naturally as they're returned and discarded.

#[cfg(not(test))]
use crate::infrastructure::config::PoolConfig;
use crate::infrastructure::config::RedisConfig;
#[cfg(not(test))]
use crate::infrastructure::redis::connection::InstrumentedPool;
#[cfg(not(test))]
use std::sync::Arc;
#[cfg(not(test))]
use std::time::Duration;
#[cfg(not(test))]
use tokio::time::interval;
use tracing::warn;
#[cfg(not(test))]
use tracing::{error, info};

/// Start the sentinel watcher as a background task.
///
/// Returns a `JoinHandle` that runs until the process exits.
#[cfg(not(test))]
pub fn spawn_sentinel_watcher(
    pool: Arc<InstrumentedPool>,
    redis_config: RedisConfig,
    pool_config: PoolConfig,
) -> tokio::task::JoinHandle<()> {
    let poll_secs = redis_config.sentinel_poll_interval_secs.max(1);
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(poll_secs));

        // Skip the immediate first tick
        ticker.tick().await;

        loop {
            ticker.tick().await;

            let current_url = pool.resolved_url();

            match resolve_current_master(&redis_config).await {
                Ok(new_url) => {
                    if new_url != current_url {
                        warn!(
                            old_master = %mask_password(&current_url),
                            new_master = %mask_password(&new_url),
                            "Sentinel detected master change — initiating pool swap"
                        );

                        match pool.build_replacement_pool(&new_url, &pool_config).await {
                            Ok(new_pool) => {
                                pool.swap_pool(new_pool, new_url.clone());
                                info!(
                                    master = %mask_password(&new_url),
                                    "Pool swapped to new master successfully"
                                );
                            }
                            Err(e) => {
                                error!(
                                    new_master = %mask_password(&new_url),
                                    error = %e,
                                    "Failed to create pool for new master — keeping old pool"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Sentinel poll failed — will retry next interval");
                }
            }
        }
    })
}

/// Query sentinel for the current master address.
async fn resolve_current_master(config: &RedisConfig) -> Result<String, String> {
    let sentinel_urls = config.sentinel_node_urls();
    let master_name = &config.sentinel_master_name;

    for url in &sentinel_urls {
        let mut sentinel_info: redis::ConnectionInfo = url
            .parse()
            .map_err(|e| format!("invalid sentinel URL {url}: {e}"))?;

        if let Some(ref sentinel_pw) = config.sentinel_password {
            let redis_settings = sentinel_info
                .redis_settings()
                .clone()
                .set_password(sentinel_pw);
            sentinel_info = sentinel_info.set_redis_settings(redis_settings);
        }

        let client =
            redis::Client::open(sentinel_info).map_err(|e| format!("open sentinel: {e}"))?;

        let mut conn = match client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                warn!(sentinel = %url, error = %e, "Sentinel unreachable during poll");
                continue;
            }
        };

        let master_addr: Result<Vec<String>, _> = redis::cmd("SENTINEL")
            .arg("get-master-addr-by-name")
            .arg(master_name)
            .query_async(&mut conn)
            .await;

        match master_addr {
            Ok(addr) if addr.len() >= 2 => {
                let scheme = if config.tls_enabled {
                    "rediss"
                } else {
                    "redis"
                };
                let auth = config
                    .password
                    .as_ref()
                    .map_or(String::new(), |pw| format!(":{pw}@"));
                let db = if config.database > 0 {
                    format!("/{}", config.database)
                } else {
                    String::new()
                };
                let insecure = if config.tls_enabled && config.tls_skip_verify {
                    "#insecure"
                } else {
                    ""
                };
                return Ok(format!(
                    "{scheme}://{auth}{}:{}{db}{insecure}",
                    addr[0], addr[1]
                ));
            }
            Ok(_) => continue,
            Err(e) => {
                warn!(sentinel = %url, error = %e, "SENTINEL query failed during poll");
                continue;
            }
        }
    }

    Err(format!("no sentinel could resolve master '{master_name}'"))
}

fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.find('@')
        && let Some(colon_pos) = url[..at_pos].rfind(':')
    {
        return format!("{}:***@{}", &url[..colon_pos], &url[at_pos + 1..]);
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_password() {
        assert_eq!(
            mask_password("redis://:secret@host:6379"),
            "redis://:***@host:6379"
        );
        assert_eq!(mask_password("redis://host:6379"), "redis://host:6379");
    }

    #[test]
    fn test_mask_password_no_auth() {
        assert_eq!(mask_password("redis://host:6379/0"), "redis://host:6379/0");
    }

    #[test]
    fn test_mask_password_with_tls() {
        assert_eq!(
            mask_password("rediss://:pw@host:6380#insecure"),
            "rediss://:***@host:6380#insecure"
        );
    }

    #[tokio::test]
    async fn test_resolve_current_master_no_sentinels() {
        let config = RedisConfig {
            sentinel_enabled: true,
            sentinel_nodes: "redis://127.0.0.1:1".to_string(),
            sentinel_master_name: "mymaster".to_string(),
            ..RedisConfig::default()
        };
        let result = resolve_current_master(&config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no sentinel could resolve"));
    }

    // build_replacement_pool is #[cfg(not(test))] because it depends on
    // InstrumentedPool::build_pool which is also test-gated. Integration
    // testing of sentinel failover is done via the sentinel E2E test suite.
}
