//! Vector Value Entities
//!
//! Core domain entities for Vector Sets operations.

#[derive(Debug, Clone)]
pub struct VectorAddResult {
    pub key: String,
    pub added_count: u64,
}

#[derive(Debug, Clone)]
pub struct VectorRangeResult {
    pub items: Vec<VectorItem>,
}

#[derive(Debug, Clone)]
pub struct VectorSimResult {
    pub items: Vec<VectorItem>,
}

#[derive(Debug, Clone)]
pub struct VectorItem {
    pub id: String,
    pub score: Option<f64>,
    pub vector: Option<Vec<f32>>,
    pub attributes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VectorInfo {
    pub dimension: u64,
    pub distance_metric: String,
    pub data_type: String,
    pub count: u64,
}
