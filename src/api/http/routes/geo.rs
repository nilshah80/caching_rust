//! Geo Routes
//!
//! HTTP endpoints for Redis geospatial operations.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};

use crate::api::http::schemas::geo::{
    GeoAddRequest, GeoAddResponse, GeoDistQuery, GeoDistResponse, GeoHashRequest, GeoHashResponse,
    GeoPosRequest, GeoPosResponse, GeoPositionSchema, GeoRadiusByMemberQuery, GeoRadiusQuery,
    GeoSearchRequest, GeoSearchResponse, GeoSearchResultItem, GeoSearchStoreRequest,
    GeoSearchStoreResponse,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::GeoPosition;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create Geo routes
pub fn geo_routes() -> Router<AppState> {
    Router::new()
        // Core geo operations
        .route("/api/v1/geo/{key}", post(geo_add))
        .route("/api/v1/geo/{key}/pos", post(geo_pos))
        .route("/api/v1/geo/{key}/dist/{member1}/{member2}", get(geo_dist))
        .route("/api/v1/geo/{key}/hash", post(geo_hash))
        // Modern search operations (Redis 6.2+)
        .route("/api/v1/geo/{key}/search", post(geo_search))
        .route("/api/v1/geo/{dest_key}/searchstore", post(geo_search_store))
        // Legacy operations (deprecated but still supported)
        .route("/api/v1/geo/{key}/radius", get(geo_radius))
        .route(
            "/api/v1/geo/{key}/radius/{member}",
            get(geo_radius_by_member),
        )
}

/// GEOADD - Add one or more geospatial items to a sorted set
#[utoipa::path(
    post,
    path = "/api/v1/geo/{key}",
    tag = "Geo",
    params(
        ("key" = String, Path, description = "The geo key")
    ),
    request_body = GeoAddRequest,
    responses(
        (status = 200, description = "Members added successfully", body = GeoAddResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn geo_add(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<GeoAddRequest>,
) -> Result<Json<ApiResponse<GeoAddResponse>>, CacheError> {
    let options = request.to_options();
    let members: Vec<_> = request.members.into_iter().map(|m| m.into()).collect();
    let result = state.geo_service.geo_add(&key, members, options).await?;
    Ok(Json(ApiResponse::success(GeoAddResponse {
        added: result.added,
        changed: result.changed,
    })))
}

/// GEOPOS - Get the positions of specified members
#[utoipa::path(
    post,
    path = "/api/v1/geo/{key}/pos",
    tag = "Geo",
    params(
        ("key" = String, Path, description = "The geo key")
    ),
    request_body = GeoPosRequest,
    responses(
        (status = 200, description = "Positions retrieved", body = GeoPosResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn geo_pos(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<GeoPosRequest>,
) -> Result<Json<ApiResponse<GeoPosResponse>>, CacheError> {
    let positions = state.geo_service.geo_pos(&key, request.members).await?;
    let positions: Vec<Option<GeoPositionSchema>> = positions
        .into_iter()
        .map(|p| p.map(|pos| pos.into()))
        .collect();
    Ok(Json(ApiResponse::success(GeoPosResponse { positions })))
}

/// GEODIST - Get the distance between two members
#[utoipa::path(
    get,
    path = "/api/v1/geo/{key}/dist/{member1}/{member2}",
    tag = "Geo",
    params(
        ("key" = String, Path, description = "The geo key"),
        ("member1" = String, Path, description = "First member"),
        ("member2" = String, Path, description = "Second member"),
        ("unit" = Option<String>, Query, description = "Distance unit: m, km, mi, ft (default: m)")
    ),
    responses(
        (status = 200, description = "Distance calculated", body = GeoDistResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn geo_dist(
    State(state): State<AppState>,
    Path((key, member1, member2)): Path<(String, String, String)>,
    Query(query): Query<GeoDistQuery>,
) -> Result<Json<ApiResponse<GeoDistResponse>>, CacheError> {
    let unit = query.unit.clone();
    let distance = state
        .geo_service
        .geo_dist(&key, &member1, &member2, unit.clone().into())
        .await?;
    Ok(Json(ApiResponse::success(GeoDistResponse {
        distance,
        unit,
    })))
}

/// GEOHASH - Get the geohash strings of members
#[utoipa::path(
    post,
    path = "/api/v1/geo/{key}/hash",
    tag = "Geo",
    params(
        ("key" = String, Path, description = "The geo key")
    ),
    request_body = GeoHashRequest,
    responses(
        (status = 200, description = "Geohashes retrieved", body = GeoHashResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn geo_hash(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<GeoHashRequest>,
) -> Result<Json<ApiResponse<GeoHashResponse>>, CacheError> {
    let hashes = state.geo_service.geo_hash(&key, request.members).await?;
    Ok(Json(ApiResponse::success(GeoHashResponse { hashes })))
}

/// GEOSEARCH - Search for members in a geographic area
#[utoipa::path(
    post,
    path = "/api/v1/geo/{key}/search",
    tag = "Geo",
    params(
        ("key" = String, Path, description = "The geo key")
    ),
    request_body = GeoSearchRequest,
    responses(
        (status = 200, description = "Search results", body = GeoSearchResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn geo_search(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<GeoSearchRequest>,
) -> Result<Json<ApiResponse<GeoSearchResponse>>, CacheError> {
    let results = state
        .geo_service
        .geo_search(
            &key,
            request.center.into(),
            request.shape.into(),
            request.options.into(),
        )
        .await?;
    let results: Vec<GeoSearchResultItem> = results.into_iter().map(|r| r.into()).collect();
    Ok(Json(ApiResponse::success(GeoSearchResponse { results })))
}

/// GEOSEARCHSTORE - Search and store results in a destination key
#[utoipa::path(
    post,
    path = "/api/v1/geo/{dest_key}/searchstore",
    tag = "Geo",
    params(
        ("dest_key" = String, Path, description = "Destination key to store results")
    ),
    request_body = GeoSearchStoreRequest,
    responses(
        (status = 200, description = "Results stored", body = GeoSearchStoreResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn geo_search_store(
    State(state): State<AppState>,
    Path(dest_key): Path<String>,
    Json(request): Json<GeoSearchStoreRequest>,
) -> Result<Json<ApiResponse<GeoSearchStoreResponse>>, CacheError> {
    let result = state
        .geo_service
        .geo_search_store(
            &dest_key,
            &request.source_key,
            request.center.into(),
            request.shape.into(),
            request.options.into(),
            request.store_dist,
        )
        .await?;
    Ok(Json(ApiResponse::success(GeoSearchStoreResponse {
        stored: result.stored,
    })))
}

/// GEORADIUS - Query members within a radius (legacy, use GEOSEARCH instead)
#[utoipa::path(
    get,
    path = "/api/v1/geo/{key}/radius",
    tag = "Geo",
    params(
        ("key" = String, Path, description = "The geo key"),
        ("longitude" = f64, Query, description = "Center longitude"),
        ("latitude" = f64, Query, description = "Center latitude"),
        ("radius" = f64, Query, description = "Search radius"),
        ("unit" = Option<String>, Query, description = "Distance unit: m, km, mi, ft (default: m)"),
        ("with_dist" = Option<bool>, Query, description = "Include distance in results"),
        ("with_coord" = Option<bool>, Query, description = "Include coordinates in results"),
        ("with_hash" = Option<bool>, Query, description = "Include geohash in results"),
        ("sort" = Option<String>, Query, description = "Sort order: ASC or DESC"),
        ("count" = Option<usize>, Query, description = "Maximum number of results")
    ),
    responses(
        (status = 200, description = "Search results", body = GeoSearchResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn geo_radius(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<GeoRadiusQuery>,
) -> Result<Json<ApiResponse<GeoSearchResponse>>, CacheError> {
    let position = GeoPosition::new(query.longitude, query.latitude);
    let results = state
        .geo_service
        .geo_radius(
            &key,
            position,
            query.radius,
            query.unit.clone().into(),
            query.to_options(),
        )
        .await?;
    let results: Vec<GeoSearchResultItem> = results.into_iter().map(|r| r.into()).collect();
    Ok(Json(ApiResponse::success(GeoSearchResponse { results })))
}

/// GEORADIUSBYMEMBER - Query members within a radius of an existing member (legacy, use GEOSEARCH instead)
#[utoipa::path(
    get,
    path = "/api/v1/geo/{key}/radius/{member}",
    tag = "Geo",
    params(
        ("key" = String, Path, description = "The geo key"),
        ("member" = String, Path, description = "Member to search around"),
        ("radius" = f64, Query, description = "Search radius"),
        ("unit" = Option<String>, Query, description = "Distance unit: m, km, mi, ft (default: m)"),
        ("with_dist" = Option<bool>, Query, description = "Include distance in results"),
        ("with_coord" = Option<bool>, Query, description = "Include coordinates in results"),
        ("with_hash" = Option<bool>, Query, description = "Include geohash in results"),
        ("sort" = Option<String>, Query, description = "Sort order: ASC or DESC"),
        ("count" = Option<usize>, Query, description = "Maximum number of results")
    ),
    responses(
        (status = 200, description = "Search results", body = GeoSearchResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn geo_radius_by_member(
    State(state): State<AppState>,
    Path((key, member)): Path<(String, String)>,
    Query(query): Query<GeoRadiusByMemberQuery>,
) -> Result<Json<ApiResponse<GeoSearchResponse>>, CacheError> {
    let results = state
        .geo_service
        .geo_radius_by_member(
            &key,
            &member,
            query.radius,
            query.unit.clone().into(),
            query.to_options(),
        )
        .await?;
    let results: Vec<GeoSearchResultItem> = results.into_iter().map(|r| r.into()).collect();
    Ok(Json(ApiResponse::success(GeoSearchResponse { results })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::{GeoAddOptions, GeoMember};
    use crate::test_support::test_state_with_geo_repo;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[test]
    fn test_geo_routes_structure() {
        let _routes = geo_routes();
    }

    #[tokio::test]
    async fn test_geo_add() {
        let (state, _) = test_state_with_geo_repo();
        let app = geo_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/geo/locations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{
                            "members": [
                                {"member": "Berlin", "longitude": 13.361389, "latitude": 52.519444},
                                {"member": "Paris", "longitude": 2.349014, "latitude": 48.864716}
                            ]
                        }"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_geo_pos() {
        let (state, _) = test_state_with_geo_repo();
        let app = geo_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/geo/locations/pos")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"members": ["Berlin", "Paris"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_geo_dist() {
        let (state, _) = test_state_with_geo_repo();
        let app = geo_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/geo/locations/dist/Berlin/Paris?unit=km")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_geo_hash() {
        let (state, _) = test_state_with_geo_repo();
        let app = geo_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/geo/locations/hash")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"members": ["Berlin"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_geo_search() {
        let (state, _) = test_state_with_geo_repo();
        let app = geo_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/geo/locations/search")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{
                            "center": {"type": "FROMLONLAT", "longitude": 13.361389, "latitude": 52.519444},
                            "shape": {"type": "BYRADIUS", "radius": 500.0, "unit": "km"},
                            "options": {"with_dist": true, "sort": "ASC", "count": 10}
                        }"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_geo_search_store() {
        let (state, _) = test_state_with_geo_repo();
        let app = geo_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/geo/nearby/searchstore")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{
                            "source_key": "locations",
                            "center": {"type": "FROMMEMBER", "member": "Berlin"},
                            "shape": {"type": "BYRADIUS", "radius": 1000.0, "unit": "km"}
                        }"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_geo_radius() {
        let (state, _) = test_state_with_geo_repo();
        let app = geo_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/geo/locations/radius?longitude=13.361389&latitude=52.519444&radius=500&unit=km&with_dist=true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_geo_radius_by_member() {
        let (state, _) = test_state_with_geo_repo();
        let app = geo_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/geo/locations/radius/Berlin?radius=500&unit=km")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_geo_search_results_mapping() {
        let (state, _) = test_state_with_geo_repo();
        state
            .geo_service
            .geo_add(
                "locations",
                vec![
                    GeoMember::new("Berlin", 13.361389, 52.519444),
                    GeoMember::new("Paris", 2.349014, 48.864716),
                ],
                GeoAddOptions::default(),
            )
            .await
            .unwrap();

        let app = geo_routes().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/geo/locations/search")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{
                            "center": {"type": "FROMLONLAT", "longitude": 13.361389, "latitude": 52.519444},
                            "shape": {"type": "BYRADIUS", "radius": 500.0, "unit": "km"},
                            "options": {"with_dist": true}
                        }"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["data"]["results"].is_array());
    }

    #[tokio::test]
    async fn test_geo_radius_results_mapping() {
        let (state, _) = test_state_with_geo_repo();
        state
            .geo_service
            .geo_add(
                "locations",
                vec![GeoMember::new("Berlin", 13.361389, 52.519444)],
                GeoAddOptions::default(),
            )
            .await
            .unwrap();

        let app = geo_routes().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/geo/locations/radius?longitude=13.361389&latitude=52.519444&radius=500&unit=km")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["data"]["results"].is_array());
    }

    #[tokio::test]
    async fn test_geo_radius_by_member_results_mapping() {
        let (state, _) = test_state_with_geo_repo();
        state
            .geo_service
            .geo_add(
                "locations",
                vec![GeoMember::new("Berlin", 13.361389, 52.519444)],
                GeoAddOptions::default(),
            )
            .await
            .unwrap();

        let app = geo_routes().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/geo/locations/radius/Berlin?radius=500&unit=km")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["data"]["results"].is_array());
    }
}
