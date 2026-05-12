//! Search Schemas
//!
//! Request/response schemas for RediSearch operations.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domain::entities::{
    AggregateOptions, AggregateResult, AggregateStep, AliasResult, DictDumpResult, DictResult,
    ExplainResult, GeoFilter, HighlightOptions, IndexAlterResult, IndexCreateOptions,
    IndexCreateResult, IndexDataType, IndexDropResult, IndexInfo, NumericFilter, ProfileResult,
    ProfileType, SearchFieldSchema, SearchFieldType, SearchOptions, SearchResult, SortBy,
    SpellcheckOptions, SpellcheckResult, SugAddResult, SugDelResult, SugGetOptions, SugLenResult,
    Suggestion, SummarizeOptions, SynonymGroup, SynonymUpdateResult, VectorFieldOptions,
};

// ==================== Index Operations ====================

/// Request to create a search index
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateIndexRequest {
    /// Index name
    #[validate(length(min = 1, max = 128, message = "Index name must be 1-128 characters"))]
    pub index: String,

    /// Index options
    #[serde(default)]
    pub options: IndexCreateOptionsDto,

    /// Field schema definitions
    #[validate(length(min = 1, message = "At least one field is required"))]
    pub schema: Vec<SearchFieldSchemaDto>,
}

/// Index creation options DTO
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct IndexCreateOptionsDto {
    /// Data type to index (HASH or JSON)
    #[serde(default)]
    pub on: Option<String>,

    /// Key prefixes to index
    #[serde(default)]
    pub prefixes: Vec<String>,

    /// Filter expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,

    /// Default language for text fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Field containing document language
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_field: Option<String>,

    /// Default score for documents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    /// Field containing document score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_field: Option<String>,

    /// Payload field name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_field: Option<String>,

    /// Enable MAXTEXTFIELDS
    #[serde(default)]
    pub maxtextfields: Option<bool>,

    /// Don't save term offsets
    #[serde(default)]
    pub no_offsets: bool,

    /// Temporary index (TTL in seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary: Option<u64>,

    /// Don't store field bits
    #[serde(default)]
    pub no_fields: bool,

    /// Don't store term frequencies
    #[serde(default)]
    pub no_freqs: bool,

    /// Don't highlight results
    #[serde(default)]
    pub no_hl: bool,

    /// Skip initial scan
    #[serde(default)]
    pub skip_initial_scan: bool,

    /// Stop words list
    #[serde(default)]
    pub stopwords: Vec<String>,
}

impl From<IndexCreateOptionsDto> for IndexCreateOptions {
    fn from(dto: IndexCreateOptionsDto) -> Self {
        IndexCreateOptions {
            on: dto
                .on
                .map(|s| match s.to_uppercase().as_str() {
                    "JSON" => IndexDataType::Json,
                    _ => IndexDataType::Hash,
                })
                .unwrap_or_default(),
            prefixes: dto.prefixes,
            filter: dto.filter,
            language: dto.language,
            language_field: dto.language_field,
            score: dto.score,
            score_field: dto.score_field,
            payload_field: dto.payload_field,
            maxtextfields: dto.maxtextfields,
            no_offsets: dto.no_offsets,
            temporary: dto.temporary,
            no_fields: dto.no_fields,
            no_freqs: dto.no_freqs,
            no_hl: dto.no_hl,
            skip_initial_scan: dto.skip_initial_scan,
            stopwords: dto.stopwords,
        }
    }
}

/// Field schema DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchFieldSchemaDto {
    /// Field name
    pub name: String,

    /// Alias for the field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,

    /// Field type (TEXT, TAG, NUMERIC, GEO, VECTOR, GEOSHAPE)
    pub field_type: String,

    /// Whether field is sortable
    #[serde(default)]
    pub sortable: bool,

    /// UNF - keep original value for sorting
    #[serde(default)]
    pub unf: bool,

    /// Skip indexing this field
    #[serde(default)]
    pub no_index: bool,

    /// Text field weight
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,

    /// Disable stemming
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_stem: Option<bool>,

    /// Phonetic matcher
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phonetic: Option<String>,

    /// Tag separator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,

    /// Case sensitive tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,

    /// Index empty values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_empty: Option<bool>,

    /// Vector field options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_options: Option<VectorFieldOptions>,
}

/// Error for invalid field type
#[derive(Debug, Clone)]
pub struct InvalidFieldTypeError(pub String);

