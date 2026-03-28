//! Cluster API Schemas
//!
//! Request and response types for Redis Cluster endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response for CLUSTER KEYSLOT
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct KeySlotResponse {
    pub key: String,
    pub slot: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyslot_response_serialization() {
        let resp = KeySlotResponse {
            key: "test".to_string(),
            slot: 12539,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("12539"));
        assert!(json.contains("test"));
    }
}
