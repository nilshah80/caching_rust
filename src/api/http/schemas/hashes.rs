use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domain::repositories::{ExpireCondition, HSetExCondition, HashExpiration};

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

/// Condition for hash field expiration commands.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExpireConditionSchema {
    /// Set expiry only when the field has no expiry
    Nx,
    /// Set expiry only when the field has an existing expiry
    Xx,
    /// Set expiry only when the new expiry is greater than current one
    Gt,
    /// Set expiry only when the new expiry is less than current one
    Lt,
}

impl From<ExpireConditionSchema> for ExpireCondition {
    fn from(schema: ExpireConditionSchema) -> Self {
        match schema {
            ExpireConditionSchema::Nx => ExpireCondition::NX,
            ExpireConditionSchema::Xx => ExpireCondition::XX,
            ExpireConditionSchema::Gt => ExpireCondition::GT,
            ExpireConditionSchema::Lt => ExpireCondition::LT,
        }
    }
}

/// Request body for HEXPIRE command.
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct HExpireRequest {
    #[validate(length(min = 1, message = "At least one field required"))]
    pub fields: Vec<String>,
    pub seconds: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<ExpireConditionSchema>,
}

/// Request body for HPEXPIRE command.
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct HPExpireRequest {
    #[validate(length(min = 1, message = "At least one field required"))]
    pub fields: Vec<String>,
    pub milliseconds: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<ExpireConditionSchema>,
}

/// Request body for HEXPIREAT command.
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct HExpireAtRequest {
    #[validate(length(min = 1, message = "At least one field required"))]
    pub fields: Vec<String>,
    pub unix_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<ExpireConditionSchema>,
}

/// Request body for HPEXPIREAT command.
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct HPExpireAtRequest {
    #[validate(length(min = 1, message = "At least one field required"))]
    pub fields: Vec<String>,
    pub unix_time_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<ExpireConditionSchema>,
}

/// Request body for field query commands (HEXPIRETIME, HPEXPIRETIME, HTTL, HPTTL, HPERSIST).
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct HFieldsRequest {
    #[validate(length(min = 1, message = "At least one field required"))]
    pub fields: Vec<String>,
}

/// Result for a single field in hash field expiration responses.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HExpireFieldResult {
    pub field: String,
    pub result: i64,
}

/// Response for hash field expiration commands.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HExpireResponse {
    pub results: Vec<HExpireFieldResult>,
}

// --- Redis 8.0+ Hash schemas ---

/// Expiration options for HGETEX (no KEEPTTL).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HGetExExpirationSchema {
    /// Set expiry in seconds
    Ex(i64),
    /// Set expiry in milliseconds
    Px(i64),
    /// Set expiry as unix timestamp (seconds)
    Exat(i64),
    /// Set expiry as unix timestamp (milliseconds)
    Pxat(i64),
    /// Remove existing expiry
    Persist,
}

impl From<HGetExExpirationSchema> for HashExpiration {
    fn from(schema: HGetExExpirationSchema) -> Self {
        match schema {
            HGetExExpirationSchema::Ex(s) => HashExpiration::Ex(s),
            HGetExExpirationSchema::Px(ms) => HashExpiration::Px(ms),
            HGetExExpirationSchema::Exat(ts) => HashExpiration::Exat(ts),
            HGetExExpirationSchema::Pxat(ts) => HashExpiration::Pxat(ts),
            HGetExExpirationSchema::Persist => HashExpiration::Persist,
        }
    }
}

/// Expiration options for HSETEX (no PERSIST, has KEEPTTL).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HSetExExpirationSchema {
    /// Set expiry in seconds
    Ex(i64),
    /// Set expiry in milliseconds
    Px(i64),
    /// Set expiry as unix timestamp (seconds)
    Exat(i64),
    /// Set expiry as unix timestamp (milliseconds)
    Pxat(i64),
    /// Keep existing TTL
    Keepttl,
}