impl std::fmt::Display for InvalidFieldTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid field type '{}'. Valid types are: TEXT, TAG, NUMERIC, GEO, VECTOR, GEOSHAPE",
            self.0
        )
    }
}

impl std::error::Error for InvalidFieldTypeError {}

impl TryFrom<SearchFieldSchemaDto> for SearchFieldSchema {
    type Error = InvalidFieldTypeError;

    fn try_from(dto: SearchFieldSchemaDto) -> Result<Self, Self::Error> {
        let field_type = match dto.field_type.to_uppercase().as_str() {
            "TEXT" => SearchFieldType::Text,
            "TAG" => SearchFieldType::Tag,
            "NUMERIC" => SearchFieldType::Numeric,
            "GEO" => SearchFieldType::Geo,
            "VECTOR" => SearchFieldType::Vector,
            "GEOSHAPE" => SearchFieldType::Geoshape,
            _ => return Err(InvalidFieldTypeError(dto.field_type)),
        };

        Ok(SearchFieldSchema {
            name: dto.name,
            alias: dto.alias,
            field_type,
            sortable: dto.sortable,
            unf: dto.unf,
            no_index: dto.no_index,
            weight: dto.weight,
            no_stem: dto.no_stem,
            phonetic: dto.phonetic.and_then(|p| {
                use crate::domain::entities::PhoneticMatcher;
                match p.as_str() {
                    "dm:en" => Some(PhoneticMatcher::DmEn),
                    "dm:fr" => Some(PhoneticMatcher::DmFr),
                    "dm:pt" => Some(PhoneticMatcher::DmPt),
                    "dm:es" => Some(PhoneticMatcher::DmEs),
                    _ => None,
                }
            }),
            separator: dto.separator,
            case_sensitive: dto.case_sensitive,
            index_empty: dto.index_empty,
            vector_options: dto.vector_options,
            missing_field_policy: None,
        })
    }
}

/// Response for index creation
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateIndexResponse {
    /// Index name
    pub index: String,
    /// Whether creation succeeded
    pub success: bool,
}

impl From<IndexCreateResult> for CreateIndexResponse {
    fn from(result: IndexCreateResult) -> Self {
        CreateIndexResponse {
            index: result.index,
            success: result.success,
        }
    }
}

/// Query parameters for dropping an index
#[derive(Debug, Deserialize, ToSchema)]
pub struct DropIndexParams {
    /// Delete indexed documents
    #[serde(default)]
    pub dd: bool,
}

/// Response for dropping an index
#[derive(Debug, Serialize, ToSchema)]
pub struct DropIndexResponse {
    /// Index name
    pub index: String,
    /// Whether documents were deleted
    pub delete_docs: bool,
    /// Whether operation succeeded
    pub success: bool,
}

impl From<IndexDropResult> for DropIndexResponse {
    fn from(result: IndexDropResult) -> Self {
        DropIndexResponse {
            index: result.index,
            delete_docs: result.delete_docs,
            success: result.success,
        }
    }
}

/// Response for listing indices
#[derive(Debug, Serialize, ToSchema)]
pub struct ListIndicesResponse {
    /// List of index names
    pub indices: Vec<String>,
}

/// Response for index info
#[derive(Debug, Serialize, ToSchema)]
pub struct IndexInfoResponse {
    /// Index information
    #[serde(flatten)]
    pub info: IndexInfo,
}

/// Request to add a field to an index
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AlterIndexRequest {
    /// Field schema to add
    pub field: SearchFieldSchemaDto,
}

/// Response for altering an index
#[derive(Debug, Serialize, ToSchema)]
pub struct AlterIndexResponse {
    /// Index name
    pub index: String,
    /// Field added
    pub field: String,
    /// Whether operation succeeded
    pub success: bool,
}

impl From<IndexAlterResult> for AlterIndexResponse {
    fn from(result: IndexAlterResult) -> Self {
        AlterIndexResponse {
            index: result.index,
            field: result.field,
            success: result.success,
        }
    }
}

// ==================== Search Operations ====================

/// Request for search query
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SearchRequest {
    /// Search query string
    #[validate(length(min = 1, message = "Query cannot be empty"))]
    pub query: String,

    /// Search options
    #[serde(flatten, default)]
    pub options: SearchOptionsDto,
}

