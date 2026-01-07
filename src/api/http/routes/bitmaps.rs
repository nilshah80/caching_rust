//! Bitmap Routes
//!
//! HTTP endpoints for Redis bitmap operations.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};

use crate::api::http::schemas::bitmaps::{
    BitCountQuery, BitCountResponse, BitGetResponse, BitOpRequest, BitOpResponse, BitPosQuery,
    BitPosResponse, BitSetRequest, BitSetResponse, BitfieldRequest, BitfieldResponse,
};
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create Bitmap routes
pub fn bitmap_routes() -> Router<AppState> {
    Router::new()
        // Basic bit operations
        .route("/api/v1/bitmaps/{key}/bit/{offset}", get(getbit))
        .route("/api/v1/bitmaps/{key}/bit/{offset}", put(setbit))
        // Counting operations
        .route("/api/v1/bitmaps/{key}/count", get(bitcount))
        .route("/api/v1/bitmaps/{key}/pos", get(bitpos))
        // Bitwise operations
        .route("/api/v1/bitmaps/operations", post(bitop))
        // BITFIELD operations
        .route("/api/v1/bitmaps/{key}/bitfield", post(bitfield))
        .route("/api/v1/bitmaps/{key}/bitfield/ro", post(bitfield_ro))
}

/// GETBIT - Get the bit value at a specific offset
#[utoipa::path(
    get,
    path = "/api/v1/bitmaps/{key}/bit/{offset}",
    tag = "Bitmaps",
    params(
        ("key" = String, Path, description = "The bitmap key"),
        ("offset" = u64, Path, description = "The bit offset")
    ),
    responses(
        (status = 200, description = "Bit value retrieved", body = BitGetResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn getbit(
    State(state): State<AppState>,
    Path((key, offset)): Path<(String, u64)>,
) -> Result<Json<ApiResponse<BitGetResponse>>, CacheError> {
    let value = state.bitmap_service.getbit(&key, offset).await?;
    Ok(Json(ApiResponse::success(BitGetResponse { value })))
}

/// SETBIT - Set the bit value at a specific offset
#[utoipa::path(
    put,
    path = "/api/v1/bitmaps/{key}/bit/{offset}",
    tag = "Bitmaps",
    params(
        ("key" = String, Path, description = "The bitmap key"),
        ("offset" = u64, Path, description = "The bit offset")
    ),
    request_body = BitSetRequest,
    responses(
        (status = 200, description = "Bit set successfully", body = BitSetResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn setbit(
    State(state): State<AppState>,
    Path((key, offset)): Path<(String, u64)>,
    Json(request): Json<BitSetRequest>,
) -> Result<Json<ApiResponse<BitSetResponse>>, CacheError> {
    let original_value = state
        .bitmap_service
        .setbit(&key, offset, request.value)
        .await?;
    Ok(Json(ApiResponse::success(BitSetResponse { original_value })))
}

/// BITCOUNT - Count the number of set bits in a string
#[utoipa::path(
    get,
    path = "/api/v1/bitmaps/{key}/count",
    tag = "Bitmaps",
    params(
        ("key" = String, Path, description = "The bitmap key"),
        ("start" = Option<i64>, Query, description = "Start position (byte or bit index)"),
        ("end" = Option<i64>, Query, description = "End position (byte or bit index)"),
        ("use_bit" = Option<bool>, Query, description = "If true, start/end are bit positions; if false (default), byte positions")
    ),
    responses(
        (status = 200, description = "Bit count retrieved", body = BitCountResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn bitcount(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<BitCountQuery>,
) -> Result<Json<ApiResponse<BitCountResponse>>, CacheError> {
    let count = state
        .bitmap_service
        .bitcount(&key, query.start, query.end, query.use_bit)
        .await?;
    Ok(Json(ApiResponse::success(BitCountResponse { count })))
}

/// BITPOS - Find the position of the first bit set to 0 or 1
#[utoipa::path(
    get,
    path = "/api/v1/bitmaps/{key}/pos",
    tag = "Bitmaps",
    params(
        ("key" = String, Path, description = "The bitmap key"),
        ("bit" = bool, Query, description = "The bit value to search for (true=1, false=0)"),
        ("start" = Option<i64>, Query, description = "Start position (byte or bit index)"),
        ("end" = Option<i64>, Query, description = "End position (byte or bit index)"),
        ("use_bit" = Option<bool>, Query, description = "If true, start/end are bit positions; if false (default), byte positions")
    ),
    responses(
        (status = 200, description = "Bit position found", body = BitPosResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn bitpos(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<BitPosQuery>,
) -> Result<Json<ApiResponse<BitPosResponse>>, CacheError> {
    let position = state
        .bitmap_service
        .bitpos(&key, query.bit, query.start, query.end, query.use_bit)
        .await?;
    Ok(Json(ApiResponse::success(BitPosResponse { position })))
}

/// BITOP - Perform bitwise operations between strings
#[utoipa::path(
    post,
    path = "/api/v1/bitmaps/operations",
    tag = "Bitmaps",
    request_body = BitOpRequest,
    responses(
        (status = 200, description = "Bitwise operation completed", body = BitOpResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn bitop(
    State(state): State<AppState>,
    Json(request): Json<BitOpRequest>,
) -> Result<Json<ApiResponse<BitOpResponse>>, CacheError> {
    let size = state
        .bitmap_service
        .bitop(request.operation.into(), &request.dest_key, request.keys)
        .await?;
    Ok(Json(ApiResponse::success(BitOpResponse { size })))
}

/// BITFIELD - Perform arbitrary bitfield operations on a string
#[utoipa::path(
    post,
    path = "/api/v1/bitmaps/{key}/bitfield",
    tag = "Bitmaps",
    params(
        ("key" = String, Path, description = "The bitmap key")
    ),
    request_body = BitfieldRequest,
    responses(
        (status = 200, description = "Bitfield operations completed", body = BitfieldResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn bitfield(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<BitfieldRequest>,
) -> Result<Json<ApiResponse<BitfieldResponse>>, CacheError> {
    let commands: Vec<_> = request.commands.into_iter().map(|c| c.into()).collect();
    let result = state.bitmap_service.bitfield(&key, commands).await?;
    Ok(Json(ApiResponse::success(result.into())))
}

/// BITFIELD_RO - Read-only variant of BITFIELD (only GET operations)
#[utoipa::path(
    post,
    path = "/api/v1/bitmaps/{key}/bitfield/ro",
    tag = "Bitmaps",
    params(
        ("key" = String, Path, description = "The bitmap key")
    ),
    request_body = BitfieldRequest,
    responses(
        (status = 200, description = "Bitfield read operations completed", body = BitfieldResponse),
        (status = 400, description = "Invalid request (only GET operations allowed)")
    )
)]
pub async fn bitfield_ro(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<BitfieldRequest>,
) -> Result<Json<ApiResponse<BitfieldResponse>>, CacheError> {
    let commands: Vec<_> = request.commands.into_iter().map(|c| c.into()).collect();
    let result = state.bitmap_service.bitfield_ro(&key, commands).await?;
    Ok(Json(ApiResponse::success(result.into())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state_with_bitmap_repo;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[test]
    fn test_bitmap_routes_structure() {
        let _routes = bitmap_routes();
    }

    #[tokio::test]
    async fn test_getbit() {
        let (state, _) = test_state_with_bitmap_repo();
        let app = bitmap_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/bitmaps/mybitmap/bit/7")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_setbit() {
        let (state, _) = test_state_with_bitmap_repo();
        let app = bitmap_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/bitmaps/mybitmap/bit/7")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"value": true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bitcount() {
        let (state, _) = test_state_with_bitmap_repo();
        let app = bitmap_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/bitmaps/mybitmap/count")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bitcount_with_range() {
        let (state, _) = test_state_with_bitmap_repo();
        let app = bitmap_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/bitmaps/mybitmap/count?start=0&end=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bitpos() {
        let (state, _) = test_state_with_bitmap_repo();
        let app = bitmap_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/bitmaps/mybitmap/pos?bit=true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bitop() {
        let (state, _) = test_state_with_bitmap_repo();
        let app = bitmap_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bitmaps/operations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"operation": "AND", "dest_key": "result", "keys": ["key1", "key2"]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bitfield() {
        let (state, _) = test_state_with_bitmap_repo();
        let app = bitmap_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bitmaps/mybitmap/bitfield")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"commands": [{"command": "GET", "encoding": {"type": "unsigned", "bits": 8}, "offset": 0}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bitfield_ro() {
        let (state, _) = test_state_with_bitmap_repo();
        let app = bitmap_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bitmaps/mybitmap/bitfield/ro")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"commands": [{"command": "GET", "encoding": {"type": "unsigned", "bits": 8}, "offset": 0}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
