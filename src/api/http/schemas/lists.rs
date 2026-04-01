//! List Schemas
//!
//! Request and response types for list API endpoints.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// Request to push values to a list
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ListPushRequest {
    /// Values to push to the list
    #[validate(length(min = 1, message = "At least one value is required"))]
    pub values: Vec<String>,
}

/// Request to pop values from a list
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListPopRequest {
    /// Number of elements to pop (default: 1)
    #[serde(default)]
    pub count: Option<u32>,
}

/// Query parameters for LRANGE
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListRangeQuery {
    /// Start index (inclusive, 0-based, negative counts from end)
    #[serde(default)]
    pub start: i64,
    /// Stop index (inclusive, -1 means end of list)
    #[serde(default = "default_stop")]
    pub stop: i64,
}

fn default_stop() -> i64 {
    -1
}

/// Request to get element at index
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListIndexQuery {
    /// Index of element to retrieve (0-based, negative counts from end)
    pub index: i64,
}

/// Request to set element at index
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ListSetRequest {
    /// Index to set value at
    pub index: i64,
    /// Value to set
    pub value: String,
}

/// Request to insert element relative to pivot
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ListInsertRequest {
    /// Position relative to pivot (before or after)
    pub position: InsertPositionParam,
    /// Pivot element to find
    pub pivot: String,
    /// Value to insert
    pub value: String,
}

/// Position for LINSERT
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InsertPositionParam {
    Before,
    After,
}

impl From<InsertPositionParam> for crate::domain::repositories::InsertPosition {
    fn from(pos: InsertPositionParam) -> Self {
        match pos {
            InsertPositionParam::Before => crate::domain::repositories::InsertPosition::Before,
            InsertPositionParam::After => crate::domain::repositories::InsertPosition::After,
        }
    }
}

/// Request to remove elements
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListRemoveRequest {
    /// Number of occurrences to remove (0 = all, positive = from head, negative = from tail)
    #[serde(default)]
    pub count: i64,
    /// Value to remove
    pub value: String,
}

/// Request to trim list
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListTrimRequest {
    /// Start index to keep
    pub start: i64,
    /// Stop index to keep (inclusive)
    pub stop: i64,
}

/// Query parameters for LPOS
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListPosQuery {
    /// Element to find
    pub element: String,
    /// Starting rank for search (0 = first, negative = from end)
    pub rank: Option<i64>,
    /// Number of matching indices to return
    pub count: Option<i64>,
    /// Maximum number of comparisons
    pub max_len: Option<i64>,
}

/// Direction for list move operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ListDirectionParam {
    Left,
    Right,
}

impl From<ListDirectionParam> for crate::domain::repositories::ListDirection {
    fn from(dir: ListDirectionParam) -> Self {
        match dir {
            ListDirectionParam::Left => crate::domain::repositories::ListDirection::Left,
            ListDirectionParam::Right => crate::domain::repositories::ListDirection::Right,
        }
    }
}

/// Request to move element between lists
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ListMoveRequest {
    /// Source list key
    #[validate(length(min = 1))]
    pub source: String,
    /// Destination list key
    #[validate(length(min = 1))]
    pub destination: String,
    /// Direction to pop from source (left or right)
    pub src_direction: ListDirectionParam,
    /// Direction to push to destination (left or right)
    pub dst_direction: ListDirectionParam,
}

/// Request for blocking pop operations
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct BlockingPopRequest {
    /// Keys to pop from (checked in order)
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
    /// Timeout in seconds (server-enforced max from configuration)
    #[validate(range(min = 1))]
    pub timeout_seconds: u32,
}

/// Request for blocking move operation
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct BlockingMoveRequest {
    /// Source list key
    #[validate(length(min = 1))]
    pub source: String,
    /// Destination list key
    #[validate(length(min = 1))]
    pub destination: String,
    /// Direction to pop from source (left or right)
    pub src_direction: ListDirectionParam,
    /// Direction to push to destination (left or right)
    pub dst_direction: ListDirectionParam,
    /// Timeout in seconds (server-enforced max from configuration)
    #[validate(range(min = 1))]
    pub timeout_seconds: u32,
}