/// Search options DTO
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct SearchOptionsDto {
    /// Don't return document content
    #[serde(default)]
    pub nocontent: bool,

    /// Verbatim - don't expand query
    #[serde(default)]
    pub verbatim: bool,

    /// Don't use stopwords
    #[serde(default)]
    pub nostopwords: bool,

    /// Return scores with results
    #[serde(default)]
    pub withscores: bool,

    /// Return payloads with results
    #[serde(default)]
    pub withpayloads: bool,

    /// Return sort keys with results
    #[serde(default)]
    pub withsortkeys: bool,

    /// Numeric filters
    #[serde(default)]
    pub filters: Vec<NumericFilter>,

    /// Geographic filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geofilter: Option<GeoFilter>,

    /// In-keys filter
    #[serde(default)]
    pub inkeys: Vec<String>,

    /// In-fields filter
    #[serde(default)]
    pub infields: Vec<String>,

    /// Return specific fields
    #[serde(default)]
    pub return_fields: Vec<String>,

    /// Summarize options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summarize: Option<SummarizeOptions>,

    /// Highlight options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<HighlightOptions>,

    /// Slop for phrase queries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slop: Option<u32>,

    /// Timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,

    /// In order flag for slop
    #[serde(default)]
    pub inorder: bool,

    /// Language for stemming
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Custom scorer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scorer: Option<String>,

    /// Explain score
    #[serde(default)]
    pub explainscore: bool,

    /// Use Reciprocal Rank Fusion (hybrid search)
    #[serde(default)]
    pub rrf: bool,

    /// Sort by field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sortby: Option<SortBy>,

    /// Limit offset
    #[serde(default)]
    pub offset: u64,

    /// Limit count (default 10)
    #[serde(default = "default_limit")]
    pub limit: u64,

    /// Parameters for parameterized queries
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,

    /// Dialect version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialect: Option<u32>,
}

fn default_limit() -> u64 {
    10
}

impl From<SearchOptionsDto> for SearchOptions {
    fn from(dto: SearchOptionsDto) -> Self {
        SearchOptions {
            nocontent: dto.nocontent,
            verbatim: dto.verbatim,
            nostopwords: dto.nostopwords,
            withscores: dto.withscores,
            withpayloads: dto.withpayloads,
            withsortkeys: dto.withsortkeys,
            filters: dto.filters,
            geofilter: dto.geofilter,
            inkeys: dto.inkeys,
            infields: dto.infields,
            return_fields: dto.return_fields,
            summarize: dto.summarize,
            highlight: dto.highlight,
            slop: dto.slop,
            timeout: dto.timeout,
            inorder: dto.inorder,
            language: dto.language,
            scorer: dto.scorer,
            explainscore: dto.explainscore,
            rrf: dto.rrf,
            sortby: dto.sortby,
            offset: dto.offset,
            limit: dto.limit,
            params: dto.params,
            dialect: dto.dialect,
        }
    }
}

/// Response for search query
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    /// Total results
    pub total_results: u64,
    /// Documents
    pub documents: Vec<serde_json::Value>,
}

impl From<SearchResult> for SearchResponse {
    fn from(result: SearchResult) -> Self {
        SearchResponse {
            total_results: result.total_results,
            documents: result
                .documents
                .into_iter()
                .map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null))
                .collect(),
        }
    }
}

// ==================== Aggregate Operations ====================

/// Request for aggregate query
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AggregateRequest {
    /// Aggregate query string
    #[validate(length(min = 1, message = "Query cannot be empty"))]
    pub query: String,

    /// Aggregate options
    #[serde(flatten, default)]
    pub options: AggregateOptionsDto,
}

/// Aggregate options DTO
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct AggregateOptionsDto {
    /// Verbatim - don't expand query
    #[serde(default)]
    pub verbatim: bool,

    /// Load fields from documents
    #[serde(default)]
    pub load: Vec<String>,

    /// Load all fields
    #[serde(default)]
    pub load_all: bool,

    /// Timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,

    /// Aggregation pipeline steps
    #[serde(default)]
    pub pipeline: Vec<AggregateStep>,

    /// Parameters for parameterized queries
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,

    /// Dialect version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialect: Option<u32>,

    /// Use cursor for result pagination
    #[serde(default)]
    pub withcursor: bool,

    /// Count for cursor pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor_count: Option<u64>,
}

impl From<AggregateOptionsDto> for AggregateOptions {
    fn from(dto: AggregateOptionsDto) -> Self {
        AggregateOptions {
            verbatim: dto.verbatim,
            load: dto.load,
            load_all: dto.load_all,
            timeout: dto.timeout,
            pipeline: dto.pipeline,
            params: dto.params,
            dialect: dto.dialect,
            withcursor: dto.withcursor,
            cursor_count: dto.cursor_count,
        }
    }
}

