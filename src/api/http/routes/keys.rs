//! Key Routes
//!
//! HTTP endpoints for Redis key management operations.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

use validator::Validate;

use crate::api::http::schemas::keys::{
    CopyRequest, CopyResponse, DeleteKeysRequest, DeleteKeysResponse, DumpResponse, ExistsRequest,
    ExistsResponse, ExpireRequest, ExpireResponse, KeyInfoResponse, KeysParams, KeysResponse,
    ObjectInfoResponse, PersistResponse, RandomKeyResponse, RenameRequest, RenameResponse,
    RestoreRequest, RestoreResponse, ScanParams, ScanResponse, SortRequest, SortResponse,
    SortStoreRequest, SortStoreResponse, TouchRequest, TouchResponse, TtlResponse, TypeResponse,
};
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create key management routes
pub fn key_routes() -> Router<AppState> {
    Router::new()
        // Batch operations
        .route("/api/v1/keys/delete", post(delete_keys))
        .route("/api/v1/keys/exists", post(exists_keys))
        .route("/api/v1/keys/touch", post(touch_keys))
        // Scan and search
        .route("/api/v1/keys/scan", get(scan_keys))
        .route("/api/v1/keys", get(list_keys))
        .route("/api/v1/keys/random", get(random_key))
        // Single key operations
        .route("/api/v1/keys/{key}", get(get_key_info))
        .route("/api/v1/keys/{key}", delete(delete_single_key))
        // TTL operations
        .route("/api/v1/keys/{key}/ttl", get(get_ttl))
        .route("/api/v1/keys/{key}/expire", patch(set_expire))
        .route("/api/v1/keys/{key}/persist", patch(persist_key))
        // Type operation
        .route("/api/v1/keys/{key}/type", get(get_type))
        // Rename operations
        .route("/api/v1/keys/{key}/rename", patch(rename_key))
        // Copy operation
        .route("/api/v1/keys/{key}/copy", post(copy_key))
        // Serialization
        .route("/api/v1/keys/{key}/dump", get(dump_key))
        .route("/api/v1/keys/{key}/restore", post(restore_key))
        // Object info
        .route("/api/v1/keys/{key}/object", get(get_object_info))
        // Sort operations
        .route("/api/v1/keys/{key}/sort", post(sort_key))
        .route("/api/v1/keys/{key}/sort/store", post(sort_store_key))
        .route("/api/v1/keys/{key}/sort/readonly", post(sort_ro_key))
}

/// POST /api/v1/keys/delete
///
/// Delete one or more keys.
#[utoipa::path(
    post,
    path = "/api/v1/keys/delete",
    request_body = DeleteKeysRequest,
    responses(
        (status = 200, description = "Keys deleted", body = DeleteKeysResponse)
    ),
    tag = "Keys"
)]
pub async fn delete_keys(
    State(state): State<AppState>,
    Json(request): Json<DeleteKeysRequest>,
) -> Result<Json<ApiResponse<DeleteKeysResponse>>, CacheError> {
    let result = if request.async_delete {
        state.key_service.unlink(request.keys).await?
    } else {
        state.key_service.delete(request.keys).await?
    };

    Ok(Json(ApiResponse::new(DeleteKeysResponse {
        deleted: result.deleted,
        not_found: result.not_found,
        count: result.count,
    })))
}

/// POST /api/v1/keys/exists
///
/// Check if one or more keys exist.
#[utoipa::path(
    post,
    path = "/api/v1/keys/exists",
    request_body = ExistsRequest,
    responses(
        (status = 200, description = "Existence check result", body = ExistsResponse)
    ),
    tag = "Keys"
)]
pub async fn exists_keys(
    State(state): State<AppState>,
    Json(request): Json<ExistsRequest>,
) -> Result<Json<ApiResponse<ExistsResponse>>, CacheError> {
    let result = state.key_service.exists(request.keys).await?;

    Ok(Json(ApiResponse::new(ExistsResponse {
        existing: result.existing,
        missing: result.missing,
        count: result.count,
    })))
}

