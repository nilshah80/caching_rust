//! Set Schemas
//!
//! Request and response types for set API endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request to add members to a set
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct SetAddRequest {
    /// Members to add to the set
    #[validate(length(min = 1, message = "At least one member is required"))]
    pub members: Vec<String>,
}

/// Request to remove members from a set
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct SetRemoveRequest {
    /// Members to remove from the set
    #[validate(length(min = 1, message = "At least one member is required"))]
    pub members: Vec<String>,
}

/// Request to check if a member exists
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SetIsMemberRequest {
    /// Member to check
    pub member: String,
}

/// Request to check if multiple members exist
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct SetMIsMemberRequest {
    /// Members to check
    #[validate(length(min = 1, message = "At least one member is required"))]
    pub members: Vec<String>,
}

/// Query parameters for SRANDMEMBER
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SetRandMemberQuery {
    /// Number of members to return (positive = distinct, negative = may repeat)
    pub count: Option<i64>,
}

/// Request to pop random members from a set
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SetPopRequest {
    /// Number of members to pop (default: 1)
    #[serde(default)]
    pub count: Option<u32>,
}

/// Request to move a member between sets
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct SetMoveRequest {
    /// Source set key
    #[validate(length(min = 1))]
    pub source: String,
    /// Destination set key
    #[validate(length(min = 1))]
    pub destination: String,
    /// Member to move
    pub member: String,
}

/// Request for set algebra operations (SINTER, SUNION, SDIFF)
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct SetAlgebraRequest {
    /// Keys of sets to operate on
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
}

/// Request for set algebra store operations (SINTERSTORE, SUNIONSTORE, SDIFFSTORE)
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct SetAlgebraStoreRequest {
    /// Destination key to store result
    #[validate(length(min = 1))]
    pub destination: String,
    /// Keys of sets to operate on
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
}

/// Request for SINTERCARD operation
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct SetInterCardRequest {
    /// Keys of sets to operate on
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
    /// Optional limit to stop early
    pub limit: Option<u64>,
}

/// Query parameters for SSCAN
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SetScanQuery {
    /// Cursor position (0 to start)
    #[serde(default)]
    pub cursor: u64,
    /// Pattern to match members
    pub pattern: Option<String>,
    /// Hint for number of members to return per call
    pub count: Option<u64>,
}

/// Response from add operations
#[derive(Debug, Serialize, ToSchema)]
pub struct SetAddResponse {
    /// Number of members that were added (not already in the set)
    pub added: i64,
}

/// Response from remove operations
#[derive(Debug, Serialize, ToSchema)]
pub struct SetRemoveResponse {
    /// Number of members that were removed
    pub removed: i64,
}

/// Response for set members
#[derive(Debug, Serialize, ToSchema)]
pub struct SetMembersResponse {
    /// All members of the set
    pub members: Vec<String>,
}

/// Response from SISMEMBER
#[derive(Debug, Serialize, ToSchema)]
pub struct SetIsMemberResponse {
    /// Whether the member exists in the set
    pub is_member: bool,
}

/// Response from SMISMEMBER
#[derive(Debug, Serialize, ToSchema)]
pub struct SetMIsMemberResponse {
    /// For each member, whether it exists in the set
    pub results: Vec<bool>,
}

/// Response for set cardinality
#[derive(Debug, Serialize, ToSchema)]
pub struct SetCardResponse {
    /// Number of members in the set
    pub cardinality: i64,
}

/// Response from random member operations
#[derive(Debug, Serialize, ToSchema)]
pub struct SetRandMemberResponse {
    /// Random members from the set
    pub members: Vec<String>,
}

/// Response from pop operations
#[derive(Debug, Serialize, ToSchema)]
pub struct SetPopResponse {
    /// Members that were popped from the set
    pub members: Vec<String>,
}

/// Response from SMOVE
#[derive(Debug, Serialize, ToSchema)]
pub struct SetMoveResponse {
    /// Whether the member was moved (true if found and moved, false if not in source)
    pub moved: bool,
}

/// Response for set algebra operations
#[derive(Debug, Serialize, ToSchema)]
pub struct SetAlgebraResponse {
    /// Resulting members from the set operation
    pub members: Vec<String>,
}

/// Response for set algebra store operations
#[derive(Debug, Serialize, ToSchema)]
pub struct SetAlgebraStoreResponse {
    /// Number of members in the resulting set
    pub count: i64,
}

/// Response from SINTERCARD
#[derive(Debug, Serialize, ToSchema)]
pub struct SetInterCardResponse {
    /// Cardinality of the intersection
    pub cardinality: i64,
}

/// Response from SSCAN
#[derive(Debug, Serialize, ToSchema)]
pub struct SetScanResponse {
    /// Cursor for next iteration (0 = complete)
    pub cursor: u64,
    /// Members returned in this batch
    pub members: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_add_request() {
        let req: SetAddRequest = serde_json::from_str(r#"{"members": ["a", "b", "c"]}"#).unwrap();
        assert_eq!(req.members.len(), 3);
    }

    #[test]
    fn test_set_algebra_request() {
        let req: SetAlgebraRequest = serde_json::from_str(r#"{"keys": ["set1", "set2"]}"#).unwrap();
        assert_eq!(req.keys.len(), 2);
    }

    #[test]
    fn test_set_scan_query_defaults() {
        let query: SetScanQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.cursor, 0);
        assert!(query.pattern.is_none());
        assert!(query.count.is_none());
    }

    #[test]
    fn test_set_pop_request_default() {
        let req: SetPopRequest = serde_json::from_str(r#"{}"#).unwrap();
        assert!(req.count.is_none());
    }
}