/// Response for aggregate query
#[derive(Debug, Serialize, ToSchema)]
pub struct AggregateResponse {
    /// Total results
    pub total_results: u64,
    /// Result rows
    pub rows: Vec<serde_json::Value>,
    /// Cursor ID (if WITHCURSOR was used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor_id: Option<u64>,
}

impl From<AggregateResult> for AggregateResponse {
    fn from(result: AggregateResult) -> Self {
        AggregateResponse {
            total_results: result.total_results,
            cursor_id: result.cursor_id,
            rows: result
                .rows
                .into_iter()
                .map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null))
                .collect(),
        }
    }
}

// ==================== Hybrid Search Operations ====================

/// Request for hybrid text+vector search (FT.HYBRID, Redis 8.4+)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct HybridSearchRequest {
    /// Text search query for the SEARCH clause
    #[validate(length(min = 1, message = "Query cannot be empty"))]
    pub query: String,

    /// Optional scorer for the SEARCH clause
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_scorer: Option<String>,

    /// Optional YIELD_SCORE_AS name for text search score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_yield_score_as: Option<String>,

    /// Vector field name for the VSIM clause
    #[validate(length(min = 1, message = "VSIM field name cannot be empty"))]
    pub vsim_field: String,

    /// Vector similarity input (ELE or VALUES) — required
    pub vsim_input: crate::domain::entities::VsimInput,

    /// Optional YIELD_SCORE_AS name for vector score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vsim_yield_score_as: Option<String>,

    /// Fields to LOAD from documents
    #[serde(default)]
    pub load: Vec<String>,

    /// APPLY expressions
    #[serde(default)]
    pub apply: Vec<crate::domain::entities::ApplyStep>,

    /// SORTBY specifications
    #[serde(default)]
    pub sortby: Vec<SortBy>,

    /// LIMIT offset
    #[serde(default)]
    pub offset: u64,

    /// LIMIT count (default 10)
    #[serde(default = "default_limit")]
    pub limit: u64,

    /// Parameters for parameterized queries
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,

    /// FILTER expressions
    #[serde(default)]
    pub filters: Vec<String>,

    /// Combination strategy (RRF or LINEAR with parameters)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub combine: Option<crate::domain::entities::CombineStrategy>,

    /// Execution policy: ADHOC or BATCHES
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,

    /// Batch size (when policy is BATCHES)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u32>,
}

impl From<HybridSearchRequest> for crate::domain::entities::HybridSearchOptions {
    fn from(req: HybridSearchRequest) -> Self {
        crate::domain::entities::HybridSearchOptions {
            query: req.query,
            search_scorer: req.search_scorer,
            search_yield_score_as: req.search_yield_score_as,
            vsim_field: req.vsim_field,
            vsim_input: req.vsim_input,
            vsim_yield_score_as: req.vsim_yield_score_as,
            load: req.load,
            apply: req.apply,
            sortby: req.sortby,
            offset: req.offset,
            limit: req.limit,
            params: req.params,
            filters: req.filters,
            combine: req.combine,
            policy: req.policy,
            batch_size: req.batch_size,
        }
    }
}

/// Response for hybrid search query
#[derive(Debug, Serialize, ToSchema)]
pub struct HybridSearchResponse {
    /// Total results
    pub total_results: u64,
    /// Documents
    pub documents: Vec<serde_json::Value>,
}

impl From<crate::domain::entities::HybridSearchResult> for HybridSearchResponse {
    fn from(result: crate::domain::entities::HybridSearchResult) -> Self {
        HybridSearchResponse {
            total_results: result.total_results,
            documents: result
                .documents
                .into_iter()
                .map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null))
                .collect(),
        }
    }
}

// ==================== Explain Operations ====================

/// Request for explain query
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ExplainRequest {
    /// Query to explain
    #[validate(length(min = 1, message = "Query cannot be empty"))]
    pub query: String,

    /// Dialect version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialect: Option<u32>,
}

/// Response for explain query
#[derive(Debug, Serialize, ToSchema)]
pub struct ExplainResponse {
    /// Query execution plan
    pub plan: String,
}

impl From<ExplainResult> for ExplainResponse {
    fn from(result: ExplainResult) -> Self {
        ExplainResponse { plan: result.plan }
    }
}

// ==================== Profile Operations ====================