/// POST /api/v1/keys/touch
///
/// Update the last access time of keys.
#[utoipa::path(
    post,
    path = "/api/v1/keys/touch",
    request_body = TouchRequest,
    responses(
        (status = 200, description = "Keys touched", body = TouchResponse)
    ),
    tag = "Keys"
)]
pub async fn touch_keys(
    State(state): State<AppState>,
    Json(request): Json<TouchRequest>,
) -> Result<Json<ApiResponse<TouchResponse>>, CacheError> {
    let result = state.key_service.touch(request.keys).await?;

    Ok(Json(ApiResponse::new(TouchResponse {
        count: result.count,
    })))
}

/// GET /api/v1/keys/scan
///
/// Incrementally iterate over keys.
#[utoipa::path(
    get,
    path = "/api/v1/keys/scan",
    params(
        ("cursor" = u64, Query, description = "Cursor position (0 to start)"),
        ("pattern" = Option<String>, Query, description = "Pattern to match"),
        ("count" = Option<u64>, Query, description = "Hint for number of keys to return"),
        ("type" = Option<String>, Query, description = "Filter by key type")
    ),
    responses(
        (status = 200, description = "Scan results", body = ScanResponse)
    ),
    tag = "Keys"
)]
pub async fn scan_keys(
    State(state): State<AppState>,
    Query(params): Query<ScanParams>,
) -> Result<Json<ApiResponse<ScanResponse>>, CacheError> {
    let result = state
        .key_service
        .scan(params.cursor, params.pattern, params.count, params.key_type)
        .await?;

    Ok(Json(ApiResponse::new(ScanResponse {
        cursor: result.cursor,
        keys: result.keys,
        count: result.count,
    })))
}

/// GET /api/v1/keys
///
/// Find all keys matching a pattern (use with caution in production).
#[utoipa::path(
    get,
    path = "/api/v1/keys",
    params(
        ("pattern" = String, Query, description = "Pattern to match (e.g., 'user:*')")
    ),
    responses(
        (status = 200, description = "Matching keys", body = KeysResponse)
    ),
    tag = "Keys"
)]
pub async fn list_keys(
    State(state): State<AppState>,
    Query(params): Query<KeysParams>,
) -> Result<Json<ApiResponse<KeysResponse>>, CacheError> {
    let keys = state.key_service.keys(&params.pattern).await?;

    Ok(Json(ApiResponse::new(KeysResponse {
        count: keys.len(),
        keys,
    })))
}

/// GET /api/v1/keys/random
///
/// Get a random key from the database.
#[utoipa::path(
    get,
    path = "/api/v1/keys/random",
    responses(
        (status = 200, description = "Random key", body = RandomKeyResponse)
    ),
    tag = "Keys"
)]
pub async fn random_key(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<RandomKeyResponse>>, CacheError> {
    let result = state.key_service.random_key().await?;

    Ok(Json(ApiResponse::new(RandomKeyResponse {
        key: result.key,
    })))
}

/// GET /api/v1/keys/:key
///
/// Get comprehensive information about a key.
#[utoipa::path(
    get,
    path = "/api/v1/keys/{key}",
    params(
        ("key" = String, Path, description = "The key to get info for")
    ),
    responses(
        (status = 200, description = "Key information", body = KeyInfoResponse)
    ),
    tag = "Keys"
)]
pub async fn get_key_info(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<KeyInfoResponse>>, CacheError> {
    let info = state.key_service.key_info(&key).await?;

    Ok(Json(ApiResponse::new(KeyInfoResponse {
        key: info.key,
        key_type: info.key_type,
        ttl: info.ttl,
        pttl: info.pttl,
        exists: info.exists,
        memory_usage: info.memory_usage,
        encoding: info.encoding,
        idle_time: info.idle_time,
        ref_count: info.ref_count,
    })))
}

/// DELETE /api/v1/keys/:key
///
/// Delete a single key.
#[utoipa::path(
    delete,
    path = "/api/v1/keys/{key}",
    params(
        ("key" = String, Path, description = "The key to delete")
    ),
    responses(
        (status = 200, description = "Key deleted", body = DeleteKeysResponse)
    ),
    tag = "Keys"
)]
pub async fn delete_single_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<DeleteKeysResponse>>, CacheError> {
    let result = state.key_service.delete(vec![key]).await?;

    Ok(Json(ApiResponse::new(DeleteKeysResponse {
        deleted: result.deleted,
        not_found: result.not_found,
        count: result.count,
    })))
}