impl From<HSetExExpirationSchema> for HashExpiration {
    fn from(schema: HSetExExpirationSchema) -> Self {
        match schema {
            HSetExExpirationSchema::Ex(s) => HashExpiration::Ex(s),
            HSetExExpirationSchema::Px(ms) => HashExpiration::Px(ms),
            HSetExExpirationSchema::Exat(ts) => HashExpiration::Exat(ts),
            HSetExExpirationSchema::Pxat(ts) => HashExpiration::Pxat(ts),
            HSetExExpirationSchema::Keepttl => HashExpiration::Keepttl,
        }
    }
}

/// Condition for HSETEX command.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum HSetExConditionSchema {
    /// Set field only if it does not already exist
    Fnx,
    /// Set field only if it already exists
    Fxx,
}

impl From<HSetExConditionSchema> for HSetExCondition {
    fn from(schema: HSetExConditionSchema) -> Self {
        match schema {
            HSetExConditionSchema::Fnx => HSetExCondition::FNX,
            HSetExConditionSchema::Fxx => HSetExCondition::FXX,
        }
    }
}

/// Request body for HGETEX command (Redis 8.0+).
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct HGetExRequest {
    /// Fields to get values for
    #[validate(length(min = 1, message = "At least one field required"))]
    pub fields: Vec<String>,
    /// Optional expiration to set/remove on the fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<HGetExExpirationSchema>,
}

/// Request body for HSETEX command (Redis 8.0+).
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct HSetExRequest {
    /// Field-value pairs to set
    #[validate(length(min = 1, message = "At least one field required"))]
    pub fields: HashMap<String, String>,
    /// Optional condition for setting fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<HSetExConditionSchema>,
    /// Optional expiration for the fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<HSetExExpirationSchema>,
}

/// Request body for HGETDEL command (Redis 8.0+).
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct HGetDelRequest {
    /// Fields to get values for and then delete
    #[validate(length(min = 1, message = "At least one field required"))]
    pub fields: Vec<String>,
}

/// Response for HGETEX command.
#[derive(Debug, Serialize, ToSchema)]
pub struct HGetExResponse {
    pub values: Vec<Option<String>>,
}

/// Response for HSETEX command.
#[derive(Debug, Serialize, ToSchema)]
pub struct HSetExResponse {
    pub count: i64,
}

/// Response for HGETDEL command.
#[derive(Debug, Serialize, ToSchema)]
pub struct HGetDelResponse {
    pub values: Vec<Option<String>>,
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

