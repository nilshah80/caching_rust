//! Cluster API Routes
//!
//! Endpoints for Redis Cluster info operations.
//! All endpoints are admin-protected.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};

use crate::api::http::middleware::admin_auth::{ADMIN_API_KEY_HEADER, validate_admin_key};
use crate::api::http::schemas::cluster::KeySlotResponse;
use crate::domain::repositories::{ClusterInfo, ClusterNode, ClusterSlotRange};
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create cluster routes (all admin-protected)
pub fn cluster_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/cluster/info", get(cluster_info))
        .route("/api/v1/cluster/nodes", get(cluster_nodes))
        .route("/api/v1/cluster/slots", get(cluster_slots))
        .route("/api/v1/cluster/shards", get(cluster_shards))
        .route("/api/v1/cluster/keyslot/{key}", get(cluster_keyslot))
}

fn verify_admin_key(headers: &HeaderMap, state: &AppState) -> Result<(), StatusCode> {
    let token = headers
        .get(ADMIN_API_KEY_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    validate_admin_key(state, token).map_err(|_| StatusCode::UNAUTHORIZED)
}

/// Get cluster info (CLUSTER INFO)
#[utoipa::path(
    get,
    path = "/api/v1/cluster/info",
    responses(
        (status = 200, description = "Cluster info", body = ClusterInfo),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Cluster"
)]
async fn cluster_info(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ClusterInfo>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    let info = state
        .cluster_service
        .cluster_info()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ApiResponse::success(info)))
}

/// Get cluster nodes (CLUSTER NODES)
#[utoipa::path(
    get,
    path = "/api/v1/cluster/nodes",
    responses(
        (status = 200, description = "Cluster nodes", body = Vec<ClusterNode>),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Cluster"
)]
async fn cluster_nodes(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ClusterNode>>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    let nodes = state
        .cluster_service
        .cluster_nodes()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ApiResponse::success(nodes)))
}

/// Get cluster slot mapping (CLUSTER SLOTS)
#[utoipa::path(
    get,
    path = "/api/v1/cluster/slots",
    responses(
        (status = 200, description = "Cluster slot mapping", body = Vec<ClusterSlotRange>),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Cluster"
)]
async fn cluster_slots(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ClusterSlotRange>>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    let slots = state
        .cluster_service
        .cluster_slots()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ApiResponse::success(slots)))
}

/// Get cluster shards (CLUSTER SHARDS, Redis 7.0+)
#[utoipa::path(
    get,
    path = "/api/v1/cluster/shards",
    responses(
        (status = 200, description = "Cluster shards"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Cluster"
)]
async fn cluster_shards(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    let shards = state
        .cluster_service
        .cluster_shards()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // Convert redis::Value to serde_json::Value for the response
    let json_value = redis_value_to_json(&shards);
    Ok(Json(ApiResponse::success(json_value)))
}

/// Get hash slot for a key (CLUSTER KEYSLOT)
#[utoipa::path(
    get,
    path = "/api/v1/cluster/keyslot/{key}",
    responses(
        (status = 200, description = "Key slot number", body = KeySlotResponse),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Cluster"
)]
async fn cluster_keyslot(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<KeySlotResponse>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    let slot = state
        .cluster_service
        .cluster_keyslot(&key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ApiResponse::success(KeySlotResponse { key, slot })))
}

/// Convert redis::Value to serde_json::Value
fn redis_value_to_json(value: &redis::Value) -> serde_json::Value {
    match value {
        redis::Value::Nil => serde_json::Value::Null,
        redis::Value::Int(n) => serde_json::Value::Number((*n).into()),
        redis::Value::BulkString(b) => {
            serde_json::Value::String(String::from_utf8_lossy(b).to_string())
        }
        redis::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(redis_value_to_json).collect())
        }
        redis::Value::SimpleString(s) => serde_json::Value::String(s.clone()),
        redis::Value::Okay => serde_json::Value::String("OK".to_string()),
        redis::Value::Map(pairs) => {
            let map: serde_json::Map<String, serde_json::Value> = pairs
                .iter()
                .filter_map(|(k, v)| {
                    let key_str = match k {
                        redis::Value::BulkString(b) => Some(String::from_utf8_lossy(b).to_string()),
                        redis::Value::SimpleString(s) => Some(s.clone()),
                        _ => None,
                    };
                    key_str.map(|ks| (ks, redis_value_to_json(v)))
                })
                .collect();
            serde_json::Value::Object(map)
        }
        _ => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_value_to_json_nil() {
        assert_eq!(
            redis_value_to_json(&redis::Value::Nil),
            serde_json::Value::Null
        );
    }

    #[test]
    fn test_redis_value_to_json_int() {
        assert_eq!(
            redis_value_to_json(&redis::Value::Int(42)),
            serde_json::json!(42)
        );
    }

    #[test]
    fn test_redis_value_to_json_string() {
        assert_eq!(
            redis_value_to_json(&redis::Value::BulkString(b"hello".to_vec())),
            serde_json::json!("hello")
        );
    }

    #[test]
    fn test_redis_value_to_json_array() {
        let val = redis::Value::Array(vec![redis::Value::Int(1), redis::Value::Int(2)]);
        assert_eq!(redis_value_to_json(&val), serde_json::json!([1, 2]));
    }

    #[test]
    fn test_redis_value_to_json_ok() {
        assert_eq!(
            redis_value_to_json(&redis::Value::Okay),
            serde_json::json!("OK")
        );
    }

    #[test]
    fn test_redis_value_to_json_simple_string() {
        assert_eq!(
            redis_value_to_json(&redis::Value::SimpleString("PONG".to_string())),
            serde_json::json!("PONG")
        );
    }

    #[test]
    fn test_redis_value_to_json_map() {
        let val = redis::Value::Map(vec![
            (
                redis::Value::BulkString(b"key".to_vec()),
                redis::Value::Int(42),
            ),
            (
                redis::Value::SimpleString("name".to_string()),
                redis::Value::BulkString(b"test".to_vec()),
            ),
        ]);
        let json = redis_value_to_json(&val);
        assert_eq!(json["key"], serde_json::json!(42));
        assert_eq!(json["name"], serde_json::json!("test"));
    }
}