/// GET /api/v1/keys/:key/ttl
///
/// Get the TTL of a key.
#[utoipa::path(
    get,
    path = "/api/v1/keys/{key}/ttl",
    params(
        ("key" = String, Path, description = "The key to get TTL for")
    ),
    responses(
        (status = 200, description = "TTL information", body = TtlResponse)
    ),
    tag = "Keys"
)]
pub async fn get_ttl(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<TtlResponse>>, CacheError> {
    let ttl = state.key_service.ttl(&key).await?;
    let pttl = state.key_service.pttl(&key).await?;

    Ok(Json(ApiResponse::new(TtlResponse {
        key,
        ttl,
        pttl: Some(pttl),
    })))
}

/// PATCH /api/v1/keys/:key/expire
///
/// Set expiration on a key.
#[utoipa::path(
    patch,
    path = "/api/v1/keys/{key}/expire",
    params(
        ("key" = String, Path, description = "The key to set expiration on")
    ),
    request_body = ExpireRequest,
    responses(
        (status = 200, description = "Expiration set", body = ExpireResponse)
    ),
    tag = "Keys"
)]
pub async fn set_expire(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<ExpireRequest>,
) -> Result<Json<ApiResponse<ExpireResponse>>, CacheError> {
    let ExpireRequest {
        seconds,
        milliseconds,
        expire_at,
        pexpire_at,
        nx,
        xx,
        gt,
        lt,
    } = request;

    let result = match (seconds, milliseconds, expire_at, pexpire_at) {
        (Some(seconds), _, _, _) => {
            state
                .key_service
                .expire(&key, seconds, nx, xx, gt, lt)
                .await?
        }
        (None, Some(milliseconds), _, _) => {
            state
                .key_service
                .pexpire(&key, milliseconds, nx, xx, gt, lt)
                .await?
        }
        (None, None, Some(timestamp), _) => {
            state
                .key_service
                .expire_at(&key, timestamp, nx, xx, gt, lt)
                .await?
        }
        (None, None, None, Some(timestamp)) => {
            state
                .key_service
                .pexpire_at(&key, timestamp, nx, xx, gt, lt)
                .await?
        }
        _ => {
            return Err(CacheError::InvalidInput(
                "One of seconds, milliseconds, expire_at, or pexpire_at is required".to_string(),
            ));
        }
    };

    Ok(Json(ApiResponse::new(ExpireResponse {
        key: result.key,
        success: result.success,
        new_ttl: result.new_ttl,
    })))
}

/// PATCH /api/v1/keys/:key/persist
///
/// Remove expiration from a key.
#[utoipa::path(
    patch,
    path = "/api/v1/keys/{key}/persist",
    params(
        ("key" = String, Path, description = "The key to persist")
    ),
    responses(
        (status = 200, description = "TTL removed", body = PersistResponse)
    ),
    tag = "Keys"
)]
pub async fn persist_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<PersistResponse>>, CacheError> {
    let result = state.key_service.persist(&key).await?;

    Ok(Json(ApiResponse::new(PersistResponse {
        key: result.key,
        success: result.success,
    })))
}