/// Request for profile query
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ProfileRequest {
    /// Profile type (SEARCH or AGGREGATE)
    pub profile_type: String,

    /// Query to profile
    #[validate(length(min = 1, message = "Query cannot be empty"))]
    pub query: String,

    /// Whether to limit profile output
    #[serde(default)]
    pub limited: bool,

    /// Search options (when profile_type is SEARCH)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_options: Option<SearchOptionsDto>,

    /// Aggregate options (when profile_type is AGGREGATE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregate_options: Option<AggregateOptionsDto>,
}

/// Response for profile query
#[derive(Debug, Serialize, ToSchema)]
pub struct ProfileResponse {
    /// Query results
    pub results: serde_json::Value,
    /// Profile information
    pub profile: serde_json::Value,
}

impl From<ProfileResult> for ProfileResponse {
    fn from(result: ProfileResult) -> Self {
        ProfileResponse {
            results: result.results,
            profile: serde_json::to_value(result.profile).unwrap_or(serde_json::Value::Null),
        }
    }
}

// ==================== Alias Operations ====================

/// Request to create/update an alias
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AliasRequest {
    /// Alias name
    #[validate(length(min = 1, max = 128, message = "Alias name must be 1-128 characters"))]
    pub alias: String,

    /// Target index name
    #[validate(length(min = 1, max = 128, message = "Index name must be 1-128 characters"))]
    pub index: String,
}

/// Response for alias operations
#[derive(Debug, Serialize, ToSchema)]
pub struct AliasResponse {
    /// Alias name
    pub alias: String,
    /// Target index
    pub index: String,
    /// Whether operation succeeded
    pub success: bool,
}

impl From<AliasResult> for AliasResponse {
    fn from(result: AliasResult) -> Self {
        AliasResponse {
            alias: result.alias,
            index: result.index,
            success: result.success,
        }
    }
}

// ==================== Autocomplete Operations ====================

/// Request to add a suggestion
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SugAddRequest {
    /// Suggestion string
    #[validate(length(min = 1, message = "Suggestion string cannot be empty"))]
    pub string: String,

    /// Suggestion score
    pub score: f64,

    /// Increment existing score
    #[serde(default)]
    pub incr: bool,

    /// Optional payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
}

/// Response for adding a suggestion
#[derive(Debug, Serialize, ToSchema)]
pub struct SugAddResponse {
    /// Dictionary key
    pub key: String,
    /// Dictionary size
    pub size: i64,
}

impl From<SugAddResult> for SugAddResponse {
    fn from(result: SugAddResult) -> Self {
        SugAddResponse {
            key: result.key,
            size: result.size,
        }
    }
}

/// Query parameters for getting suggestions
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SugGetParams {
    /// Prefix to search for
    pub prefix: String,

    /// Enable fuzzy matching
    #[serde(default)]
    pub fuzzy: bool,

    /// Return scores
    #[serde(default)]
    pub withscores: bool,

    /// Return payloads
    #[serde(default)]
    pub withpayloads: bool,

    /// Maximum suggestions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
}

impl From<SugGetParams> for SugGetOptions {
    fn from(params: SugGetParams) -> Self {
        SugGetOptions {
            fuzzy: params.fuzzy,
            withscores: params.withscores,
            withpayloads: params.withpayloads,
            max: params.max,
        }
    }
}

/// Response for getting suggestions
#[derive(Debug, Serialize, ToSchema)]
pub struct SugGetResponse {
    /// Key
    pub key: String,
    /// Prefix searched
    pub prefix: String,
    /// Suggestions
    pub suggestions: Vec<Suggestion>,
}

/// Request to delete a suggestion
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SugDelRequest {
    /// Suggestion string to delete
    #[validate(length(min = 1, message = "Suggestion string cannot be empty"))]
    pub string: String,
}

/// Response for deleting a suggestion
#[derive(Debug, Serialize, ToSchema)]
pub struct SugDelResponse {
    /// Dictionary key
    pub key: String,
    /// Whether deletion succeeded
    pub deleted: bool,
}

impl From<SugDelResult> for SugDelResponse {
    fn from(result: SugDelResult) -> Self {
        SugDelResponse {
            key: result.key,
            deleted: result.deleted,
        }
    }
}

/// Response for dictionary length
#[derive(Debug, Serialize, ToSchema)]
pub struct SugLenResponse {
    /// Dictionary key
    pub key: String,
    /// Dictionary size
    pub size: i64,
}

impl From<SugLenResult> for SugLenResponse {
    fn from(result: SugLenResult) -> Self {
        SugLenResponse {
            key: result.key,
            size: result.size,
        }
    }
}

// ==================== Synonym Operations ====================

