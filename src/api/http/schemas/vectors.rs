//! Vector Sets Schemas
//!
//! Request and response schemas for Vector Sets operations.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domain::entities::{VectorAddResult, VectorSimResult};

// ==================== Vector Sets Schemas ====================

#[derive(Debug, Deserialize, Validate, ToSchema)]
#[validate(schema(function = "validate_vadd_items"))]
pub struct VectorAddRequest {
    /// Items to add, mapping element name to vector embedding
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: std::collections::HashMap<String, Vec<f32>>,
}

fn validate_vadd_items(req: &VectorAddRequest) -> Result<(), validator::ValidationError> {
    for (name, vec) in &req.items {
        if vec.is_empty() {
            let mut err = validator::ValidationError::new("empty_vector");
            err.message = Some(format!("Vector for element '{}' must not be empty", name).into());
            return Err(err);
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorAddResponse {
    pub key: String,
    pub added_count: u64,
}

impl From<VectorAddResult> for VectorAddResponse {
    fn from(result: VectorAddResult) -> Self {
        Self {
            key: result.key,
            added_count: result.added_count,
        }
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VectorRemRequest {
    /// Items to remove
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorRemResponse {
    pub removed_count: u64,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VectorSimRequest {
    /// Vector to compare against
    #[validate(length(min = 1, message = "Vector must not be empty"))]
    pub vector: Vec<f32>,
    /// Number of top items to return (must be >= 1)
    #[validate(range(min = 1, message = "k must be at least 1"))]
    pub k: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorSimItemResponse {
    pub id: String,
    pub score: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorSimResponse {
    pub items: Vec<VectorSimItemResponse>,
}

impl From<VectorSimResult> for VectorSimResponse {
    fn from(result: VectorSimResult) -> Self {
        let items = result
            .items
            .into_iter()
            .map(|item| VectorSimItemResponse {
                id: item.id,
                score: item.score,
            })
            .collect();
        Self { items }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorCardResponse {
    pub count: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorDimResponse {
    pub dimension: u64,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VectorEmbRequest {
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorEmbResponse {
    pub embeddings: Vec<Option<Vec<f32>>>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VectorIsMemberRequest {
    #[validate(length(min = 1, message = "At least one item is required"))]
    pub items: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorIsMemberResponse {
    pub results: Vec<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorLinksLayer {
    /// HNSW layer index (0 = bottom layer)
    pub layer: usize,
    /// Neighbors in this layer
    pub neighbors: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorLinksResponse {
    /// Neighbors grouped by HNSW graph layer
    pub layers: Vec<VectorLinksLayer>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VectorRandMemberRequest {
    /// Number of random members to return (negative for duplicates allowed)
    #[validate(custom(function = "validate_nonzero_i64"))]
    pub count: i64,
}

fn validate_nonzero_i64(value: i64) -> Result<(), validator::ValidationError> {
    if value == 0 {
        return Err(validator::ValidationError::new("count must not be zero"));
    }
    Ok(())
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorRandMemberResponse {
    pub members: Vec<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
#[validate(schema(function = "validate_vrange_count"))]
pub struct VectorRangeRequest {
    /// Start of lexicographic range (e.g. "-", "[a", "(a")
    #[validate(length(min = 1, message = "Start range cannot be empty"))]
    pub start: String,
    /// End of lexicographic range (e.g. "+", "[z", "(z")
    #[validate(length(min = 1, message = "End range cannot be empty"))]
    pub end: String,
    /// Maximum number of results (-1 for all elements, positive for a limit)
    pub count: Option<i64>,
}

fn validate_vrange_count(req: &VectorRangeRequest) -> Result<(), validator::ValidationError> {
    if let Some(c) = req.count
        && c != -1
        && c < 1
    {
        let mut err = validator::ValidationError::new("invalid_count");
        err.message = Some("count must be -1 (all) or a positive integer".into());
        return Err(err);
    }
    Ok(())
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorRangeItemResponse {
    pub id: String,
    pub score: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorRangeResponse {
    pub items: Vec<VectorRangeItemResponse>,
}

impl From<crate::domain::entities::VectorRangeResult> for VectorRangeResponse {
    fn from(result: crate::domain::entities::VectorRangeResult) -> Self {
        let items = result
            .items
            .into_iter()
            .map(|item| VectorRangeItemResponse {
                id: item.id,
                score: item.score,
            })
            .collect();
        Self { items }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorInfoResponse {
    pub dimension: u64,
    pub distance_metric: String,
    pub data_type: String,
    pub count: u64,
}

impl From<crate::domain::entities::VectorInfo> for VectorInfoResponse {
    fn from(info: crate::domain::entities::VectorInfo) -> Self {
        Self {
            dimension: info.dimension,
            distance_metric: info.distance_metric,
            data_type: info.data_type,
            count: info.count,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorGetAttrResponse {
    pub attributes: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VectorSetAttrRequest {
    /// JSON attributes to set on the element. Use an empty string "" to delete attributes.
    pub attributes: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VectorSetAttrResponse {
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{VectorInfo, VectorItem, VectorRangeResult};
    use std::collections::HashMap;
    use validator::Validate;

    #[test]
    fn test_vector_add_request_validates_non_empty_vectors() {
        let request = VectorAddRequest {
            items: HashMap::from([("item-1".to_string(), vec![1.0, 2.0])]),
        };
        assert!(request.validate().is_ok());

        let request = VectorAddRequest {
            items: HashMap::from([("item-1".to_string(), Vec::new())]),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_vector_rand_member_rejects_zero_count() {
        assert!(VectorRandMemberRequest { count: 1 }.validate().is_ok());
        assert!(VectorRandMemberRequest { count: -1 }.validate().is_ok());
        assert!(VectorRandMemberRequest { count: 0 }.validate().is_err());
    }

    #[test]
    fn test_vector_range_count_validation() {
        assert!(
            VectorRangeRequest {
                start: "-".to_string(),
                end: "+".to_string(),
                count: None,
            }
            .validate()
            .is_ok()
        );
        assert!(
            VectorRangeRequest {
                start: "-".to_string(),
                end: "+".to_string(),
                count: Some(-1),
            }
            .validate()
            .is_ok()
        );
        assert!(
            VectorRangeRequest {
                start: "-".to_string(),
                end: "+".to_string(),
                count: Some(3),
            }
            .validate()
            .is_ok()
        );
        assert!(
            VectorRangeRequest {
                start: "-".to_string(),
                end: "+".to_string(),
                count: Some(0),
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn test_vector_response_conversions() {
        let add_response = VectorAddResponse::from(VectorAddResult {
            key: "vectors".to_string(),
            added_count: 2,
        });
        assert_eq!(add_response.key, "vectors");
        assert_eq!(add_response.added_count, 2);

        let sim_response = VectorSimResponse::from(VectorSimResult {
            items: vec![VectorItem {
                id: "item-1".to_string(),
                score: Some(0.75),
                vector: Some(vec![1.0, 2.0]),
                attributes: Some("{}".to_string()),
            }],
        });
        assert_eq!(sim_response.items[0].id, "item-1");
        assert_eq!(sim_response.items[0].score, Some(0.75));

        let range_response = VectorRangeResponse::from(VectorRangeResult {
            items: vec![VectorItem {
                id: "item-2".to_string(),
                score: None,
                vector: None,
                attributes: None,
            }],
        });
        assert_eq!(range_response.items[0].id, "item-2");
        assert_eq!(range_response.items[0].score, None);

        let info_response = VectorInfoResponse::from(VectorInfo {
            dimension: 128,
            distance_metric: "L2".to_string(),
            data_type: "FLOAT32".to_string(),
            count: 10,
        });
        assert_eq!(info_response.dimension, 128);
        assert_eq!(info_response.distance_metric, "L2");
        assert_eq!(info_response.data_type, "FLOAT32");
        assert_eq!(info_response.count, 10);
    }
}