/// GET /api/v1/keys/:key/type
///
/// Get the type of a key.
#[utoipa::path(
    get,
    path = "/api/v1/keys/{key}/type",
    params(
        ("key" = String, Path, description = "The key to get type of")
    ),
    responses(
        (status = 200, description = "Key type", body = TypeResponse)
    ),
    tag = "Keys"
)]
pub async fn get_type(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<TypeResponse>>, CacheError> {
    let key_type = state.key_service.key_type(&key).await?;

    Ok(Json(ApiResponse::new(TypeResponse { key, key_type })))
}

/// PATCH /api/v1/keys/:key/rename
///
/// Rename a key.
#[utoipa::path(
    patch,
    path = "/api/v1/keys/{key}/rename",
    params(
        ("key" = String, Path, description = "The key to rename")
    ),
    request_body = RenameRequest,
    responses(
        (status = 200, description = "Key renamed", body = RenameResponse)
    ),
    tag = "Keys"
)]
pub async fn rename_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<RenameRequest>,
) -> Result<Json<ApiResponse<RenameResponse>>, CacheError> {
    let result = if request.nx {
        state.key_service.rename_nx(&key, &request.new_key).await?
    } else {
        state.key_service.rename(&key, &request.new_key).await?
    };

    Ok(Json(ApiResponse::new(RenameResponse {
        old_key: result.old_key,
        new_key: result.new_key,
        success: result.success,
    })))
}

/// POST /api/v1/keys/:key/copy
///
/// Copy a key to a new key.
#[utoipa::path(
    post,
    path = "/api/v1/keys/{key}/copy",
    params(
        ("key" = String, Path, description = "The source key")
    ),
    request_body = CopyRequest,
    responses(
        (status = 200, description = "Key copied", body = CopyResponse)
    ),
    tag = "Keys"
)]
pub async fn copy_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<CopyRequest>,
) -> Result<Json<ApiResponse<CopyResponse>>, CacheError> {
    let result = state
        .key_service
        .copy(&key, &request.destination, request.db, request.replace)
        .await?;

    Ok(Json(ApiResponse::new(CopyResponse {
        source: result.source,
        destination: result.destination,
        success: result.success,
    })))
}

/// GET /api/v1/keys/:key/dump
///
/// Serialize a key's value.
#[utoipa::path(
    get,
    path = "/api/v1/keys/{key}/dump",
    params(
        ("key" = String, Path, description = "The key to dump")
    ),
    responses(
        (status = 200, description = "Serialized value", body = DumpResponse)
    ),
    tag = "Keys"
)]
pub async fn dump_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<DumpResponse>>, CacheError> {
    let result = state.key_service.dump(&key).await?;

    Ok(Json(ApiResponse::new(DumpResponse {
        key: result.key,
        data: result.data,
    })))
}

/// POST /api/v1/keys/:key/restore
///
/// Restore a serialized value to a key.
#[utoipa::path(
    post,
    path = "/api/v1/keys/{key}/restore",
    params(
        ("key" = String, Path, description = "The key to restore to")
    ),
    request_body = RestoreRequest,
    responses(
        (status = 200, description = "Value restored", body = RestoreResponse),
        (status = 400, description = "Invalid input, invalid base64, negative TTL, or mutually exclusive IDLETIME/FREQ")
    ),
    tag = "Keys"
)]
pub async fn restore_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<RestoreRequest>,
) -> Result<Json<ApiResponse<RestoreResponse>>, CacheError> {
    let data = BASE64
        .decode(&request.data)
        .map_err(|e| CacheError::InvalidInput(format!("Invalid base64 data: {}", e)))?;
    let options = crate::domain::entities::RestoreOptions {
        ttl: request.ttl,
        replace: request.replace,
        absttl: request.absttl,
        idletime: request.idletime,
        freq: request.freq,
    };
    let success = state.key_service.restore(&key, &data, options).await?;

    Ok(Json(ApiResponse::new(RestoreResponse { key, success })))
}