/// Response for synonym dump
#[derive(Debug, Serialize, ToSchema)]
pub struct SynonymDumpResponse {
    /// Index name
    pub index: String,
    /// Synonym groups
    pub groups: Vec<SynonymGroup>,
}

/// Request to update synonyms
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SynonymUpdateRequest {
    /// Synonym group ID
    #[validate(length(min = 1, message = "Group ID cannot be empty"))]
    pub group_id: String,

    /// Terms in the group
    #[validate(length(min = 1, message = "At least one term is required"))]
    pub terms: Vec<String>,

    /// Skip initial scan
    #[serde(default)]
    pub skip_initial_scan: bool,
}

/// Response for synonym update
#[derive(Debug, Serialize, ToSchema)]
pub struct SynonymUpdateResponse {
    /// Index name
    pub index: String,
    /// Group ID
    pub group_id: String,
    /// Whether operation succeeded
    pub success: bool,
}

impl From<SynonymUpdateResult> for SynonymUpdateResponse {
    fn from(result: SynonymUpdateResult) -> Self {
        SynonymUpdateResponse {
            index: result.index,
            group_id: result.group_id,
            success: result.success,
        }
    }
}

// ==================== Spellcheck Operations ====================

/// Request for spellcheck
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct SpellcheckRequest {
    /// Query to check
    #[validate(length(min = 1, message = "Query cannot be empty"))]
    pub query: String,

    /// Maximum edit distance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<u32>,

    /// Include dictionary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<String>,

    /// Exclude dictionary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<String>,

    /// Dialect version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialect: Option<u32>,
}

impl From<SpellcheckRequest> for SpellcheckOptions {
    fn from(req: SpellcheckRequest) -> Self {
        SpellcheckOptions {
            distance: req.distance,
            include: req.include,
            exclude: req.exclude,
            dialect: req.dialect,
        }
    }
}

/// Response for spellcheck
#[derive(Debug, Serialize, ToSchema)]
pub struct SpellcheckResponse {
    /// Index name
    pub index: String,
    /// Spellcheck results
    #[serde(flatten)]
    pub result: SpellcheckResult,
}

// ==================== Dictionary Operations ====================

/// Request to add/delete dictionary terms
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct DictTermsRequest {
    /// Terms to add/delete
    #[validate(length(min = 1, message = "At least one term is required"))]
    pub terms: Vec<String>,
}

/// Response for dictionary add/delete
#[derive(Debug, Serialize, ToSchema)]
pub struct DictResponse {
    /// Dictionary name
    pub dict: String,
    /// Number of terms affected
    pub count: i64,
}

impl From<DictResult> for DictResponse {
    fn from(result: DictResult) -> Self {
        DictResponse {
            dict: result.dict,
            count: result.count,
        }
    }
}

/// Response for dictionary dump
#[derive(Debug, Serialize, ToSchema)]
pub struct DictDumpResponse {
    /// Dictionary name
    pub dict: String,
    /// Terms in dictionary
    pub terms: Vec<String>,
}

impl From<DictDumpResult> for DictDumpResponse {
    fn from(result: DictDumpResult) -> Self {
        DictDumpResponse {
            dict: result.dict,
            terms: result.terms,
        }
    }
}

// ==================== Configuration Operations ====================

/// Request to set a configuration option
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SearchConfigSetRequest {
    /// Option value
    #[validate(length(min = 1, message = "Value cannot be empty"))]
    pub value: String,
}

/// Response for configuration get
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchConfigGetResponse {
    /// Configuration key-value pairs
    pub config: std::collections::HashMap<String, String>,
}

/// Response for configuration set
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchConfigSetResponse {
    /// Option name
    pub option: String,
    /// Whether operation succeeded
    pub success: bool,
}

// ==================== Cursor Operations ====================

/// Query parameters for cursor read
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CursorReadParams {
    /// Maximum count to read
    pub count: Option<u64>,
}

/// Response for cursor read
#[derive(Debug, Serialize, ToSchema)]
pub struct CursorReadResponse {
    /// Next cursor ID (0 if exhausted)
    pub cursor_id: u64,
    /// Result rows
    pub rows: Vec<serde_json::Value>,
}

impl From<crate::domain::entities::CursorReadResult> for CursorReadResponse {
    fn from(result: crate::domain::entities::CursorReadResult) -> Self {
        CursorReadResponse {
            cursor_id: result.cursor_id,
            rows: result
                .rows
                .into_iter()
                .map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null))
                .collect(),
        }
    }
}

