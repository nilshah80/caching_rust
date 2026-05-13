//! Cluster API Routes
//!
//! Endpoints for Redis Cluster info operations.
//! All endpoints are admin-protected.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};

use crate::api::http::middleware::admin_auth::{ADMIN_API_KEY_HEADER, validate_admin_key};
use crate::api::http::schemas::cluster::{
    ClusterCountKeysInSlotResponse, ClusterGetKeysInSlotQuery, ClusterGetKeysInSlotResponse,
    ClusterIdResponse, ClusterLinksResponse, ClusterReplicasResponse, ClusterShardIdResponse,
    KeySlotResponse, SlotStatsQuery, SlotStatsResponse, SlotStatsSchema,
};
use crate::domain::errors::CacheError;
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
        .route("/api/v1/cluster/myid", get(cluster_myid))
        .route("/api/v1/cluster/myshardid", get(cluster_myshardid))
        .route("/api/v1/cluster/links", get(cluster_links))
        .route("/api/v1/cluster/replicas/{node_id}", get(cluster_replicas))
        .route("/api/v1/cluster/keyslot/{key}", get(cluster_keyslot))
        .route(
            "/api/v1/cluster/countkeysinslot/{slot}",
            get(cluster_countkeysinslot),
        )
        .route(
            "/api/v1/cluster/getkeysinslot/{slot}",
            get(cluster_getkeysinslot),
        )
        .route("/api/v1/cluster/slot-stats", get(cluster_slot_stats))
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

/// Get this node's cluster ID (CLUSTER MYID)
#[utoipa::path(
    get,
    path = "/api/v1/cluster/myid",
    responses(
        (status = 200, description = "Current cluster node ID", body = ClusterIdResponse),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Cluster"
)]
pub async fn cluster_myid(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ClusterIdResponse>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    let id = state
        .cluster_service
        .cluster_myid()
        .await
        .map_err(|err| cache_error_to_status(&err))?;
    Ok(Json(ApiResponse::success(ClusterIdResponse { id })))
}

/// Get this node's shard ID (CLUSTER MYSHARDID)
#[utoipa::path(
    get,
    path = "/api/v1/cluster/myshardid",
    responses(
        (status = 200, description = "Current cluster shard ID", body = ClusterShardIdResponse),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Cluster"
)]
pub async fn cluster_myshardid(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ClusterShardIdResponse>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    let shard_id = state
        .cluster_service
        .cluster_myshardid()
        .await
        .map_err(|err| cache_error_to_status(&err))?;
    Ok(Json(ApiResponse::success(ClusterShardIdResponse {
        shard_id,
    })))
}

/// Get cluster bus links (CLUSTER LINKS)
#[utoipa::path(
    get,
    path = "/api/v1/cluster/links",
    responses(
        (status = 200, description = "Cluster bus links", body = ClusterLinksResponse),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Cluster"
)]
pub async fn cluster_links(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ClusterLinksResponse>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    let links = state
        .cluster_service
        .cluster_links()
        .await
        .map_err(|err| cache_error_to_status(&err))?;
    Ok(Json(ApiResponse::success(ClusterLinksResponse {
        links: redis_value_to_json(&links),
    })))
}

/// Get replicas for a master node (CLUSTER REPLICAS)
#[utoipa::path(
    get,
    path = "/api/v1/cluster/replicas/{node_id}",
    responses(
        (status = 200, description = "Replica nodes for a master node", body = ClusterReplicasResponse),
        (status = 400, description = "Invalid node id"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Cluster"
)]
pub async fn cluster_replicas(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(node_id): Path<String>,
) -> Result<Json<ApiResponse<ClusterReplicasResponse>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    let replicas = state
        .cluster_service
        .cluster_replicas(&node_id)
        .await
        .map_err(|err| cache_error_to_status(&err))?;
    Ok(Json(ApiResponse::success(ClusterReplicasResponse {
        replicas,
    })))
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

/// Count keys in a hash slot (CLUSTER COUNTKEYSINSLOT)
#[utoipa::path(
    get,
    path = "/api/v1/cluster/countkeysinslot/{slot}",
    responses(
        (status = 200, description = "Number of keys in the hash slot", body = ClusterCountKeysInSlotResponse),
        (status = 400, description = "Invalid slot"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Cluster"
)]
pub async fn cluster_countkeysinslot(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(slot): Path<u16>,
) -> Result<Json<ApiResponse<ClusterCountKeysInSlotResponse>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    let count = state
        .cluster_service
        .cluster_countkeysinslot(slot)
        .await
        .map_err(|err| cache_error_to_status(&err))?;
    Ok(Json(ApiResponse::success(ClusterCountKeysInSlotResponse {
        slot,
        count,
    })))
}