/// GET /api/v1/keys/:key/object
///
/// Get object information about a key.
#[utoipa::path(
    get,
    path = "/api/v1/keys/{key}/object",
    params(
        ("key" = String, Path, description = "The key to get object info for")
    ),
    responses(
        (status = 200, description = "Object information", body = ObjectInfoResponse)
    ),
    tag = "Keys"
)]
pub async fn get_object_info(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<ObjectInfoResponse>>, CacheError> {
    let encoding = state.key_service.object_encoding(&key).await?;
    let ref_count = state.key_service.object_refcount(&key).await?;
    let idle_time = state.key_service.object_idletime(&key).await?;
    let freq = state.key_service.object_freq(&key).await?;

    Ok(Json(ApiResponse::new(ObjectInfoResponse {
        key,
        encoding,
        ref_count,
        idle_time,
        freq,
    })))
}

/// POST /api/v1/keys/:key/sort
///
/// Sort the elements in a list, set or sorted set.
#[utoipa::path(
    post,
    path = "/api/v1/keys/{key}/sort",
    params(
        ("key" = String, Path, description = "The key to sort")
    ),
    request_body = SortRequest,
    responses(
        (status = 200, description = "Sorted elements", body = SortResponse)
    ),
    tag = "Keys"
)]
pub async fn sort_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SortRequest>,
) -> Result<Json<ApiResponse<SortResponse>>, CacheError> {
    let options = request.into_sort_options();
    let values = state.key_service.sort(&key, options).await?;
    Ok(Json(ApiResponse::new(SortResponse { values })))
}

/// POST /api/v1/keys/:key/sort/store
///
/// Sort elements and store the result in a destination key.
#[utoipa::path(
    post,
    path = "/api/v1/keys/{key}/sort/store",
    params(
        ("key" = String, Path, description = "The key to sort")
    ),
    request_body = SortStoreRequest,
    responses(
        (status = 200, description = "Elements sorted and stored", body = SortStoreResponse)
    ),
    tag = "Keys"
)]
pub async fn sort_store_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SortStoreRequest>,
) -> Result<Json<ApiResponse<SortStoreResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let destination = request.destination;
    let options = request.options.into_sort_options();
    let count = state
        .key_service
        .sort_store(&key, &destination, options)
        .await?;
    Ok(Json(ApiResponse::new(SortStoreResponse { count })))
}

