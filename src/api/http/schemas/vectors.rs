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