/// Response for cursor delete
#[derive(Debug, Serialize, ToSchema)]
pub struct CursorDelResponse {
    /// Whether deletion succeeded
    pub success: bool,
}

// ==================== Helper Functions ====================

/// Parse profile type from string
pub fn parse_profile_type(s: &str) -> Option<ProfileType> {
    match s.to_uppercase().as_str() {
        "SEARCH" => Some(ProfileType::Search),
        "AGGREGATE" => Some(ProfileType::Aggregate),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{PhoneticMatcher, SearchDocument};
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_index_create_options_conversion() {
        let dto = IndexCreateOptionsDto {
            on: Some("JSON".to_string()),
            prefixes: vec!["user:".to_string()],
            ..Default::default()
        };

        let options: IndexCreateOptions = dto.into();
        assert!(matches!(options.on, IndexDataType::Json));
        assert_eq!(options.prefixes, vec!["user:"]);
    }

    #[test]
    fn test_index_create_options_defaults_unknown_on_to_hash() {
        let options: IndexCreateOptions = IndexCreateOptionsDto {
            on: Some("hash".to_string()),
            ..Default::default()
        }
        .into();
        assert!(matches!(options.on, IndexDataType::Hash));

        let options: IndexCreateOptions = IndexCreateOptionsDto {
            on: Some("unknown".to_string()),
            ..Default::default()
        }
        .into();
        assert!(matches!(options.on, IndexDataType::Hash));
    }

    #[test]
    fn test_field_schema_conversion() {
        let dto = SearchFieldSchemaDto {
            name: "title".to_string(),
            alias: None,
            field_type: "TEXT".to_string(),
            sortable: true,
            unf: false,
            no_index: false,
            weight: Some(2.0),
            no_stem: None,
            phonetic: None,
            separator: None,
            case_sensitive: None,
            index_empty: None,
            vector_options: None,
        };

        let schema: SearchFieldSchema = dto.try_into().unwrap();
        assert_eq!(schema.name, "title");
        assert!(matches!(schema.field_type, SearchFieldType::Text));
        assert!(schema.sortable);
        assert_eq!(schema.weight, Some(2.0));
    }

    #[test]
    fn test_search_options_conversion() {
        let dto = SearchOptionsDto {
            withscores: true,
            limit: 20,
            offset: 10,
            ..Default::default()
        };

        let options: SearchOptions = dto.into();
        assert!(options.withscores);
        assert_eq!(options.limit, 20);
        assert_eq!(options.offset, 10);
    }

    #[test]
    fn test_parse_profile_type() {
        assert!(matches!(
            parse_profile_type("SEARCH"),
            Some(ProfileType::Search)
        ));
        assert!(matches!(
            parse_profile_type("search"),
            Some(ProfileType::Search)
        ));
        assert!(matches!(
            parse_profile_type("AGGREGATE"),
            Some(ProfileType::Aggregate)
        ));
        assert!(parse_profile_type("invalid").is_none());
    }

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 10);
    }

    #[test]
    fn test_field_schema_conversion_variants() {
        let valid_variants = vec![
            ("TEXT", SearchFieldType::Text),
            ("TAG", SearchFieldType::Tag),
            ("NUMERIC", SearchFieldType::Numeric),
            ("GEO", SearchFieldType::Geo),
            ("VECTOR", SearchFieldType::Vector),
            ("GEOSHAPE", SearchFieldType::Geoshape),
        ];

        for (field_type, expected) in valid_variants {
            let dto = SearchFieldSchemaDto {
                name: "field".to_string(),
                alias: None,
                field_type: field_type.to_string(),
                sortable: false,
                unf: false,
                no_index: false,
                weight: None,
                no_stem: None,
                phonetic: None,
                separator: None,
                case_sensitive: None,
                index_empty: None,
                vector_options: None,
            };
            let schema: Result<SearchFieldSchema, _> = dto.try_into();
            assert!(schema.is_ok());
            assert_eq!(schema.unwrap().field_type, expected);
        }

        // Test invalid field type returns error
        let invalid_dto = SearchFieldSchemaDto {
            name: "field".to_string(),
            alias: None,
            field_type: "INVALID".to_string(),
            sortable: false,
            unf: false,
            no_index: false,
            weight: None,
            no_stem: None,
            phonetic: None,
            separator: None,
            case_sensitive: None,
            index_empty: None,
            vector_options: None,
        };
        let result: Result<SearchFieldSchema, InvalidFieldTypeError> = invalid_dto.try_into();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID"));

        let phonetics = vec![
            ("dm:en", Some(PhoneticMatcher::DmEn)),
            ("dm:fr", Some(PhoneticMatcher::DmFr)),
            ("dm:pt", Some(PhoneticMatcher::DmPt)),
            ("dm:es", Some(PhoneticMatcher::DmEs)),
            ("unknown", None),
        ];

        for (phonetic, expected) in phonetics {
            let dto = SearchFieldSchemaDto {
                name: "field".to_string(),
                alias: None,
                field_type: "TEXT".to_string(),
                sortable: false,
                unf: false,
                no_index: false,
                weight: None,
                no_stem: None,
                phonetic: Some(phonetic.to_string()),
                separator: None,
                case_sensitive: None,
                index_empty: None,
                vector_options: None,
            };
            let schema: SearchFieldSchema = dto.try_into().unwrap();
            assert_eq!(schema.phonetic, expected);
        }
    }

    #[test]
    fn test_response_conversions() {
        let create = CreateIndexResponse::from(IndexCreateResult {
            index: "idx".to_string(),
            success: true,
        });
        assert_eq!(create.index, "idx");
        assert!(create.success);

        let drop = DropIndexResponse::from(IndexDropResult {
            index: "idx".to_string(),
            delete_docs: true,
            success: true,
        });
        assert!(drop.delete_docs);
        assert!(drop.success);

        let alter = AlterIndexResponse::from(IndexAlterResult {
            index: "idx".to_string(),
            field: "title".to_string(),
            success: true,
        });
        assert_eq!(alter.field, "title");

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), json!("doc"));
        let document = SearchDocument {
            id: "doc1".to_string(),
            score: Some(1.0),
            payload: Some("payload".to_string()),
            sortkey: None,
            fields,
            score_explanation: None,
        };
        let search = SearchResponse::from(SearchResult {
            total_results: 1,
            documents: vec![document],
        });
        assert_eq!(search.total_results, 1);
        assert_eq!(search.documents.len(), 1);

        let mut row = HashMap::new();
        row.insert("count".to_string(), json!(1));
        let aggregate = AggregateResponse::from(AggregateResult {
            total_results: 1,
            rows: vec![row],
            cursor_id: None,
        });
        assert_eq!(aggregate.rows.len(), 1);

        let explain = ExplainResponse::from(ExplainResult {
            plan: "INTERSECT".to_string(),
        });
        assert_eq!(explain.plan, "INTERSECT");

        let mut profile = HashMap::new();
        profile.insert("time".to_string(), json!(1));
        let profile_response = ProfileResponse::from(ProfileResult {
            results: json!({"ok": true}),
            profile,
        });
        assert!(profile_response.results.is_object());

        let alias = AliasResponse::from(AliasResult {
            alias: "alias".to_string(),
            index: "idx".to_string(),
            success: true,
        });
        assert!(alias.success);

        let sug_add = SugAddResponse::from(SugAddResult {
            key: "dict".to_string(),
            size: 2,
        });
        assert_eq!(sug_add.size, 2);

        let sug_del = SugDelResponse::from(SugDelResult {
            key: "dict".to_string(),
            deleted: true,
        });
        assert!(sug_del.deleted);

        let sug_len = SugLenResponse::from(SugLenResult {
            key: "dict".to_string(),
            size: 3,
        });
        assert_eq!(sug_len.size, 3);

        let syn_update = SynonymUpdateResponse::from(SynonymUpdateResult {
            index: "idx".to_string(),
            group_id: "1".to_string(),
            success: true,
        });
        assert!(syn_update.success);

        let spellcheck: SpellcheckOptions = SpellcheckRequest {
            query: "hello".to_string(),
            distance: Some(1),
            include: Some("dict".to_string()),
            exclude: None,
            dialect: Some(2),
        }
        .into();
        assert_eq!(spellcheck.distance, Some(1));

        let dict = DictResponse::from(DictResult {
            dict: "dict".to_string(),
            count: 2,
        });
        assert_eq!(dict.count, 2);

        let dict_dump = DictDumpResponse::from(DictDumpResult {
            dict: "dict".to_string(),
            terms: vec!["a".to_string()],
        });
        assert_eq!(dict_dump.terms.len(), 1);
    }

    #[test]
    fn test_option_conversions() {
        let options: SugGetOptions = SugGetParams {
            prefix: "pre".to_string(),
            fuzzy: true,
            withscores: true,
            withpayloads: true,
            max: Some(3),
        }
        .into();
        assert!(options.fuzzy);
        assert_eq!(options.max, Some(3));
    }
}