/// POST /api/v1/keys/:key/sort/readonly
///
/// Read-only sort (Redis 7.0+).
#[utoipa::path(
    post,
    path = "/api/v1/keys/{key}/sort/readonly",
    params(
        ("key" = String, Path, description = "The key to sort (read-only)")
    ),
    request_body = SortRequest,
    responses(
        (status = 200, description = "Sorted elements (read-only)", body = SortResponse)
    ),
    tag = "Keys"
)]
pub async fn sort_ro_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SortRequest>,
) -> Result<Json<ApiResponse<SortResponse>>, CacheError> {
    let options = request.into_sort_options();
    let values = state.key_service.sort_ro(&key, options).await?;
    Ok(Json(ApiResponse::new(SortResponse { values })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state;
    use axum::Json;
    use axum::extract::{Path, Query, State};

    #[tokio::test]
    async fn test_key_routes_batch_and_query() {
        let (state, _string_repo, key_repo, _admin_repo) = test_state();
        key_repo.insert("alpha", "one");
        key_repo.insert("beta", "two");
        key_repo.insert("gamma", "three");
        let state = State(state);

        let exists = exists_keys(
            state.clone(),
            Json(ExistsRequest {
                keys: vec!["alpha".to_string(), "missing".to_string()],
            }),
        )
        .await
        .unwrap();
        let data = exists.0.data.expect("data");
        assert_eq!(data.count, 1);

        let touch = touch_keys(
            state.clone(),
            Json(TouchRequest {
                keys: vec!["alpha".to_string(), "beta".to_string()],
            }),
        )
        .await
        .unwrap();
        let data = touch.0.data.expect("data");
        assert_eq!(data.count, 2);

        let scan = scan_keys(
            state.clone(),
            Query(ScanParams {
                cursor: 0,
                pattern: Some("a".to_string()),
                count: Some(10),
                key_type: None,
            }),
        )
        .await
        .unwrap();
        let data = scan.0.data.expect("data");
        assert!(data.keys.iter().any(|key| key.contains('a')));

        let list = list_keys(
            state.clone(),
            Query(KeysParams {
                pattern: "*".to_string(),
            }),
        )
        .await
        .unwrap();
        let data = list.0.data.expect("data");
        assert!(data.count >= 3);

        let random = random_key(state.clone()).await.unwrap();
        assert!(random.0.data.expect("data").key.is_some());

        let info = get_key_info(state.clone(), Path("alpha".to_string()))
            .await
            .unwrap();
        assert!(info.0.data.expect("data").exists);

        let delete = delete_keys(
            state.clone(),
            Json(DeleteKeysRequest {
                keys: vec!["alpha".to_string()],
                async_delete: false,
            }),
        )
        .await
        .unwrap();
        assert_eq!(delete.0.data.expect("data").count, 1);

        let delete_async = delete_keys(
            state.clone(),
            Json(DeleteKeysRequest {
                keys: vec!["beta".to_string()],
                async_delete: true,
            }),
        )
        .await
        .unwrap();
        assert_eq!(delete_async.0.data.expect("data").count, 1);

        let delete_single = delete_single_key(state, Path("gamma".to_string()))
            .await
            .unwrap();
        assert_eq!(delete_single.0.data.expect("data").count, 1);
    }

    #[tokio::test]
    async fn test_key_routes_ttl_and_expire() {
        let (state, _string_repo, key_repo, _admin_repo) = test_state();
        key_repo.insert("ttl_key", "value");
        let state = State(state);

        let ttl = get_ttl(state.clone(), Path("ttl_key".to_string()))
            .await
            .unwrap();
        assert_eq!(ttl.0.data.expect("data").ttl, -1);

        let expire = set_expire(
            state.clone(),
            Path("ttl_key".to_string()),
            Json(ExpireRequest {
                seconds: Some(10),
                milliseconds: None,
                expire_at: None,
                pexpire_at: None,
                nx: false,
                xx: false,
                gt: false,
                lt: false,
            }),
        )
        .await
        .unwrap();
        assert!(expire.0.data.expect("data").success);

        let expire = set_expire(
            state.clone(),
            Path("ttl_key".to_string()),
            Json(ExpireRequest {
                seconds: None,
                milliseconds: Some(500),
                expire_at: None,
                pexpire_at: None,
                nx: false,
                xx: false,
                gt: false,
                lt: false,
            }),
        )
        .await
        .unwrap();
        assert!(expire.0.data.expect("data").success);

        let expire = set_expire(
            state.clone(),
            Path("ttl_key".to_string()),
            Json(ExpireRequest {
                seconds: None,
                milliseconds: None,
                expire_at: Some(1000),
                pexpire_at: None,
                nx: false,
                xx: false,
                gt: false,
                lt: false,
            }),
        )
        .await
        .unwrap();
        assert!(expire.0.data.expect("data").success);

        let expire = set_expire(
            state.clone(),
            Path("ttl_key".to_string()),
            Json(ExpireRequest {
                seconds: None,
                milliseconds: None,
                expire_at: None,
                pexpire_at: Some(1000),
                nx: false,
                xx: false,
                gt: false,
                lt: false,
            }),
        )
        .await
        .unwrap();
        assert!(expire.0.data.expect("data").success);

        let err = set_expire(
            state,
            Path("ttl_key".to_string()),
            Json(ExpireRequest {
                seconds: None,
                milliseconds: None,
                expire_at: None,
                pexpire_at: None,
                nx: false,
                xx: false,
                gt: false,
                lt: false,
            }),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_key_routes_mutations_and_object_info() {
        let (state, _string_repo, key_repo, _admin_repo) = test_state();
        key_repo.insert("rename_src", "one");
        key_repo.insert("rename_dest", "two");
        key_repo.insert("copy_src", "three");
        key_repo.insert("dump_key", "dump");
        key_repo.insert("object_key", "obj");
        let state = State(state);

        let persist = persist_key(state.clone(), Path("rename_src".to_string()))
            .await
            .unwrap();
        assert!(persist.0.data.expect("data").success);

        let key_type = get_type(state.clone(), Path("rename_src".to_string()))
            .await
            .unwrap();
        assert_eq!(key_type.0.data.expect("data").key_type, "string");

        let renamed = rename_key(
            state.clone(),
            Path("rename_src".to_string()),
            Json(RenameRequest {
                new_key: "rename_new".to_string(),
                nx: false,
            }),
        )
        .await
        .unwrap();
        assert!(renamed.0.data.expect("data").success);

        let renamed_nx = rename_key(
            state.clone(),
            Path("rename_new".to_string()),
            Json(RenameRequest {
                new_key: "rename_dest".to_string(),
                nx: true,
            }),
        )
        .await
        .unwrap();
        assert!(!renamed_nx.0.data.expect("data").success);

        let copied = copy_key(
            state.clone(),
            Path("copy_src".to_string()),
            Json(CopyRequest {
                destination: "copy_dest".to_string(),
                db: None,
                replace: false,
            }),
        )
        .await
        .unwrap();
        assert!(copied.0.data.expect("data").success);

        let dumped = dump_key(state.clone(), Path("dump_key".to_string()))
            .await
            .unwrap();
        assert!(dumped.0.data.expect("data").data.is_some());

        let restored = restore_key(
            state.clone(),
            Path("restore_key".to_string()),
            Json(RestoreRequest {
                ttl: 0,
                data: BASE64.encode("restored"),
                replace: false,
                absttl: false,
                idletime: None,
                freq: None,
            }),
        )
        .await
        .unwrap();
        assert!(restored.0.data.expect("data").success);

        let err = restore_key(
            state.clone(),
            Path("restore_key".to_string()),
            Json(RestoreRequest {
                ttl: 0,
                data: "not_base64".to_string(),
                replace: false,
                absttl: false,
                idletime: None,
                freq: None,
            }),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let object_info = get_object_info(state, Path("object_key".to_string()))
            .await
            .unwrap();
        let data = object_info.0.data.expect("data");
        assert_eq!(data.encoding.as_deref(), Some("embstr"));
        assert_eq!(data.ref_count, Some(1));
        assert_eq!(data.idle_time, Some(0));
        assert_eq!(data.freq, Some(0));
    }

    #[tokio::test]
    async fn test_sort_key_returns_ok() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state();
        let state = State(state);

        let result = sort_key(
            state,
            Path("mylist".to_string()),
            Json(SortRequest {
                by: None,
                get: vec![],
                offset: None,
                count: None,
                order: crate::api::http::schemas::keys::SortOrderSchema::Asc,
                alpha: false,
            }),
        )
        .await
        .unwrap();
        let data = result.0.data.expect("data");
        assert!(data.values.is_empty());
    }

    #[tokio::test]
    async fn test_sort_store_key_empty_destination_fails() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state();
        let state = State(state);

        let err = sort_store_key(
            state,
            Path("mylist".to_string()),
            Json(SortStoreRequest {
                destination: "".to_string(),
                options: SortRequest {
                    by: None,
                    get: vec![],
                    offset: None,
                    count: None,
                    order: crate::api::http::schemas::keys::SortOrderSchema::Asc,
                    alpha: false,
                },
            }),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_sort_ro_key_returns_ok() {
        let (state, _string_repo, _key_repo, _admin_repo) = test_state();
        let state = State(state);

        let result = sort_ro_key(
            state,
            Path("mylist".to_string()),
            Json(SortRequest {
                by: None,
                get: vec![],
                offset: None,
                count: None,
                order: crate::api::http::schemas::keys::SortOrderSchema::Asc,
                alpha: false,
            }),
        )
        .await
        .unwrap();
        let data = result.0.data.expect("data");
        assert!(data.values.is_empty());
    }
}
