//! Vector Sets API Routes
//!
//! HTTP endpoints for Vector Sets operations.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use validator::Validate;

use crate::api::http::schemas::{
    VectorAddRequest, VectorAddResponse, VectorCardResponse, VectorDimResponse, VectorEmbRequest,
    VectorEmbResponse, VectorGetAttrResponse, VectorInfoResponse, VectorIsMemberRequest,
    VectorIsMemberResponse, VectorLinksResponse, VectorRandMemberRequest, VectorRandMemberResponse,
    VectorRangeRequest, VectorRangeResponse, VectorRemRequest, VectorRemResponse,
    VectorSetAttrRequest, VectorSetAttrResponse, VectorSimRequest, VectorSimResponse,
};
use crate::domain::errors::{CacheError, ErrorResponse};
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create the core vector router (excludes VRANGE which needs separate gating)
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/vectors/{key}/add", post(vadd))
        .route("/api/v1/vectors/{key}/rem", post(vrem))
        .route("/api/v1/vectors/{key}/sim", post(vsim))
        .route("/api/v1/vectors/{key}/card", get(vcard))
        .route("/api/v1/vectors/{key}/dim", get(vdim))
        .route("/api/v1/vectors/{key}/emb", post(vemb))
        .route("/api/v1/vectors/{key}/ismember", post(vismember))
        .route("/api/v1/vectors/{key}/links/{item}", get(vlinks))
        .route("/api/v1/vectors/{key}/randmember", post(vrandmember))
        .route("/api/v1/vectors/{key}/info", get(vinfo))
        .route("/api/v1/vectors/{key}/attr/{item}", get(vgetattr))
        .route("/api/v1/vectors/{key}/attr/{item}", post(vsetattr))
}

/// Create the VRANGE router (gated separately — absent on some early 8.x builds)
pub fn vector_range_router() -> Router<AppState> {
    Router::new().route("/api/v1/vectors/{key}/range", post(vrange))
}

