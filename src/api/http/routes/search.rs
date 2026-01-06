//! Search Routes
//!
//! HTTP endpoints for RediSearch operations.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use validator::Validate;

use crate::api::http::schemas::search::{
    AggregateRequest, AggregateResponse, AliasRequest, AliasResponse,
    AlterIndexRequest, AlterIndexResponse, CreateIndexRequest, CreateIndexResponse,
    DictDumpResponse, DictResponse, DictTermsRequest, DropIndexParams, DropIndexResponse,
    ExplainRequest, ExplainResponse, IndexInfoResponse, ListIndicesResponse,
    ProfileRequest, ProfileResponse, SearchRequest, SearchResponse,
    SpellcheckRequest, SpellcheckResponse, SugAddRequest, SugAddResponse, SugDelRequest,
    SugDelResponse, SugGetParams, SugGetResponse, SugLenResponse, SynonymDumpResponse,
    SynonymUpdateRequest, SynonymUpdateResponse, parse_profile_type,
};
use crate::domain::entities::{SugAddOptions, SugGetOptions};
use crate::domain::errors::CacheError;
use crate::shared::app_state::AppState;
use crate::shared::response::ApiResponse;

/// Create search routes
pub fn search_routes() -> Router<AppState> {
    Router::new()
        // Index operations
        .route("/api/v1/search/indices", post(create_index))
        .route("/api/v1/search/indices", get(list_indices))
        .route("/api/v1/search/indices/{index}", get(get_index_info))
        .route("/api/v1/search/indices/{index}", delete(drop_index))
        .route("/api/v1/search/indices/{index}/fields", post(alter_index))
        // Query operations
        .route("/api/v1/search/indices/{index}/search", post(search))
        .route("/api/v1/search/indices/{index}/aggregate", post(aggregate))
        .route("/api/v1/search/indices/{index}/explain", post(explain))
        .route("/api/v1/search/indices/{index}/profile", post(profile))
        // Alias operations
        .route("/api/v1/search/aliases", post(alias_add))
        .route("/api/v1/search/aliases/{alias}", delete(alias_del))
        .route("/api/v1/search/aliases/{alias}", put(alias_update))
        // Autocomplete operations
        .route("/api/v1/search/suggest/{key}", post(sug_add))
        .route("/api/v1/search/suggest/{key}", get(sug_get))
        .route("/api/v1/search/suggest/{key}", delete(sug_del))
        .route("/api/v1/search/suggest/{key}/len", get(sug_len))
        // Synonym operations
        .route("/api/v1/search/indices/{index}/synonyms", get(syn_dump))
        .route("/api/v1/search/indices/{index}/synonyms/{group}", put(syn_update))
        // Spellcheck operations
        .route("/api/v1/search/indices/{index}/spellcheck", post(spellcheck))
        // Dictionary operations
        .route("/api/v1/search/dicts/{dict}/terms", post(dict_add))
        .route("/api/v1/search/dicts/{dict}/terms", delete(dict_del))
        .route("/api/v1/search/dicts/{dict}/terms", get(dict_dump))
}

// ==================== Index Operations ====================

