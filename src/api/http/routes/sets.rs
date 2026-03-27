//! Set Routes
//!
//! HTTP endpoints for Redis set operations.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post},
};

use crate::api::http::schemas::sets::{
    SetAddRequest, SetAddResponse, SetAlgebraRequest, SetAlgebraResponse, SetAlgebraStoreRequest,
    SetAlgebraStoreResponse, SetCardResponse, SetInterCardRequest, SetInterCardResponse,
    SetIsMemberRequest, SetIsMemberResponse, SetMIsMemberRequest, SetMIsMemberResponse,
    SetMembersResponse, SetMoveRequest, SetMoveResponse, SetPopRequest, SetPopResponse,
    SetRandMemberQuery, SetRandMemberResponse, SetRemoveRequest, SetRemoveResponse, SetScanQuery,
    SetScanResponse,
};
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create Set routes
pub fn set_routes() -> Router<AppState> {
    Router::new()
        // Basic operations
        .route("/api/v1/sets/{key}/members", post(sadd))
        .route("/api/v1/sets/{key}/members", get(smembers))
        .route("/api/v1/sets/{key}/members", delete(srem))
        .route("/api/v1/sets/{key}/ismember", post(sismember))
        .route("/api/v1/sets/{key}/mismember", post(smismember))
        .route("/api/v1/sets/{key}/card", get(scard))
        // Random access operations
        .route("/api/v1/sets/{key}/random", get(srandmember))
        .route("/api/v1/sets/{key}/pop", post(spop))
        .route("/api/v1/sets/move", post(smove))
        // Set algebra operations
        .route("/api/v1/sets/inter", post(sinter))
        .route("/api/v1/sets/interstore", post(sinterstore))
        .route("/api/v1/sets/intercard", post(sintercard))
        .route("/api/v1/sets/union", post(sunion))
        .route("/api/v1/sets/unionstore", post(sunionstore))
        .route("/api/v1/sets/diff", post(sdiff))
        .route("/api/v1/sets/diffstore", post(sdiffstore))
        // Scan operation
        .route("/api/v1/sets/{key}/scan", get(sscan))
}

/// SADD - Add members to a set
#[utoipa::path(
    post,
    path = "/api/v1/sets/{key}/members",
    tag = "Sets",
    params(
        ("key" = String, Path, description = "The set key")
    ),
    request_body = SetAddRequest,
    responses(
        (status = 200, description = "Members added successfully", body = SetAddResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn sadd(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SetAddRequest>,
) -> Result<Json<ApiResponse<SetAddResponse>>, CacheError> {
    let added = state.set_service.sadd(&key, request.members).await?;
    Ok(Json(ApiResponse::success(SetAddResponse { added })))
}

/// SREM - Remove members from a set
#[utoipa::path(
    delete,
    path = "/api/v1/sets/{key}/members",
    tag = "Sets",
    params(
        ("key" = String, Path, description = "The set key")
    ),
    request_body = SetRemoveRequest,
    responses(
        (status = 200, description = "Members removed successfully", body = SetRemoveResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn srem(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SetRemoveRequest>,
) -> Result<Json<ApiResponse<SetRemoveResponse>>, CacheError> {
    let removed = state.set_service.srem(&key, request.members).await?;
    Ok(Json(ApiResponse::success(SetRemoveResponse { removed })))
}

/// SMEMBERS - Get all members of a set
#[utoipa::path(
    get,
    path = "/api/v1/sets/{key}/members",
    tag = "Sets",
    params(
        ("key" = String, Path, description = "The set key")
    ),
    responses(
        (status = 200, description = "Set members", body = SetMembersResponse)
    )
)]
pub async fn smembers(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<SetMembersResponse>>, CacheError> {
    let members = state.set_service.smembers(&key).await?;
    Ok(Json(ApiResponse::success(SetMembersResponse { members })))
}

/// SISMEMBER - Check if a member exists in a set
#[utoipa::path(
    post,
    path = "/api/v1/sets/{key}/ismember",
    tag = "Sets",
    params(
        ("key" = String, Path, description = "The set key")
    ),
    request_body = SetIsMemberRequest,
    responses(
        (status = 200, description = "Member existence check result", body = SetIsMemberResponse)
    )
)]
pub async fn sismember(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SetIsMemberRequest>,
) -> Result<Json<ApiResponse<SetIsMemberResponse>>, CacheError> {
    let is_member = state.set_service.sismember(&key, &request.member).await?;
    Ok(Json(ApiResponse::success(SetIsMemberResponse {
        is_member,
    })))
}

/// SMISMEMBER - Check if multiple members exist in a set
#[utoipa::path(
    post,
    path = "/api/v1/sets/{key}/mismember",
    tag = "Sets",
    params(
        ("key" = String, Path, description = "The set key")
    ),
    request_body = SetMIsMemberRequest,
    responses(
        (status = 200, description = "Member existence check results", body = SetMIsMemberResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn smismember(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SetMIsMemberRequest>,
) -> Result<Json<ApiResponse<SetMIsMemberResponse>>, CacheError> {
    let results = state.set_service.smismember(&key, request.members).await?;
    Ok(Json(ApiResponse::success(SetMIsMemberResponse { results })))
}

/// SCARD - Get the number of members in a set
#[utoipa::path(
    get,
    path = "/api/v1/sets/{key}/card",
    tag = "Sets",
    params(
        ("key" = String, Path, description = "The set key")
    ),
    responses(
        (status = 200, description = "Set cardinality", body = SetCardResponse)
    )
)]
pub async fn scard(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<SetCardResponse>>, CacheError> {
    let cardinality = state.set_service.scard(&key).await?;
    Ok(Json(ApiResponse::success(SetCardResponse { cardinality })))
}

/// SRANDMEMBER - Get random members from a set without removing them
#[utoipa::path(
    get,
    path = "/api/v1/sets/{key}/random",
    tag = "Sets",
    params(
        ("key" = String, Path, description = "The set key"),
        ("count" = Option<i64>, Query, description = "Number of members (positive = distinct, negative = may repeat)")
    ),
    responses(
        (status = 200, description = "Random members", body = SetRandMemberResponse)
    )
)]
pub async fn srandmember(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<SetRandMemberQuery>,
) -> Result<Json<ApiResponse<SetRandMemberResponse>>, CacheError> {
    let members = state.set_service.srandmember(&key, query.count).await?;
    Ok(Json(ApiResponse::success(SetRandMemberResponse {
        members,
    })))
}

