use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct SetHashRequest {
    #[validate(length(min = 1))]
    pub items: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct SetHashNxRequest {
    #[validate(length(min = 1))]
    pub field: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct GetMultipleFieldsRequest {
    #[validate(length(min = 1))]
    pub fields: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HashIncrRequest {
    pub delta: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HashIncrFloatRequest {
    pub delta: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ScanHashQuery {
    #[serde(default)]
    pub cursor: u64,
    pub pattern: Option<String>,
    pub count: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RandomFieldQuery {
    pub count: Option<i64>,
    #[serde(default)]
    pub with_values: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HashFieldEntry {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HashScanResponse {
    pub cursor: u64,
    pub entries: Vec<HashFieldEntry>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HashRandomFieldResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<HashFieldEntry>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_query_defaults() {
        let query: ScanHashQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.cursor, 0);
        assert!(query.pattern.is_none());
        assert!(query.count.is_none());
    }

    #[test]
    fn test_hash_response_builders() {
        let entry = HashFieldEntry {
            field: "f".to_string(),
            value: "v".to_string(),
        };
        let scan = HashScanResponse {
            cursor: 0,
            entries: vec![entry],
        };
        assert_eq!(scan.entries.len(), 1);

        let random = HashRandomFieldResponse {
            fields: Some(vec!["f1".to_string()]),
            entries: None,
        };
        assert_eq!(random.fields.unwrap().len(), 1);
    }
}