/// Get key names from a hash slot (CLUSTER GETKEYSINSLOT)
#[utoipa::path(
    get,
    path = "/api/v1/cluster/getkeysinslot/{slot}",
    params(
        ("count" = u64, Query, description = "Maximum number of key names to return")
    ),
    responses(
        (status = 200, description = "Key names in the hash slot", body = ClusterGetKeysInSlotResponse),
        (status = 400, description = "Invalid slot or count"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Cluster"
)]
pub async fn cluster_getkeysinslot(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(slot): Path<u16>,
    Query(query): Query<ClusterGetKeysInSlotQuery>,
) -> Result<Json<ApiResponse<ClusterGetKeysInSlotResponse>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    let keys = state
        .cluster_service
        .cluster_getkeysinslot(slot, query.count)
        .await
        .map_err(|err| cache_error_to_status(&err))?;
    Ok(Json(ApiResponse::success(ClusterGetKeysInSlotResponse {
        slot,
        count: query.count,
        keys,
    })))
}

/// GET /api/v1/cluster/slot-stats
///
/// Per-slot usage statistics for slots assigned to the connected node
/// (CLUSTER SLOT-STATS, Redis 8.2+). Either `slot_start`+`slot_end` or
/// `order_by` must be supplied — empty queries are rejected with HTTP 400.
#[utoipa::path(
    get,
    path = "/api/v1/cluster/slot-stats",
    params(
        ("slot_start" = Option<u16>, Query, description = "Inclusive slot range start (paired with slot_end)"),
        ("slot_end" = Option<u16>, Query, description = "Inclusive slot range end (paired with slot_start)"),
        ("order_by" = Option<String>, Query,
            description = "key_count | cpu_usec | memory_bytes | network_bytes_in | network_bytes_out"),
        ("limit" = Option<i64>, Query, description = "Row cap when paired with order_by"),
        ("order" = Option<String>, Query, description = "asc | desc (default asc)"),
    ),
    responses(
        (status = 200, description = "Slot stats for the connected node", body = SlotStatsResponse),
        (status = 400, description = "Invalid filter combination"),
        (status = 401, description = "Unauthorized"),
        (status = 501, description = "CLUSTER SLOT-STATS requires Redis 8.2+")
    ),
    tag = "Cluster"
)]
async fn cluster_slot_stats(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<SlotStatsQuery>,
) -> Result<Json<ApiResponse<SlotStatsResponse>>, StatusCode> {
    verify_admin_key(&headers, &state)?;
    if !state.capabilities.features.cluster_slot_stats {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }
    let filter = query
        .into_filter()
        .map_err(|err| cache_error_to_status(&err))?;
    let stats = state
        .cluster_service
        .cluster_slot_stats(filter)
        .await
        .map_err(|err| cache_error_to_status(&err))?;
    let body = SlotStatsResponse {
        slots: stats.into_iter().map(SlotStatsSchema::from).collect(),
    };
    Ok(Json(ApiResponse::success(body)))
}