/// SPOP - Remove and return random members from a set
#[utoipa::path(
    post,
    path = "/api/v1/sets/{key}/pop",
    tag = "Sets",
    params(
        ("key" = String, Path, description = "The set key")
    ),
    request_body = SetPopRequest,
    responses(
        (status = 200, description = "Popped members", body = SetPopResponse)
    )
)]
pub async fn spop(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SetPopRequest>,
) -> Result<Json<ApiResponse<SetPopResponse>>, CacheError> {
    let members = state.set_service.spop(&key, request.count).await?;
    Ok(Json(ApiResponse::success(SetPopResponse { members })))
}

/// SMOVE - Move a member from one set to another
#[utoipa::path(
    post,
    path = "/api/v1/sets/move",
    tag = "Sets",
    request_body = SetMoveRequest,
    responses(
        (status = 200, description = "Move result", body = SetMoveResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn smove(
    State(state): State<AppState>,
    Json(request): Json<SetMoveRequest>,
) -> Result<Json<ApiResponse<SetMoveResponse>>, CacheError> {
    let moved = state
        .set_service
        .smove(&request.source, &request.destination, &request.member)
        .await?;
    Ok(Json(ApiResponse::success(SetMoveResponse { moved })))
}

/// SINTER - Get the intersection of multiple sets
#[utoipa::path(
    post,
    path = "/api/v1/sets/inter",
    tag = "Sets",
    request_body = SetAlgebraRequest,
    responses(
        (status = 200, description = "Intersection members", body = SetAlgebraResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn sinter(
    State(state): State<AppState>,
    Json(request): Json<SetAlgebraRequest>,
) -> Result<Json<ApiResponse<SetAlgebraResponse>>, CacheError> {
    let members = state.set_service.sinter(request.keys).await?;
    Ok(Json(ApiResponse::success(SetAlgebraResponse { members })))
}

/// SINTERSTORE - Store the intersection of multiple sets
#[utoipa::path(
    post,
    path = "/api/v1/sets/interstore",
    tag = "Sets",
    request_body = SetAlgebraStoreRequest,
    responses(
        (status = 200, description = "Number of members in resulting set", body = SetAlgebraStoreResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn sinterstore(
    State(state): State<AppState>,
    Json(request): Json<SetAlgebraStoreRequest>,
) -> Result<Json<ApiResponse<SetAlgebraStoreResponse>>, CacheError> {
    let count = state
        .set_service
        .sinterstore(&request.destination, request.keys)
        .await?;
    Ok(Json(ApiResponse::success(SetAlgebraStoreResponse {
        count,
    })))
}

/// SINTERCARD - Get the cardinality of the intersection
#[utoipa::path(
    post,
    path = "/api/v1/sets/intercard",
    tag = "Sets",
    request_body = SetInterCardRequest,
    responses(
        (status = 200, description = "Intersection cardinality", body = SetInterCardResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn sintercard(
    State(state): State<AppState>,
    Json(request): Json<SetInterCardRequest>,
) -> Result<Json<ApiResponse<SetInterCardResponse>>, CacheError> {
    let cardinality = state
        .set_service
        .sintercard(request.keys, request.limit)
        .await?;
    Ok(Json(ApiResponse::success(SetInterCardResponse {
        cardinality,
    })))
}

/// SUNION - Get the union of multiple sets
#[utoipa::path(
    post,
    path = "/api/v1/sets/union",
    tag = "Sets",
    request_body = SetAlgebraRequest,
    responses(
        (status = 200, description = "Union members", body = SetAlgebraResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn sunion(
    State(state): State<AppState>,
    Json(request): Json<SetAlgebraRequest>,
) -> Result<Json<ApiResponse<SetAlgebraResponse>>, CacheError> {
    let members = state.set_service.sunion(request.keys).await?;
    Ok(Json(ApiResponse::success(SetAlgebraResponse { members })))
}

/// SUNIONSTORE - Store the union of multiple sets
#[utoipa::path(
    post,
    path = "/api/v1/sets/unionstore",
    tag = "Sets",
    request_body = SetAlgebraStoreRequest,
    responses(
        (status = 200, description = "Number of members in resulting set", body = SetAlgebraStoreResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn sunionstore(
    State(state): State<AppState>,
    Json(request): Json<SetAlgebraStoreRequest>,
) -> Result<Json<ApiResponse<SetAlgebraStoreResponse>>, CacheError> {
    let count = state
        .set_service
        .sunionstore(&request.destination, request.keys)
        .await?;
    Ok(Json(ApiResponse::success(SetAlgebraStoreResponse {
        count,
    })))
}

/// SDIFF - Get the difference of sets
#[utoipa::path(
    post,
    path = "/api/v1/sets/diff",
    tag = "Sets",
    request_body = SetAlgebraRequest,
    responses(
        (status = 200, description = "Difference members", body = SetAlgebraResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn sdiff(
    State(state): State<AppState>,
    Json(request): Json<SetAlgebraRequest>,
) -> Result<Json<ApiResponse<SetAlgebraResponse>>, CacheError> {
    let members = state.set_service.sdiff(request.keys).await?;
    Ok(Json(ApiResponse::success(SetAlgebraResponse { members })))
}

/// SDIFFSTORE - Store the difference of sets
#[utoipa::path(
    post,
    path = "/api/v1/sets/diffstore",
    tag = "Sets",
    request_body = SetAlgebraStoreRequest,
    responses(
        (status = 200, description = "Number of members in resulting set", body = SetAlgebraStoreResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn sdiffstore(
    State(state): State<AppState>,
    Json(request): Json<SetAlgebraStoreRequest>,
) -> Result<Json<ApiResponse<SetAlgebraStoreResponse>>, CacheError> {
    let count = state
        .set_service
        .sdiffstore(&request.destination, request.keys)
        .await?;
    Ok(Json(ApiResponse::success(SetAlgebraStoreResponse {
        count,
    })))
}

/// SSCAN - Incrementally iterate set members
#[utoipa::path(
    get,
    path = "/api/v1/sets/{key}/scan",
    tag = "Sets",
    params(
        ("key" = String, Path, description = "The set key"),
        ("cursor" = Option<u64>, Query, description = "Cursor position (0 to start)"),
        ("pattern" = Option<String>, Query, description = "Pattern to match members"),
        ("count" = Option<u64>, Query, description = "Hint for number of members to return")
    ),
    responses(
        (status = 200, description = "Scan result", body = SetScanResponse)
    )
)]
pub async fn sscan(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<SetScanQuery>,
) -> Result<Json<ApiResponse<SetScanResponse>>, CacheError> {
    let result = state
        .set_service
        .sscan(&key, query.cursor, query.pattern.as_deref(), query.count)
        .await?;
    Ok(Json(ApiResponse::success(SetScanResponse {
        cursor: result.cursor,
        members: result.members,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_state, test_state_with_set_repo};
    use axum::Json;
    use axum::extract::{Path, Query, State};
    use axum::http::Request;
    use axum::http::StatusCode;
    use std::collections::HashSet;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_set_routes() {
        let (state, _, _, _) = test_state();
        let app = set_routes().with_state(state);

        // Test SADD
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/sets/myset/members")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(r#"{"members": ["a", "b", "c"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test SMEMBERS
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/sets/myset/members")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test SCARD
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/sets/myset/card")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test SISMEMBER
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/sets/myset/ismember")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(r#"{"member": "a"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_set_algebra_routes() {
        let (state, _, _, _) = test_state();
        let app = set_routes().with_state(state);

        // Test SINTER
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/sets/inter")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(r#"{"keys": ["set1", "set2"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test SUNION
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/sets/union")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(r#"{"keys": ["set1", "set2"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test SDIFF
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/sets/diff")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(r#"{"keys": ["set1", "set2"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_set_routes_additional_handlers() {
        let (state, set_repo) = test_state_with_set_repo();
        set_repo.insert("remset", vec!["a".to_string(), "b".to_string()]);
        set_repo.insert("misset", vec!["a".to_string()]);
        set_repo.insert(
            "randset",
            vec!["x".to_string(), "y".to_string(), "z".to_string()],
        );
        set_repo.insert("popset", vec!["p".to_string(), "q".to_string()]);
        set_repo.insert("movesrc", vec!["m".to_string()]);
        set_repo.insert("inter1", vec!["a".to_string(), "b".to_string()]);
        set_repo.insert("inter2", vec!["b".to_string(), "c".to_string()]);
        set_repo.insert("union1", vec!["u1".to_string()]);
        set_repo.insert("union2", vec!["u2".to_string()]);
        set_repo.insert("diff1", vec!["d1".to_string(), "d2".to_string()]);
        set_repo.insert("diff2", vec!["d2".to_string()]);
        set_repo.insert("scanset", vec!["s1".to_string(), "s2".to_string()]);
        let state = State(state);

        let removed = srem(
            state.clone(),
            Path("remset".to_string()),
            Json(SetRemoveRequest {
                members: vec!["a".to_string(), "missing".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(removed.0.data.expect("data").removed, 1);

        let mismember = smismember(
            state.clone(),
            Path("misset".to_string()),
            Json(SetMIsMemberRequest {
                members: vec!["a".to_string(), "b".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(mismember.0.data.expect("data").results, vec![true, false]);

        let random = srandmember(
            state.clone(),
            Path("randset".to_string()),
            Query(SetRandMemberQuery { count: Some(2) }),
        )
        .await
        .unwrap();
        let members = random.0.data.expect("data").members;
        assert_eq!(members.len(), 2);
        let allowed: HashSet<String> = ["x", "y", "z"].iter().map(|m| m.to_string()).collect();
        for member in &members {
            assert!(allowed.contains(member));
        }

        let popped = spop(
            state.clone(),
            Path("popset".to_string()),
            Json(SetPopRequest { count: Some(1) }),
        )
        .await
        .unwrap();
        assert_eq!(popped.0.data.expect("data").members.len(), 1);

        let moved = smove(
            state.clone(),
            Json(SetMoveRequest {
                source: "movesrc".to_string(),
                destination: "movedest".to_string(),
                member: "m".to_string(),
            }),
        )
        .await
        .unwrap();
        assert!(moved.0.data.expect("data").moved);

        let interstore = sinterstore(
            state.clone(),
            Json(SetAlgebraStoreRequest {
                destination: "interdest".to_string(),
                keys: vec!["inter1".to_string(), "inter2".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(interstore.0.data.expect("data").count, 1);

        let intercard = sintercard(
            state.clone(),
            Json(SetInterCardRequest {
                keys: vec!["inter1".to_string(), "inter2".to_string()],
                limit: Some(1),
            }),
        )
        .await
        .unwrap();
        assert_eq!(intercard.0.data.expect("data").cardinality, 1);

        let unionstore = sunionstore(
            state.clone(),
            Json(SetAlgebraStoreRequest {
                destination: "uniondest".to_string(),
                keys: vec!["union1".to_string(), "union2".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(unionstore.0.data.expect("data").count, 2);

        let diffstore = sdiffstore(
            state.clone(),
            Json(SetAlgebraStoreRequest {
                destination: "diffdest".to_string(),
                keys: vec!["diff1".to_string(), "diff2".to_string()],
            }),
        )
        .await
        .unwrap();
        assert_eq!(diffstore.0.data.expect("data").count, 1);

        let scan = sscan(
            state,
            Path("scanset".to_string()),
            Query(SetScanQuery {
                cursor: 0,
                pattern: Some("s".to_string()),
                count: Some(1),
            }),
        )
        .await
        .unwrap();
        let scan_data = scan.0.data.expect("data");
        assert_eq!(scan_data.cursor, 0);
        assert!(scan_data.members.contains(&"s1".to_string()));
    }
}