#[utoipa::path(
    post,
    path = "/api/v1/vectors/{key}/add",
    request_body = VectorAddRequest,
    responses(
        (status = 200, description = "Vectors added successfully", body = VectorAddResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key")
    ),
    tag = "Vectors"
)]
pub async fn vadd(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<VectorAddRequest>,
) -> Result<Json<ApiResponse<VectorAddResponse>>, CacheError> {
    payload
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let result = state.vector_service.vadd(&key, payload).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    post,
    path = "/api/v1/vectors/{key}/rem",
    request_body = VectorRemRequest,
    responses(
        (status = 200, description = "Vectors removed successfully", body = VectorRemResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key")
    ),
    tag = "Vectors"
)]
pub async fn vrem(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<VectorRemRequest>,
) -> Result<Json<ApiResponse<VectorRemResponse>>, CacheError> {
    payload
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let result = state.vector_service.vrem(&key, payload).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    post,
    path = "/api/v1/vectors/{key}/sim",
    request_body = VectorSimRequest,
    responses(
        (status = 200, description = "Vector similarity query successful", body = VectorSimResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key")
    ),
    tag = "Vectors"
)]
pub async fn vsim(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<VectorSimRequest>,
) -> Result<Json<ApiResponse<VectorSimResponse>>, CacheError> {
    payload
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let result = state.vector_service.vsim(&key, payload).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    get,
    path = "/api/v1/vectors/{key}/card",
    responses(
        (status = 200, description = "Vector set cardinality", body = VectorCardResponse),
        (status = 404, description = "Key not found", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key")
    ),
    tag = "Vectors"
)]
pub async fn vcard(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<VectorCardResponse>>, CacheError> {
    let result = state.vector_service.vcard(&key).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    get,
    path = "/api/v1/vectors/{key}/dim",
    responses(
        (status = 200, description = "Vector dimensionality", body = VectorDimResponse),
        (status = 404, description = "Key not found", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key")
    ),
    tag = "Vectors"
)]
pub async fn vdim(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<VectorDimResponse>>, CacheError> {
    let result = state.vector_service.vdim(&key).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    post,
    path = "/api/v1/vectors/{key}/emb",
    request_body = VectorEmbRequest,
    responses(
        (status = 200, description = "Vector embeddings", body = VectorEmbResponse),
        (status = 400, description = "Bad request", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key")
    ),
    tag = "Vectors"
)]
pub async fn vemb(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<VectorEmbRequest>,
) -> Result<Json<ApiResponse<VectorEmbResponse>>, CacheError> {
    payload
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let result = state.vector_service.vemb(&key, payload).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    post,
    path = "/api/v1/vectors/{key}/ismember",
    request_body = VectorIsMemberRequest,
    responses(
        (status = 200, description = "Vector membership check", body = VectorIsMemberResponse),
        (status = 400, description = "Bad request", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key")
    ),
    tag = "Vectors"
)]
pub async fn vismember(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<VectorIsMemberRequest>,
) -> Result<Json<ApiResponse<VectorIsMemberResponse>>, CacheError> {
    payload
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let result = state.vector_service.vismember(&key, payload).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    get,
    path = "/api/v1/vectors/{key}/links/{item}",
    responses(
        (status = 200, description = "Vector HNSW graph neighbors", body = VectorLinksResponse),
        (status = 404, description = "Key not found", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key"),
        ("item" = String, Path, description = "Item key")
    ),
    tag = "Vectors"
)]
pub async fn vlinks(
    State(state): State<AppState>,
    Path((key, item)): Path<(String, String)>,
) -> Result<Json<ApiResponse<VectorLinksResponse>>, CacheError> {
    let result = state.vector_service.vlinks(&key, &item).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    post,
    path = "/api/v1/vectors/{key}/randmember",
    request_body = VectorRandMemberRequest,
    responses(
        (status = 200, description = "Random members", body = VectorRandMemberResponse),
        (status = 400, description = "Bad request", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key")
    ),
    tag = "Vectors"
)]
pub async fn vrandmember(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<VectorRandMemberRequest>,
) -> Result<Json<ApiResponse<VectorRandMemberResponse>>, CacheError> {
    payload
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let result = state.vector_service.vrandmember(&key, payload).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    post,
    path = "/api/v1/vectors/{key}/range",
    request_body = VectorRangeRequest,
    responses(
        (status = 200, description = "Vector range result", body = VectorRangeResponse),
        (status = 400, description = "Bad request", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key")
    ),
    tag = "Vectors"
)]
pub async fn vrange(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<VectorRangeRequest>,
) -> Result<Json<ApiResponse<VectorRangeResponse>>, CacheError> {
    payload
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let result = state.vector_service.vrange(&key, payload).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    get,
    path = "/api/v1/vectors/{key}/info",
    responses(
        (status = 200, description = "Vector set info", body = VectorInfoResponse),
        (status = 404, description = "Key not found", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key")
    ),
    tag = "Vectors"
)]
pub async fn vinfo(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<VectorInfoResponse>>, CacheError> {
    let result = state.vector_service.vinfo(&key).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    get,
    path = "/api/v1/vectors/{key}/attr/{item}",
    responses(
        (status = 200, description = "Vector item attributes (null if no attributes set or element unknown)", body = VectorGetAttrResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key"),
        ("item" = String, Path, description = "Item key")
    ),
    tag = "Vectors"
)]
pub async fn vgetattr(
    State(state): State<AppState>,
    Path((key, item)): Path<(String, String)>,
) -> Result<Json<ApiResponse<VectorGetAttrResponse>>, CacheError> {
    let result = state.vector_service.vgetattr(&key, &item).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    post,
    path = "/api/v1/vectors/{key}/attr/{item}",
    request_body = VectorSetAttrRequest,
    responses(
        (status = 200, description = "Vector attributes set cleanly", body = VectorSetAttrResponse),
        (status = 404, description = "Key not found", body = ErrorResponse)
    ),
    params(
        ("key" = String, Path, description = "Vector set key"),
        ("item" = String, Path, description = "Item key")
    ),
    tag = "Vectors"
)]
pub async fn vsetattr(
    State(state): State<AppState>,
    Path((key, item)): Path<(String, String)>,
    Json(payload): Json<VectorSetAttrRequest>,
) -> Result<Json<ApiResponse<VectorSetAttrResponse>>, CacheError> {
    payload
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;
    let result = state.vector_service.vsetattr(&key, &item, payload).await?;
    Ok(Json(ApiResponse::success(result)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::test_support::test_state_with_vector_repo;

    #[tokio::test]
    async fn test_vector_routes() {
        let (state, _) = test_state_with_vector_repo();
        let app = router().merge(vector_range_router()).with_state(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/vectors/mykey/add")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"items":{"item1":[0.1,0.2]}}"#))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/vectors/mykey/rem")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"items":["item1"]}"#))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/vectors/mykey/sim")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"vector":[0.1,0.2],"k":10}"#))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/vectors/mykey/card")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/vectors/mykey/dim")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/vectors/mykey/emb")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"items":["item1"]}"#))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/vectors/mykey/ismember")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"items":["item1"]}"#))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/vectors/mykey/links/item1")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/vectors/mykey/randmember")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"count":1}"#))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/vectors/mykey/range")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"start":"-","end":"+"}"#))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/vectors/mykey/info")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/vectors/mykey/attr/item1")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/vectors/mykey/attr/item1")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"attributes":"{}"}"#))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );
    }
}