/// POST /api/v1/search/indices
///
/// Create a search index (FT.CREATE).
#[utoipa::path(
    post,
    path = "/api/v1/search/indices",
    request_body = CreateIndexRequest,
    responses(
        (status = 200, description = "Index created successfully", body = CreateIndexResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn create_index(
    State(state): State<AppState>,
    Json(request): Json<CreateIndexRequest>,
) -> Result<Json<ApiResponse<CreateIndexResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let options = request.options.into();
    let schema: Vec<_> = request.schema
        .into_iter()
        .map(|f| f.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: crate::api::http::schemas::search::InvalidFieldTypeError| {
            CacheError::InvalidInput(e.to_string())
        })?;

    let result = state.search_service.ft_create(&request.index, options, schema).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/search/indices
///
/// List all search indices (FT._LIST).
#[utoipa::path(
    get,
    path = "/api/v1/search/indices",
    responses(
        (status = 200, description = "List of indices", body = ListIndicesResponse),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn list_indices(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ListIndicesResponse>>, CacheError> {
    let indices = state.search_service.ft_list().await?;
    Ok(Json(ApiResponse::new(ListIndicesResponse { indices })))
}

/// GET /api/v1/search/indices/:index
///
/// Get index information (FT.INFO).
#[utoipa::path(
    get,
    path = "/api/v1/search/indices/{index}",
    params(
        ("index" = String, Path, description = "Index name")
    ),
    responses(
        (status = 200, description = "Index information", body = IndexInfoResponse),
        (status = 404, description = "Index not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn get_index_info(
    State(state): State<AppState>,
    Path(index): Path<String>,
) -> Result<Json<ApiResponse<IndexInfoResponse>>, CacheError> {
    let info = state.search_service.ft_info(&index).await?;
    Ok(Json(ApiResponse::new(IndexInfoResponse { info })))
}

/// DELETE /api/v1/search/indices/:index
///
/// Drop a search index (FT.DROPINDEX).
#[utoipa::path(
    delete,
    path = "/api/v1/search/indices/{index}",
    params(
        ("index" = String, Path, description = "Index name"),
        ("dd" = bool, Query, description = "Delete indexed documents")
    ),
    responses(
        (status = 200, description = "Index dropped", body = DropIndexResponse),
        (status = 404, description = "Index not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn drop_index(
    State(state): State<AppState>,
    Path(index): Path<String>,
    Query(params): Query<DropIndexParams>,
) -> Result<Json<ApiResponse<DropIndexResponse>>, CacheError> {
    let result = state
        .search_service
        .ft_drop_index(&index, params.dd)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/search/indices/:index/fields
///
/// Add a field to an existing index (FT.ALTER).
#[utoipa::path(
    post,
    path = "/api/v1/search/indices/{index}/fields",
    params(
        ("index" = String, Path, description = "Index name")
    ),
    request_body = AlterIndexRequest,
    responses(
        (status = 200, description = "Field added", body = AlterIndexResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Index not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn alter_index(
    State(state): State<AppState>,
    Path(index): Path<String>,
    Json(request): Json<AlterIndexRequest>,
) -> Result<Json<ApiResponse<AlterIndexResponse>>, CacheError> {
    let field = request.field.try_into()
        .map_err(|e: crate::api::http::schemas::search::InvalidFieldTypeError| {
            CacheError::InvalidInput(e.to_string())
        })?;
    let result = state.search_service.ft_alter(&index, field).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

// ==================== Query Operations ====================

/// POST /api/v1/search/indices/:index/search
///
/// Execute a search query (FT.SEARCH).
#[utoipa::path(
    post,
    path = "/api/v1/search/indices/{index}/search",
    params(
        ("index" = String, Path, description = "Index name")
    ),
    request_body = SearchRequest,
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Index not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn search(
    State(state): State<AppState>,
    Path(index): Path<String>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<ApiResponse<SearchResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let options = request.options.into();
    let result = state.search_service.ft_search(&index, &request.query, options).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/search/indices/:index/aggregate
///
/// Execute an aggregation query (FT.AGGREGATE).
#[utoipa::path(
    post,
    path = "/api/v1/search/indices/{index}/aggregate",
    params(
        ("index" = String, Path, description = "Index name")
    ),
    request_body = AggregateRequest,
    responses(
        (status = 200, description = "Aggregation results", body = AggregateResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Index not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn aggregate(
    State(state): State<AppState>,
    Path(index): Path<String>,
    Json(request): Json<AggregateRequest>,
) -> Result<Json<ApiResponse<AggregateResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let options = request.options.into();
    let result = state.search_service.ft_aggregate(&index, &request.query, options).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/search/indices/:index/explain
///
/// Get query execution plan (FT.EXPLAIN).
#[utoipa::path(
    post,
    path = "/api/v1/search/indices/{index}/explain",
    params(
        ("index" = String, Path, description = "Index name")
    ),
    request_body = ExplainRequest,
    responses(
        (status = 200, description = "Query execution plan", body = ExplainResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Index not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn explain(
    State(state): State<AppState>,
    Path(index): Path<String>,
    Json(request): Json<ExplainRequest>,
) -> Result<Json<ApiResponse<ExplainResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.search_service.ft_explain(&index, &request.query, request.dialect).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// POST /api/v1/search/indices/:index/profile
///
/// Profile a query (FT.PROFILE).
#[utoipa::path(
    post,
    path = "/api/v1/search/indices/{index}/profile",
    params(
        ("index" = String, Path, description = "Index name")
    ),
    request_body = ProfileRequest,
    responses(
        (status = 200, description = "Profile results", body = ProfileResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Index not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn profile(
    State(state): State<AppState>,
    Path(index): Path<String>,
    Json(request): Json<ProfileRequest>,
) -> Result<Json<ApiResponse<ProfileResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let profile_type = parse_profile_type(&request.profile_type).ok_or_else(|| CacheError::InvalidInput("Invalid profile type. Use SEARCH or AGGREGATE".to_string()))?;

    let search_opts = request.search_options.map(|o| o.into());
    let agg_opts = request.aggregate_options.map(|o| o.into());

    let result = state.search_service.ft_profile(&index, profile_type, request.limited, &request.query, search_opts, agg_opts).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

// ==================== Alias Operations ====================

/// POST /api/v1/search/aliases
///
/// Create an index alias (FT.ALIASADD).
#[utoipa::path(
    post,
    path = "/api/v1/search/aliases",
    request_body = AliasRequest,
    responses(
        (status = 200, description = "Alias created", body = AliasResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn alias_add(
    State(state): State<AppState>,
    Json(request): Json<AliasRequest>,
) -> Result<Json<ApiResponse<AliasResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.search_service.ft_aliasadd(&request.alias, &request.index).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// DELETE /api/v1/search/aliases/:alias
///
/// Delete an index alias (FT.ALIASDEL).
#[utoipa::path(
    delete,
    path = "/api/v1/search/aliases/{alias}",
    params(
        ("alias" = String, Path, description = "Alias name")
    ),
    responses(
        (status = 200, description = "Alias deleted", body = AliasResponse),
        (status = 404, description = "Alias not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn alias_del(
    State(state): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Json<ApiResponse<AliasResponse>>, CacheError> {
    let result = state.search_service.ft_aliasdel(&alias).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

/// PUT /api/v1/search/aliases/:alias
///
/// Update an index alias (FT.ALIASUPDATE).
#[utoipa::path(
    put,
    path = "/api/v1/search/aliases/{alias}",
    params(
        ("alias" = String, Path, description = "Alias name")
    ),
    request_body = AliasRequest,
    responses(
        (status = 200, description = "Alias updated", body = AliasResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Alias not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn alias_update(
    State(state): State<AppState>,
    Path(alias): Path<String>,
    Json(request): Json<AliasRequest>,
) -> Result<Json<ApiResponse<AliasResponse>>, CacheError> {
    let result = state
        .search_service
        .ft_aliasupdate(&alias, &request.index)
        .await?;
    Ok(Json(ApiResponse::new(result.into())))
}

// ==================== Autocomplete Operations ====================

/// POST /api/v1/search/suggest/:key
///
/// Add a suggestion to a dictionary (FT.SUGADD).
#[utoipa::path(
    post,
    path = "/api/v1/search/suggest/{key}",
    params(
        ("key" = String, Path, description = "Dictionary key")
    ),
    request_body = SugAddRequest,
    responses(
        (status = 200, description = "Suggestion added", body = SugAddResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn sug_add(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SugAddRequest>,
) -> Result<Json<ApiResponse<SugAddResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let options = SugAddOptions {
        incr: request.incr,
        payload: request.payload,
    };

    let result = state.search_service.ft_sugadd(&key, &request.string, request.score, options).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/search/suggest/:key
///
/// Get suggestions for a prefix (FT.SUGGET).
#[utoipa::path(
    get,
    path = "/api/v1/search/suggest/{key}",
    params(
        ("key" = String, Path, description = "Dictionary key"),
        ("prefix" = String, Query, description = "Prefix to search for"),
        ("fuzzy" = Option<bool>, Query, description = "Enable fuzzy matching"),
        ("withscores" = Option<bool>, Query, description = "Return scores"),
        ("withpayloads" = Option<bool>, Query, description = "Return payloads"),
        ("max" = Option<u32>, Query, description = "Maximum suggestions")
    ),
    responses(
        (status = 200, description = "Suggestions retrieved", body = SugGetResponse),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn sug_get(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<SugGetParams>,
) -> Result<Json<ApiResponse<SugGetResponse>>, CacheError> {
    let options: SugGetOptions = params.clone().into();
    let suggestions = state
        .search_service
        .ft_sugget(&key, &params.prefix, options)
        .await?;

    Ok(Json(ApiResponse::new(SugGetResponse {
        key,
        prefix: params.prefix,
        suggestions,
    })))
}

/// DELETE /api/v1/search/suggest/:key
///
/// Delete a suggestion from a dictionary (FT.SUGDEL).
#[utoipa::path(
    delete,
    path = "/api/v1/search/suggest/{key}",
    params(
        ("key" = String, Path, description = "Dictionary key")
    ),
    request_body = SugDelRequest,
    responses(
        (status = 200, description = "Suggestion deleted", body = SugDelResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn sug_del(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<SugDelRequest>,
) -> Result<Json<ApiResponse<SugDelResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.search_service.ft_sugdel(&key, &request.string).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/search/suggest/:key/len
///
/// Get dictionary size (FT.SUGLEN).
#[utoipa::path(
    get,
    path = "/api/v1/search/suggest/{key}/len",
    params(
        ("key" = String, Path, description = "Dictionary key")
    ),
    responses(
        (status = 200, description = "Dictionary size", body = SugLenResponse),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn sug_len(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<SugLenResponse>>, CacheError> {
    let result = state.search_service.ft_suglen(&key).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

// ==================== Synonym Operations ====================

/// GET /api/v1/search/indices/:index/synonyms
///
/// Dump all synonym groups (FT.SYNDUMP).
#[utoipa::path(
    get,
    path = "/api/v1/search/indices/{index}/synonyms",
    params(
        ("index" = String, Path, description = "Index name")
    ),
    responses(
        (status = 200, description = "Synonym groups", body = SynonymDumpResponse),
        (status = 404, description = "Index not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn syn_dump(
    State(state): State<AppState>,
    Path(index): Path<String>,
) -> Result<Json<ApiResponse<SynonymDumpResponse>>, CacheError> {
    let groups = state.search_service.ft_syndump(&index).await?;
    Ok(Json(ApiResponse::new(SynonymDumpResponse { index, groups })))
}

/// PUT /api/v1/search/indices/:index/synonyms/:group
///
/// Update a synonym group (FT.SYNUPDATE).
#[utoipa::path(
    put,
    path = "/api/v1/search/indices/{index}/synonyms/{group}",
    params(
        ("index" = String, Path, description = "Index name"),
        ("group" = String, Path, description = "Synonym group ID")
    ),
    request_body = SynonymUpdateRequest,
    responses(
        (status = 200, description = "Synonym group updated", body = SynonymUpdateResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Index not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn syn_update(
    State(state): State<AppState>,
    Path((index, group)): Path<(String, String)>,
    Json(request): Json<SynonymUpdateRequest>,
) -> Result<Json<ApiResponse<SynonymUpdateResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.search_service.ft_synupdate(&index, &group, request.skip_initial_scan, request.terms).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

// ==================== Spellcheck Operations ====================

/// POST /api/v1/search/indices/:index/spellcheck
///
/// Check spelling in query (FT.SPELLCHECK).
#[utoipa::path(
    post,
    path = "/api/v1/search/indices/{index}/spellcheck",
    params(
        ("index" = String, Path, description = "Index name")
    ),
    request_body = SpellcheckRequest,
    responses(
        (status = 200, description = "Spellcheck results", body = SpellcheckResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Index not found"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn spellcheck(
    State(state): State<AppState>,
    Path(index): Path<String>,
    Json(request): Json<SpellcheckRequest>,
) -> Result<Json<ApiResponse<SpellcheckResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let options = request.clone().into();
    let result = state.search_service.ft_spellcheck(&index, &request.query, options).await?;

    Ok(Json(ApiResponse::new(SpellcheckResponse {
        index,
        result,
    })))
}

// ==================== Dictionary Operations ====================

/// POST /api/v1/search/dicts/:dict/terms
///
/// Add terms to a dictionary (FT.DICTADD).
#[utoipa::path(
    post,
    path = "/api/v1/search/dicts/{dict}/terms",
    params(
        ("dict" = String, Path, description = "Dictionary name")
    ),
    request_body = DictTermsRequest,
    responses(
        (status = 200, description = "Terms added", body = DictResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn dict_add(
    State(state): State<AppState>,
    Path(dict): Path<String>,
    Json(request): Json<DictTermsRequest>,
) -> Result<Json<ApiResponse<DictResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.search_service.ft_dictadd(&dict, request.terms).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// DELETE /api/v1/search/dicts/:dict/terms
///
/// Delete terms from a dictionary (FT.DICTDEL).
#[utoipa::path(
    delete,
    path = "/api/v1/search/dicts/{dict}/terms",
    params(
        ("dict" = String, Path, description = "Dictionary name")
    ),
    request_body = DictTermsRequest,
    responses(
        (status = 200, description = "Terms deleted", body = DictResponse),
        (status = 400, description = "Invalid request"),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn dict_del(
    State(state): State<AppState>,
    Path(dict): Path<String>,
    Json(request): Json<DictTermsRequest>,
) -> Result<Json<ApiResponse<DictResponse>>, CacheError> {
    request
        .validate()
        .map_err(|e| CacheError::InvalidInput(e.to_string()))?;

    let result = state.search_service.ft_dictdel(&dict, request.terms).await?;

    Ok(Json(ApiResponse::new(result.into())))
}

/// GET /api/v1/search/dicts/:dict/terms
///
/// Dump all terms in a dictionary (FT.DICTDUMP).
#[utoipa::path(
    get,
    path = "/api/v1/search/dicts/{dict}/terms",
    params(
        ("dict" = String, Path, description = "Dictionary name")
    ),
    responses(
        (status = 200, description = "Dictionary terms", body = DictDumpResponse),
        (status = 501, description = "RediSearch module not available")
    ),
    tag = "Search"
)]
async fn dict_dump(
    State(state): State<AppState>,
    Path(dict): Path<String>,
) -> Result<Json<ApiResponse<DictDumpResponse>>, CacheError> {
    let result = state.search_service.ft_dictdump(&dict).await?;
    Ok(Json(ApiResponse::new(result.into())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use crate::test_support::test_state_with_search_repo;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_search_routes() {
        let (state, _) = test_state_with_search_repo();
        let app = search_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/indices")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"index":"idx","schema":[{"name":"title","field_type":"TEXT"}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/search/indices")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/search/indices/idx")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/search/indices/idx?dd=true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/indices/idx/fields")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"field":{"name":"title","field_type":"TEXT"}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/indices/idx/search")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"query":"*"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/indices/idx/aggregate")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"query":"*"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/indices/idx/explain")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"query":"*"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/indices/idx/profile")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"profile_type":"SEARCH","query":"*","search_options":{"withscores":true},"aggregate_options":{"verbatim":true}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/aliases")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"alias":"alias","index":"idx"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/search/aliases/alias")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/search/aliases/alias")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"alias":"alias","index":"idx"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/suggest/dict")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"string":"term","score":1.0,"incr":true,"payload":"p"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/search/suggest/dict?prefix=te&fuzzy=true&withscores=true&withpayloads=true&max=5")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/search/suggest/dict")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"string":"term"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/search/suggest/dict/len")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/search/indices/idx/synonyms")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/search/indices/idx/synonyms/1")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"group_id":"1","terms":["t1"],"skip_initial_scan":true}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/indices/idx/spellcheck")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"query":"hello","distance":1,"include":"dict","exclude":"other","dialect":2}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/dicts/dict/terms")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"terms":["a","b"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/search/dicts/dict/terms")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"terms":["a"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/search/dicts/dict/terms")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_search_create_index_invalid_field_type() {
        let (state, _) = test_state_with_search_repo();
        let app = search_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/indices")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"index":"idx","schema":[{"name":"title","field_type":"INVALID"}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_search_alter_index_invalid_field_type() {
        let (state, _) = test_state_with_search_repo();
        let app = search_routes().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search/indices/idx/fields")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"field":{"name":"title","field_type":"INVALID"}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