    #[test]
    fn test_hexpire_request_empty_fields_fails() {
        let req = HExpireRequest {
            fields: vec![],
            seconds: 10,
            condition: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_hexpire_request_valid_passes() {
        let req = HExpireRequest {
            fields: vec!["f1".to_string()],
            seconds: 10,
            condition: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_hexpire_request_with_condition() {
        let req = HExpireRequest {
            fields: vec!["f1".to_string()],
            seconds: 10,
            condition: Some(ExpireConditionSchema::Nx),
        };
        assert!(req.validate().is_ok());
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("NX"));
    }

    #[test]
    fn test_hpexpire_request_empty_fields_fails() {
        let req = HPExpireRequest {
            fields: vec![],
            milliseconds: 1000,
            condition: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_hpexpire_request_valid_passes() {
        let req = HPExpireRequest {
            fields: vec!["f1".to_string()],
            milliseconds: 1000,
            condition: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_hexpire_at_request_empty_fields_fails() {
        let req = HExpireAtRequest {
            fields: vec![],
            unix_time: 1000,
            condition: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_hexpire_at_request_valid_passes() {
        let req = HExpireAtRequest {
            fields: vec!["f1".to_string()],
            unix_time: 1000,
            condition: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_hpexpire_at_request_empty_fields_fails() {
        let req = HPExpireAtRequest {
            fields: vec![],
            unix_time_ms: 1000,
            condition: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_hpexpire_at_request_valid_passes() {
        let req = HPExpireAtRequest {
            fields: vec!["f1".to_string()],
            unix_time_ms: 1000,
            condition: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_hfields_request_empty_fields_fails() {
        let req = HFieldsRequest { fields: vec![] };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_hfields_request_valid_passes() {
        let req = HFieldsRequest {
            fields: vec!["f1".to_string()],
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_expire_condition_schema_conversions() {
        use crate::domain::repositories::ExpireCondition;

        let nx: ExpireCondition = ExpireConditionSchema::Nx.into();
        assert_eq!(nx.as_str(), "NX");
        let xx: ExpireCondition = ExpireConditionSchema::Xx.into();
        assert_eq!(xx.as_str(), "XX");
        let gt: ExpireCondition = ExpireConditionSchema::Gt.into();
        assert_eq!(gt.as_str(), "GT");
        let lt: ExpireCondition = ExpireConditionSchema::Lt.into();
        assert_eq!(lt.as_str(), "LT");
    }

    #[test]
    fn test_hexpire_field_result_and_response() {
        let result = HExpireFieldResult {
            field: "f1".to_string(),
            result: 1,
        };
        let response = HExpireResponse {
            results: vec![result],
        };
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].field, "f1");
        assert_eq!(response.results[0].result, 1);

        // Test serialization
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("f1"));
    }

    #[test]
    fn test_expire_condition_schema_deserialization() {
        let nx: ExpireConditionSchema = serde_json::from_str(r#""NX""#).unwrap();
        assert!(matches!(nx, ExpireConditionSchema::Nx));
        let xx: ExpireConditionSchema = serde_json::from_str(r#""XX""#).unwrap();
        assert!(matches!(xx, ExpireConditionSchema::Xx));
        let gt: ExpireConditionSchema = serde_json::from_str(r#""GT""#).unwrap();
        assert!(matches!(gt, ExpireConditionSchema::Gt));
        let lt: ExpireConditionSchema = serde_json::from_str(r#""LT""#).unwrap();
        assert!(matches!(lt, ExpireConditionSchema::Lt));
    }

    #[test]
    fn test_hexpire_request_condition_skipped_when_none() {
        let req = HExpireRequest {
            fields: vec!["f1".to_string()],
            seconds: 10,
            condition: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("condition"));
    }

    // --- HGETEX schema tests ---

    #[test]
    fn test_hgetex_request_empty_fields_fails() {
        let req = HGetExRequest {
            fields: vec![],
            expiration: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_hgetex_request_valid_passes() {
        let req = HGetExRequest {
            fields: vec!["f1".to_string()],
            expiration: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_hgetex_request_with_expiration() {
        let req = HGetExRequest {
            fields: vec!["f1".to_string()],
            expiration: Some(HGetExExpirationSchema::Ex(60)),
        };
        assert!(req.validate().is_ok());
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("ex"));
    }

    #[test]
    fn test_hgetex_request_expiration_skipped_when_none() {
        let req = HGetExRequest {
            fields: vec!["f1".to_string()],
            expiration: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("expiration"));
    }

    #[test]
    fn test_hgetex_expiration_schema_all_variants() {
        use crate::domain::repositories::HashExpiration;

        let ex: HashExpiration = HGetExExpirationSchema::Ex(60).into();
        assert!(matches!(ex, HashExpiration::Ex(60)));

        let px: HashExpiration = HGetExExpirationSchema::Px(1000).into();
        assert!(matches!(px, HashExpiration::Px(1000)));

        let exat: HashExpiration = HGetExExpirationSchema::Exat(1700000000).into();
        assert!(matches!(exat, HashExpiration::Exat(1700000000)));

        let pxat: HashExpiration = HGetExExpirationSchema::Pxat(1700000000000).into();
        assert!(matches!(pxat, HashExpiration::Pxat(1700000000000)));

        let persist: HashExpiration = HGetExExpirationSchema::Persist.into();
        assert!(matches!(persist, HashExpiration::Persist));
    }

    #[test]
    fn test_hgetex_expiration_deserialization() {
        let ex: HGetExExpirationSchema = serde_json::from_str(r#"{"ex": 60}"#).unwrap();
        assert!(matches!(ex, HGetExExpirationSchema::Ex(60)));

        let persist: HGetExExpirationSchema = serde_json::from_str(r#""persist""#).unwrap();
        assert!(matches!(persist, HGetExExpirationSchema::Persist));
    }

    #[test]
    fn test_hgetex_response() {
        let resp = HGetExResponse {
            values: vec![Some("v1".to_string()), None],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("v1"));
        assert!(json.contains("null"));
    }

    // --- HSETEX schema tests ---

    #[test]
    fn test_hsetex_request_empty_fields_fails() {
        let req = HSetExRequest {
            fields: HashMap::new(),
            condition: None,
            expiration: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_hsetex_request_valid_passes() {
        let mut fields = HashMap::new();
        fields.insert("f1".to_string(), "v1".to_string());
        let req = HSetExRequest {
            fields,
            condition: None,
            expiration: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_hsetex_request_with_condition_and_expiration() {
        let mut fields = HashMap::new();
        fields.insert("f1".to_string(), "v1".to_string());
        let req = HSetExRequest {
            fields,
            condition: Some(HSetExConditionSchema::Fnx),
            expiration: Some(HSetExExpirationSchema::Ex(60)),
        };
        assert!(req.validate().is_ok());
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("FNX"));
        assert!(json.contains("ex"));
    }

    #[test]
    fn test_hsetex_condition_schema_conversions() {
        use crate::domain::repositories::HSetExCondition;

        let fnx: HSetExCondition = HSetExConditionSchema::Fnx.into();
        assert!(matches!(fnx, HSetExCondition::FNX));

        let fxx: HSetExCondition = HSetExConditionSchema::Fxx.into();
        assert!(matches!(fxx, HSetExCondition::FXX));
    }

    #[test]
    fn test_hsetex_condition_schema_deserialization() {
        let fnx: HSetExConditionSchema = serde_json::from_str(r#""FNX""#).unwrap();
        assert!(matches!(fnx, HSetExConditionSchema::Fnx));
        let fxx: HSetExConditionSchema = serde_json::from_str(r#""FXX""#).unwrap();
        assert!(matches!(fxx, HSetExConditionSchema::Fxx));
    }

    #[test]
    fn test_hsetex_expiration_schema_all_variants() {
        use crate::domain::repositories::HashExpiration;

        let ex: HashExpiration = HSetExExpirationSchema::Ex(60).into();
        assert!(matches!(ex, HashExpiration::Ex(60)));

        let px: HashExpiration = HSetExExpirationSchema::Px(1000).into();
        assert!(matches!(px, HashExpiration::Px(1000)));

        let exat: HashExpiration = HSetExExpirationSchema::Exat(1700000000).into();
        assert!(matches!(exat, HashExpiration::Exat(1700000000)));

        let pxat: HashExpiration = HSetExExpirationSchema::Pxat(1700000000000).into();
        assert!(matches!(pxat, HashExpiration::Pxat(1700000000000)));

        let keepttl: HashExpiration = HSetExExpirationSchema::Keepttl.into();
        assert!(matches!(keepttl, HashExpiration::Keepttl));
    }

    #[test]
    fn test_hsetex_expiration_deserialization() {
        let ex: HSetExExpirationSchema = serde_json::from_str(r#"{"ex": 60}"#).unwrap();
        assert!(matches!(ex, HSetExExpirationSchema::Ex(60)));

        let keepttl: HSetExExpirationSchema = serde_json::from_str(r#""keepttl""#).unwrap();
        assert!(matches!(keepttl, HSetExExpirationSchema::Keepttl));
    }

    #[test]
    fn test_hsetex_request_optional_fields_skipped() {
        let mut fields = HashMap::new();
        fields.insert("f1".to_string(), "v1".to_string());
        let req = HSetExRequest {
            fields,
            condition: None,
            expiration: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("condition"));
        assert!(!json.contains("expiration"));
    }

    #[test]
    fn test_hsetex_response() {
        let resp = HSetExResponse { count: 3 };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("3"));
    }

    // --- HGETDEL schema tests ---

    #[test]
    fn test_hgetdel_request_empty_fields_fails() {
        let req = HGetDelRequest { fields: vec![] };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_hgetdel_request_valid_passes() {
        let req = HGetDelRequest {
            fields: vec!["f1".to_string()],
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_hgetdel_response() {
        let resp = HGetDelResponse {
            values: vec![Some("v1".to_string()), None],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("v1"));
        assert!(json.contains("null"));
    }
}