/// Map a `CacheError` to the same set of HTTP statuses the admin module uses.
fn cache_error_to_status(err: &CacheError) -> StatusCode {
    match err {
        CacheError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        CacheError::Unauthorized => StatusCode::UNAUTHORIZED,
        CacheError::ModuleNotAvailable(_) => StatusCode::NOT_IMPLEMENTED,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
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
    use crate::application::services::ClusterService;
    use crate::domain::repositories::{ClusterEndpoint, ClusterSlotStatsFilter, SlotStats};
    use async_trait::async_trait;
    use std::sync::Arc;

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

    #[test]
    fn test_cache_error_to_status() {
        assert_eq!(
            cache_error_to_status(&CacheError::InvalidInput("x".into())),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            cache_error_to_status(&CacheError::Unauthorized),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            cache_error_to_status(&CacheError::ModuleNotAvailable("x".into())),
            StatusCode::NOT_IMPLEMENTED
        );
        assert_eq!(
            cache_error_to_status(&CacheError::Timeout),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn test_cluster_introspection_handlers_happy_path() {
        let state = test_state_with_happy_cluster();
        let headers = admin_headers(&state.config.admin.api_key);
        let state = State(state);

        let myid = cluster_myid(headers.clone(), state.clone())
            .await
            .expect("myid")
            .0
            .data
            .expect("body");
        assert_eq!(myid.id, "node-1");

        let shard = cluster_myshardid(headers.clone(), state.clone())
            .await
            .expect("myshardid")
            .0
            .data
            .expect("body");
        assert_eq!(shard.shard_id, "shard-1");

        let links = cluster_links(headers.clone(), state.clone())
            .await
            .expect("links")
            .0
            .data
            .expect("body");
        assert!(links.links.is_array());

        let replicas = cluster_replicas(headers.clone(), state.clone(), Path("node-1".to_string()))
            .await
            .expect("replicas")
            .0
            .data
            .expect("body");
        assert_eq!(replicas.replicas.len(), 1);
        assert_eq!(replicas.replicas[0].master_id.as_deref(), Some("node-1"));

        let count = cluster_countkeysinslot(headers.clone(), state.clone(), Path(42))
            .await
            .expect("countkeysinslot")
            .0
            .data
            .expect("body");
        assert_eq!(count.count, 2);

        let keys = cluster_getkeysinslot(
            headers,
            state,
            Path(42),
            Query(ClusterGetKeysInSlotQuery { count: 2 }),
        )
        .await
        .expect("getkeysinslot")
        .0
        .data
        .expect("body");
        assert_eq!(keys.keys, vec!["key:0".to_string(), "key:1".to_string()]);
    }

    #[tokio::test]
    async fn test_cluster_introspection_handlers_validate_inputs() {
        let state = test_state_with_happy_cluster();
        let headers = admin_headers(&state.config.admin.api_key);
        let state = State(state);

        let result = cluster_replicas(headers.clone(), state.clone(), Path(" ".to_string())).await;
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));

        let result = cluster_countkeysinslot(headers.clone(), state.clone(), Path(16_384)).await;
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));

        let result = cluster_getkeysinslot(
            headers,
            state,
            Path(42),
            Query(ClusterGetKeysInSlotQuery { count: 0 }),
        )
        .await;
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    fn admin_headers(api_key: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            ADMIN_API_KEY_HEADER,
            axum::http::HeaderValue::from_str(api_key).expect("header"),
        );
        headers
    }

    struct HappyClusterRepo;

    #[async_trait]
    impl crate::domain::repositories::ClusterRepository for HappyClusterRepo {
        async fn cluster_info(&self) -> Result<ClusterInfo, CacheError> {
            Ok(ClusterInfo {
                cluster_state: "ok".to_string(),
                cluster_slots_assigned: 16_384,
                cluster_slots_ok: 16_384,
                cluster_slots_pfail: 0,
                cluster_slots_fail: 0,
                cluster_known_nodes: 3,
                cluster_size: 1,
                cluster_current_epoch: 1,
                cluster_my_epoch: 1,
            })
        }

        async fn cluster_nodes(&self) -> Result<Vec<ClusterNode>, CacheError> {
            Ok(vec![cluster_node("node-1", "master", None)])
        }

        async fn cluster_slots(&self) -> Result<Vec<ClusterSlotRange>, CacheError> {
            Ok(vec![ClusterSlotRange {
                start: 0,
                end: 16_383,
                master: ClusterEndpoint {
                    host: "127.0.0.1".to_string(),
                    port: 7000,
                    node_id: Some("node-1".to_string()),
                },
                replicas: vec![],
            }])
        }

        async fn cluster_shards(&self) -> Result<redis::Value, CacheError> {
            Ok(redis::Value::Array(vec![]))
        }

        async fn cluster_myid(&self) -> Result<String, CacheError> {
            Ok("node-1".to_string())
        }

        async fn cluster_myshardid(&self) -> Result<String, CacheError> {
            Ok("shard-1".to_string())
        }

        async fn cluster_links(&self) -> Result<redis::Value, CacheError> {
            Ok(redis::Value::Array(vec![redis::Value::Map(vec![
                (
                    redis::Value::BulkString(b"node".to_vec()),
                    redis::Value::BulkString(b"node-2".to_vec()),
                ),
                (
                    redis::Value::BulkString(b"direction".to_vec()),
                    redis::Value::BulkString(b"to".to_vec()),
                ),
            ])]))
        }

        async fn cluster_replicas(&self, _node_id: &str) -> Result<Vec<ClusterNode>, CacheError> {
            Ok(vec![cluster_node(
                "replica-1",
                "slave",
                Some("node-1".to_string()),
            )])
        }

        async fn cluster_keyslot(&self, _key: &str) -> Result<u16, CacheError> {
            Ok(42)
        }

        async fn cluster_countkeysinslot(&self, _slot: u16) -> Result<u64, CacheError> {
            Ok(2)
        }

        async fn cluster_getkeysinslot(
            &self,
            _slot: u16,
            count: u64,
        ) -> Result<Vec<String>, CacheError> {
            Ok((0..count).map(|i| format!("key:{i}")).collect())
        }

        async fn cluster_slot_stats(
            &self,
            _filter: ClusterSlotStatsFilter,
        ) -> Result<Vec<SlotStats>, CacheError> {
            Ok(vec![SlotStats {
                slot: 0,
                key_count: 1,
                cpu_usec: 2,
                memory_bytes: 3,
                network_bytes_in: 4,
                network_bytes_out: 5,
            }])
        }
    }

    fn cluster_node(id: &str, flags: &str, master_id: Option<String>) -> ClusterNode {
        ClusterNode {
            id: id.to_string(),
            address: "127.0.0.1:7000@17000".to_string(),
            flags: flags.to_string(),
            master_id,
            ping_sent: 0,
            pong_recv: 1000,
            config_epoch: 1,
            link_state: "connected".to_string(),
            slots: vec![],
        }
    }

    fn test_state_with_happy_cluster() -> AppState {
        let (mut state, _, _, _) = crate::test_support::test_state();
        state.cluster_service = Arc::new(ClusterService::new(Arc::new(HappyClusterRepo)));
        let mut caps = (*state.capabilities).clone();
        caps.features.cluster_slot_stats = true;
        state.capabilities = Arc::new(caps);
        state
    }

    #[tokio::test]
    async fn test_cluster_handlers_map_noncluster_mock_to_500() {
        let (state, _, _, _) = crate::test_support::test_state();
        let headers = admin_headers(&state.config.admin.api_key);
        let state = State(state);

        assert!(matches!(
            cluster_info(headers.clone(), state.clone()).await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(matches!(
            cluster_nodes(headers.clone(), state.clone()).await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(matches!(
            cluster_slots(headers.clone(), state.clone()).await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(matches!(
            cluster_shards(headers.clone(), state.clone()).await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(matches!(
            cluster_myid(headers.clone(), state.clone()).await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(matches!(
            cluster_myshardid(headers.clone(), state.clone()).await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(matches!(
            cluster_links(headers.clone(), state.clone()).await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(matches!(
            cluster_replicas(headers.clone(), state.clone(), Path("node-1".to_string())).await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(matches!(
            cluster_keyslot(headers.clone(), state.clone(), Path("key".to_string())).await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(matches!(
            cluster_countkeysinslot(headers.clone(), state.clone(), Path(42)).await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(matches!(
            cluster_getkeysinslot(
                headers,
                state,
                Path(42),
                Query(ClusterGetKeysInSlotQuery { count: 1 })
            )
            .await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
    }

    #[tokio::test]
    async fn test_slot_stats_returns_501_when_capability_off() {
        // The default test_state has cluster_slot_stats disabled.
        let (state, _, _, _) = crate::test_support::test_state();
        let headers = admin_headers(&state.config.admin.api_key);
        let result = cluster_slot_stats(
            headers,
            State(state),
            Query(SlotStatsQuery {
                slot_start: Some(0),
                slot_end: Some(10),
                ..Default::default()
            }),
        )
        .await;
        assert!(matches!(result, Err(StatusCode::NOT_IMPLEMENTED)));
    }

    #[tokio::test]
    async fn test_slot_stats_requires_admin_auth() {
        let (state, _, _, _) = crate::test_support::test_state();
        let result = cluster_slot_stats(
            HeaderMap::new(),
            State(state),
            Query(SlotStatsQuery {
                slot_start: Some(0),
                slot_end: Some(10),
                ..Default::default()
            }),
        )
        .await;
        assert!(matches!(result, Err(StatusCode::UNAUTHORIZED)));
    }

    #[tokio::test]
    async fn test_slot_stats_rejects_empty_filter_with_400() {
        let (mut state, _, _, _) = crate::test_support::test_state();
        // Enable cluster_slot_stats so the auth+capability check passes and we
        // exercise the filter validation specifically.
        let mut caps = (*state.capabilities).clone();
        caps.features.cluster_slot_stats = true;
        state.capabilities = std::sync::Arc::new(caps);
        let headers = admin_headers(&state.config.admin.api_key);
        let result =
            cluster_slot_stats(headers, State(state), Query(SlotStatsQuery::default())).await;
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }
}