/// Response from push operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ListPushResponse {
    /// New length of the list
    pub length: i64,
}

/// Response from pop operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ListPopResponse {
    /// Popped values (empty if list is empty)
    pub values: Vec<String>,
}

/// Response from blocking pop operations
#[derive(Debug, Serialize, ToSchema)]
pub struct BlockingPopResponse {
    /// Key that was popped from
    pub key: String,
    /// Value that was popped
    pub value: String,
}

/// Response from LPOS
#[derive(Debug, Serialize, ToSchema)]
pub struct ListPosResponse {
    /// Indices where element was found (empty if not found)
    pub indices: Vec<i64>,
}

/// Response from move operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ListMoveResponse {
    /// The moved element (None if source was empty)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Response for list length
#[derive(Debug, Serialize, ToSchema)]
pub struct ListLengthResponse {
    /// Length of the list
    pub length: i64,
}

/// Response for list element at index
#[derive(Debug, Serialize, ToSchema)]
pub struct ListIndexResponse {
    /// Element at the index (None if index out of range)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Response from insert operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ListInsertResponse {
    /// New length of list (-1 if pivot not found, 0 if list doesn't exist)
    pub length: i64,
}

/// Response from remove operations
#[derive(Debug, Serialize, ToSchema)]
pub struct ListRemoveResponse {
    /// Number of elements removed
    pub removed: i64,
}

/// Request for LMPOP operation
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct LMPopRequest {
    /// Keys to pop from (checked in order)
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
    /// Direction to pop from (left or right)
    pub direction: ListDirectionParam,
    /// Number of elements to pop (default: 1, must be >= 1)
    #[serde(default)]
    #[validate(range(min = 1, message = "Count must be at least 1"))]
    pub count: Option<u32>,
}

/// Request for BLMPOP operation
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct BLMPopRequest {
    /// Keys to pop from (checked in order)
    #[validate(length(min = 1, message = "At least one key is required"))]
    pub keys: Vec<String>,
    /// Direction to pop from (left or right)
    pub direction: ListDirectionParam,
    /// Timeout in seconds (server-enforced max from configuration)
    #[validate(range(min = 1))]
    pub timeout_seconds: u32,
    /// Number of elements to pop (default: 1, must be >= 1)
    #[serde(default)]
    #[validate(range(min = 1, message = "Count must be at least 1"))]
    pub count: Option<u32>,
}

/// Response from LMPOP/BLMPOP operations
#[derive(Debug, Serialize, ToSchema)]
pub struct LMPopResponse {
    /// Key that was popped from (None if no data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Popped elements
    pub elements: Vec<String>,
}

/// Query parameters for BLPOP/BRPOP SSE streaming endpoints
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct BlockingPopStreamQuery {
    /// Number of seconds between polls (default: 5, max: 30)
    #[serde(default = "default_poll_seconds")]
    pub poll_seconds: Option<u32>,
}

/// Query parameters for BLMPOP SSE streaming endpoint
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct BLMPopStreamQuery {
    /// Comma-separated list of keys to pop from
    pub keys: String,
    /// Direction to pop from (left or right)
    pub direction: ListDirectionParam,
    /// Number of seconds between polls (default: 5, max: 30)
    #[serde(default = "default_poll_seconds")]
    pub poll_seconds: Option<u32>,
    /// Number of elements to pop (default: 1, must be >= 1)
    pub count: Option<u32>,
}

