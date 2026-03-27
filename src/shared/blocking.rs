//! Blocking Timeout Enforcement
//!
//! Shared utility for enforcing consistent timeout bounds across all
//! blocking Redis operations (BLPOP, BRPOP, BZPOPMIN, XREAD BLOCK, etc.).
//!
//! Architecture Decision 3: All blocking operations enforce a maximum timeout
//! of 30 seconds to prevent HTTP timeouts and worker starvation.

use std::time::Duration;

/// Default maximum blocking timeout (30 seconds)
pub const MAX_BLOCKING_TIMEOUT_SECS: u64 = 30;

/// Minimum blocking timeout (1 second)
const MIN_BLOCKING_TIMEOUT_SECS: u64 = 1;

/// Enforces consistent timeout bounds for blocking Redis operations.
///
/// Used by all services that perform blocking operations to ensure
/// timeouts stay within safe bounds (min 1s, max configurable).
#[derive(Debug, Clone)]
pub struct BlockingTimeoutEnforcer {
    max_timeout: Duration,
}

impl BlockingTimeoutEnforcer {
    /// Create an enforcer with the default max timeout (30s)
    pub fn new() -> Self {
        Self {
            max_timeout: Duration::from_secs(MAX_BLOCKING_TIMEOUT_SECS),
        }
    }

    /// Create an enforcer with a custom max timeout
    pub fn with_max(max_timeout_secs: u64) -> Self {
        Self {
            max_timeout: Duration::from_secs(max_timeout_secs),
        }
    }

    /// Clamp a Duration to [1s, max]
    pub fn enforce(&self, requested: Duration) -> Duration {
        requested.clamp(
            Duration::from_secs(MIN_BLOCKING_TIMEOUT_SECS),
            self.max_timeout,
        )
    }

    /// Clamp a float seconds value to [1.0, max]
    pub fn enforce_secs_f64(&self, requested: f64) -> f64 {
        requested.clamp(
            MIN_BLOCKING_TIMEOUT_SECS as f64,
            self.max_timeout.as_secs_f64(),
        )
    }

    /// Clamp a u32 seconds value and return as Duration
    pub fn enforce_u32(&self, requested: u32) -> Duration {
        self.enforce(Duration::from_secs(requested as u64))
    }

    /// Get the maximum timeout
    pub fn max_timeout(&self) -> Duration {
        self.max_timeout
    }
}

impl Default for BlockingTimeoutEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_enforcer() {
        let enforcer = BlockingTimeoutEnforcer::new();
        assert_eq!(enforcer.max_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn test_enforce_clamps_zero_to_min() {
        let enforcer = BlockingTimeoutEnforcer::new();
        assert_eq!(enforcer.enforce(Duration::ZERO), Duration::from_secs(1));
    }

    #[test]
    fn test_enforce_clamps_above_max() {
        let enforcer = BlockingTimeoutEnforcer::new();
        assert_eq!(
            enforcer.enforce(Duration::from_secs(60)),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn test_enforce_passes_through_valid() {
        let enforcer = BlockingTimeoutEnforcer::new();
        assert_eq!(
            enforcer.enforce(Duration::from_secs(15)),
            Duration::from_secs(15)
        );
    }

    #[test]
    fn test_enforce_at_boundaries() {
        let enforcer = BlockingTimeoutEnforcer::new();
        assert_eq!(
            enforcer.enforce(Duration::from_secs(1)),
            Duration::from_secs(1)
        );
        assert_eq!(
            enforcer.enforce(Duration::from_secs(30)),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn test_enforce_secs_f64() {
        let enforcer = BlockingTimeoutEnforcer::new();
        assert_eq!(enforcer.enforce_secs_f64(0.0), 1.0);
        assert_eq!(enforcer.enforce_secs_f64(15.5), 15.5);
        assert_eq!(enforcer.enforce_secs_f64(45.0), 30.0);
    }

    #[test]
    fn test_enforce_u32() {
        let enforcer = BlockingTimeoutEnforcer::new();
        assert_eq!(enforcer.enforce_u32(0), Duration::from_secs(1));
        assert_eq!(enforcer.enforce_u32(10), Duration::from_secs(10));
        assert_eq!(enforcer.enforce_u32(50), Duration::from_secs(30));
    }

    #[test]
    fn test_custom_max() {
        let enforcer = BlockingTimeoutEnforcer::with_max(10);
        assert_eq!(
            enforcer.enforce(Duration::from_secs(15)),
            Duration::from_secs(10)
        );
        assert_eq!(
            enforcer.enforce(Duration::from_secs(5)),
            Duration::from_secs(5)
        );
    }
}
