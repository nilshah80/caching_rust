//! Admin Authentication Middleware
//!
//! API key authentication for admin endpoints.

use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;

/// Header name for the admin API key
pub const ADMIN_API_KEY_HEADER: &str = "X-Admin-Api-Key";

/// Validate admin API key (Bearer token variant for use with typed headers)
pub fn validate_admin_key(state: &AppState, token: &str) -> Result<(), CacheError> {
    if token == state.config.admin.api_key {
        Ok(())
    } else {
        Err(CacheError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state;

    #[test]
    fn test_validate_admin_key() {
        let (state, _, _, _) = test_state();

        let ok = validate_admin_key(&state, &state.config.admin.api_key);
        assert!(ok.is_ok());

        let err = validate_admin_key(&state, "invalid-key");
        assert!(matches!(err, Err(CacheError::Unauthorized)));
    }
}