fn default_poll_seconds() -> Option<u32> {
    Some(5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_range_query_defaults() {
        let query: ListRangeQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.start, 0);
        assert_eq!(query.stop, -1);
    }

    #[test]
    fn test_direction_conversion() {
        let left = ListDirectionParam::Left;
        let right = ListDirectionParam::Right;

        let left_domain: crate::domain::repositories::ListDirection = left.into();
        let right_domain: crate::domain::repositories::ListDirection = right.into();

        assert_eq!(
            left_domain,
            crate::domain::repositories::ListDirection::Left
        );
        assert_eq!(
            right_domain,
            crate::domain::repositories::ListDirection::Right
        );
    }

    #[test]
    fn test_insert_position_conversion() {
        let before = InsertPositionParam::Before;
        let after = InsertPositionParam::After;

        let before_domain: crate::domain::repositories::InsertPosition = before.into();
        let after_domain: crate::domain::repositories::InsertPosition = after.into();

        assert_eq!(
            before_domain,
            crate::domain::repositories::InsertPosition::Before
        );
        assert_eq!(
            after_domain,
            crate::domain::repositories::InsertPosition::After
        );
    }

    #[test]
    fn test_default_poll_seconds() {
        assert_eq!(default_poll_seconds(), Some(5));
    }

    #[test]
    fn test_blocking_pop_request_serialization() {
        let req = BlockingPopRequest {
            keys: vec!["list1".to_string(), "list2".to_string()],
            timeout_seconds: 10,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("list1"));
        assert!(json.contains("10"));
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn blocking_pop_empty_keys_fails() {
        let req = BlockingPopRequest {
            keys: vec![],
            timeout_seconds: 5,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn blocking_pop_timeout_zero_fails() {
        let req = BlockingPopRequest {
            keys: vec!["k1".into()],
            timeout_seconds: 0,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn blocking_pop_timeout_31_passes_server_clamps_later() {
        let req = BlockingPopRequest {
            keys: vec!["k1".into()],
            timeout_seconds: 31,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn blocking_pop_valid_passes() {
        let req = BlockingPopRequest {
            keys: vec!["k1".into()],
            timeout_seconds: 5,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn blocking_move_empty_source_fails() {
        let req = BlockingMoveRequest {
            source: "".into(),
            destination: "dst".into(),
            src_direction: ListDirectionParam::Left,
            dst_direction: ListDirectionParam::Right,
            timeout_seconds: 5,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn blocking_move_empty_destination_fails() {
        let req = BlockingMoveRequest {
            source: "src".into(),
            destination: "".into(),
            src_direction: ListDirectionParam::Left,
            dst_direction: ListDirectionParam::Right,
            timeout_seconds: 5,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn blocking_move_valid_passes() {
        let req = BlockingMoveRequest {
            source: "src".into(),
            destination: "dst".into(),
            src_direction: ListDirectionParam::Left,
            dst_direction: ListDirectionParam::Right,
            timeout_seconds: 5,
        };
        assert!(req.validate().is_ok());
    }

    // ========== LMPOP validation ==========

    #[test]
    fn lmpop_empty_keys_fails() {
        let req = LMPopRequest {
            keys: vec![],
            direction: ListDirectionParam::Left,
            count: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn lmpop_count_zero_fails() {
        let req = LMPopRequest {
            keys: vec!["k".into()],
            direction: ListDirectionParam::Left,
            count: Some(0),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn lmpop_valid_passes() {
        let req = LMPopRequest {
            keys: vec!["k".into()],
            direction: ListDirectionParam::Right,
            count: Some(5),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn lmpop_no_count_passes() {
        let req = LMPopRequest {
            keys: vec!["k".into()],
            direction: ListDirectionParam::Left,
            count: None,
        };
        assert!(req.validate().is_ok());
    }

    // ========== BLMPOP validation ==========

    #[test]
    fn blmpop_empty_keys_fails() {
        let req = BLMPopRequest {
            keys: vec![],
            direction: ListDirectionParam::Left,
            timeout_seconds: 5,
            count: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn blmpop_timeout_zero_fails() {
        let req = BLMPopRequest {
            keys: vec!["k".into()],
            direction: ListDirectionParam::Left,
            timeout_seconds: 0,
            count: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn blmpop_timeout_31_passes_server_clamps_later() {
        let req = BLMPopRequest {
            keys: vec!["k".into()],
            direction: ListDirectionParam::Left,
            timeout_seconds: 31,
            count: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn blmpop_count_zero_fails() {
        let req = BLMPopRequest {
            keys: vec!["k".into()],
            direction: ListDirectionParam::Left,
            timeout_seconds: 5,
            count: Some(0),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn blmpop_valid_passes() {
        let req = BLMPopRequest {
            keys: vec!["k".into()],
            direction: ListDirectionParam::Right,
            timeout_seconds: 10,
            count: Some(3),
        };
        assert!(req.validate().is_ok());
    }
}
