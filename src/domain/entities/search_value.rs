//! Search Value Entity
//!
//! Domain entities for RediSearch operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

// ==================== Index Schema Types ====================

/// Field type for search index schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum SearchFieldType {
    /// Full-text searchable field
    Text,
    /// Exact match tag field (comma-separated values)
    Tag,
    /// Numeric field for range queries
    Numeric,
    /// Geographic coordinates (lon,lat)
    Geo,
    /// Vector field for similarity search
    Vector,
    /// Geoshape field for polygon queries
    Geoshape,
}

impl std::fmt::Display for SearchFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchFieldType::Text => write!(f, "TEXT"),
            SearchFieldType::Tag => write!(f, "TAG"),
            SearchFieldType::Numeric => write!(f, "NUMERIC"),
            SearchFieldType::Geo => write!(f, "GEO"),
            SearchFieldType::Vector => write!(f, "VECTOR"),
            SearchFieldType::Geoshape => write!(f, "GEOSHAPE"),
        }
    }
}

/// Vector similarity algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum VectorAlgorithm {
    /// Flat index (brute force)
    Flat,
    /// Hierarchical Navigable Small World
    Hnsw,
}

impl std::fmt::Display for VectorAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VectorAlgorithm::Flat => write!(f, "FLAT"),
            VectorAlgorithm::Hnsw => write!(f, "HNSW"),
        }
    }
}

/// Distance metric for vector similarity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum DistanceMetric {
    /// L2 Euclidean distance
    L2,
    /// Inner product (dot product)
    Ip,
    /// Cosine similarity
    Cosine,
}

impl std::fmt::Display for DistanceMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DistanceMetric::L2 => write!(f, "L2"),
            DistanceMetric::Ip => write!(f, "IP"),
            DistanceMetric::Cosine => write!(f, "COSINE"),
        }
    }
}

/// Vector field options
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct VectorFieldOptions {
    /// Vector similarity algorithm
    pub algorithm: Option<VectorAlgorithm>,
    /// Vector dimension
    pub dim: Option<u32>,
    /// Distance metric
    pub distance_metric: Option<DistanceMetric>,
    /// Initial capacity (FLAT algorithm)
    pub initial_cap: Option<u32>,
    /// Block size (FLAT algorithm)
    pub block_size: Option<u32>,
    /// M parameter (HNSW algorithm)
    pub m: Option<u32>,
    /// EF construction (HNSW algorithm)
    pub ef_construction: Option<u32>,
    /// EF runtime (HNSW algorithm)
    pub ef_runtime: Option<u32>,
}

/// Text field phonetic matcher
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum PhoneticMatcher {
    /// Double Metaphone for English
    #[serde(rename = "dm:en")]
    DmEn,
    /// Double Metaphone for French
    #[serde(rename = "dm:fr")]
    DmFr,
    /// Double Metaphone for Portuguese
    #[serde(rename = "dm:pt")]
    DmPt,
    /// Double Metaphone for Spanish
    #[serde(rename = "dm:es")]
    DmEs,
}

impl std::fmt::Display for PhoneticMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhoneticMatcher::DmEn => write!(f, "dm:en"),
            PhoneticMatcher::DmFr => write!(f, "dm:fr"),
            PhoneticMatcher::DmPt => write!(f, "dm:pt"),
            PhoneticMatcher::DmEs => write!(f, "dm:es"),
        }
    }
}

/// Schema definition for a search index field
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchFieldSchema {
    /// Field name (as stored in Redis)
    pub name: String,

    /// Alias for the field (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,

    /// Field type
    pub field_type: SearchFieldType,

    /// Whether the field is sortable
    #[serde(default)]
    pub sortable: bool,

    /// UNF (Unnormalized Form) - keeps original value for sorting
    #[serde(default)]
    pub unf: bool,

    /// Whether to skip indexing this field
    #[serde(default)]
    pub no_index: bool,

    /// Text field weight (default 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,

    /// Text field: enable stemming (default true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_stem: Option<bool>,

    /// Text field: phonetic matching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phonetic: Option<PhoneticMatcher>,

    /// Tag field: separator character (default comma)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,

    /// Tag field: case-sensitive matching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,

    /// Tag field: index empty values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_empty: Option<bool>,

    /// Vector field options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_options: Option<VectorFieldOptions>,

    /// Missing field policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_field_policy: Option<String>,
}

/// Data type for index (HASH or JSON)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum IndexDataType {
    /// Hash data type
    #[default]
    Hash,
    /// JSON data type
    Json,
}

impl std::fmt::Display for IndexDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexDataType::Hash => write!(f, "HASH"),
            IndexDataType::Json => write!(f, "JSON"),
        }
    }
}

/// Options for creating a search index
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct IndexCreateOptions {
    /// Data type to index (HASH or JSON)
    #[serde(default)]
    pub on: IndexDataType,

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

    /// Field containing document payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_field: Option<String>,

    /// Maximum text expansion (default 200)
    #[serde(skip_serializing_if = "Option::is_none")]
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

    /// Skip initial scan of existing documents
    #[serde(default)]
    pub skip_initial_scan: bool,

    /// Stop words list
    #[serde(default)]
    pub stopwords: Vec<String>,
}

/// Information about a search index
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexInfo {
    /// Index name
    pub index_name: String,

    /// Index options
    #[serde(default)]
    pub index_options: Vec<String>,

    /// Index definition
    #[serde(default)]
    pub index_definition: HashMap<String, serde_json::Value>,

    /// Field attributes
    #[serde(default)]
    pub attributes: Vec<HashMap<String, serde_json::Value>>,

    /// Number of documents
    pub num_docs: u64,

    /// Maximum document ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_doc_id: Option<u64>,

    /// Number of terms
    pub num_terms: u64,

    /// Number of records
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_records: Option<u64>,

    /// Inverted size in MB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverted_sz_mb: Option<f64>,

    /// Vector index size in MB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_index_sz_mb: Option<f64>,

    /// Total inverted index memory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_inverted_index_blocks: Option<u64>,

    /// Offset vectors size in MB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_vectors_sz_mb: Option<f64>,

    /// Doc table size in MB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_table_size_mb: Option<f64>,

    /// Sortable values size in MB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sortable_values_size_mb: Option<f64>,

    /// Key table size in MB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_table_size_mb: Option<f64>,

    /// Records per doc average
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_per_doc_avg: Option<f64>,

    /// Bytes per record average
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_per_record_avg: Option<f64>,

    /// Offsets per term average
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offsets_per_term_avg: Option<f64>,

    /// Offset bits per record average
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_bits_per_record_avg: Option<f64>,

    /// Whether index is being indexed
    #[serde(default)]
    pub indexing: bool,

    /// Indexing percentage complete
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent_indexed: Option<f64>,

    /// Hash indexing failures
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash_indexing_failures: Option<u64>,

    /// GC statistics
    #[serde(default)]
    pub gc_stats: HashMap<String, serde_json::Value>,

    /// Cursor statistics
    #[serde(default)]
    pub cursor_stats: HashMap<String, serde_json::Value>,
}

// ==================== Search Query Types ====================

/// Geographic filter for search
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoFilter {
    /// Field name
    pub field: String,
    /// Longitude
    pub lon: f64,
    /// Latitude
    pub lat: f64,
    /// Radius value
    pub radius: f64,
    /// Radius unit (m, km, mi, ft)
    pub unit: String,
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum SortOrder {
    /// Ascending order
    #[default]
    Asc,
    /// Descending order
    Desc,
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortOrder::Asc => write!(f, "ASC"),
            SortOrder::Desc => write!(f, "DESC"),
        }
    }
}

/// Sort by specification
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SortBy {
    /// Field to sort by
    pub field: String,
    /// Sort order
    #[serde(default)]
    pub order: SortOrder,
}

/// Highlight options
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct HighlightOptions {
    /// Fields to highlight
    #[serde(default)]
    pub fields: Vec<String>,
    /// Opening tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_tag: Option<String>,
    /// Closing tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_tag: Option<String>,
}

/// Summarize options
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SummarizeOptions {
    /// Fields to summarize
    #[serde(default)]
    pub fields: Vec<String>,
    /// Number of fragments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frags: Option<u32>,
    /// Fragment length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub len: Option<u32>,
    /// Separator between fragments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
}

/// Numeric filter for search
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NumericFilter {
    /// Field name
    pub field: String,
    /// Minimum value (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Maximum value (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Exclude minimum (make it exclusive)
    #[serde(default)]
    pub exclusive_min: bool,
    /// Exclude maximum (make it exclusive)
    #[serde(default)]
    pub exclusive_max: bool,
}

/// Options for FT.SEARCH command
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SearchOptions {
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
    pub params: HashMap<String, String>,

    /// Dialect version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialect: Option<u32>,
}

fn default_limit() -> u64 {
    10
}

/// A document returned from search
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchDocument {
    /// Document ID (Redis key)
    pub id: String,

    /// Document score (if WITHSCORES)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    /// Document payload (if WITHPAYLOADS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,

    /// Sort key (if WITHSORTKEYS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sortkey: Option<String>,

    /// Document fields
    #[serde(default)]
    pub fields: HashMap<String, serde_json::Value>,

    /// Score explanation (if EXPLAINSCORE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_explanation: Option<Vec<String>>,
}

/// Result of FT.SEARCH operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResult {
    /// Total number of results (may be more than returned)
    pub total_results: u64,

    /// Returned documents
    pub documents: Vec<SearchDocument>,
}

// ==================== Aggregation Types ====================

/// Reducer function for aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Reducer {
    /// Function name (COUNT, SUM, AVG, etc.)
    pub function: String,
    /// Arguments to the function
    #[serde(default)]
    pub args: Vec<String>,
    /// Alias for the result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// GROUPBY step in aggregation pipeline
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GroupByStep {
    /// Fields to group by
    pub fields: Vec<String>,
    /// Reducers to apply
    #[serde(default)]
    pub reducers: Vec<Reducer>,
}

/// SORTBY step in aggregation pipeline
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SortByStep {
    /// Fields to sort by with optional order
    pub fields: Vec<SortBy>,
    /// Maximum results to sort
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u64>,
}

/// APPLY step in aggregation pipeline
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApplyStep {
    /// Expression to apply
    pub expression: String,
    /// Alias for result
    pub alias: String,
}

/// LIMIT step in aggregation pipeline
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LimitStep {
    /// Offset
    pub offset: u64,
    /// Number of results
    pub num: u64,
}

/// FILTER step in aggregation pipeline
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FilterStep {
    /// Filter expression
    pub expression: String,
}

/// Aggregation pipeline step
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
pub enum AggregateStep {
    /// Group by fields with reducers
    Groupby(GroupByStep),
    /// Sort results
    Sortby(SortByStep),
    /// Apply expression
    Apply(ApplyStep),
    /// Limit results
    Limit(LimitStep),
    /// Filter results
    Filter(FilterStep),
}

/// Options for FT.AGGREGATE command
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AggregateOptions {
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
    pub params: HashMap<String, String>,

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

/// Result of FT.AGGREGATE operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AggregateResult {
    /// Total number of results
    pub total_results: u64,

    /// Result rows
    pub rows: Vec<HashMap<String, serde_json::Value>>,

    /// Cursor ID (if WITHCURSOR was used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor_id: Option<u64>,
}

/// Result of FT.CURSOR READ operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CursorReadResult {
    /// Next cursor ID (0 if exhausted)
    pub cursor_id: u64,

    /// Result rows
    pub rows: Vec<HashMap<String, serde_json::Value>>,
}

// ==================== Autocomplete Types ====================

/// Options for FT.SUGADD command
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SugAddOptions {
    /// Increment existing score instead of replacing
    #[serde(default)]
    pub incr: bool,

    /// Payload to store with suggestion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
}

/// Options for FT.SUGGET command
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SugGetOptions {
    /// Enable fuzzy matching
    #[serde(default)]
    pub fuzzy: bool,

    /// Return scores with suggestions
    #[serde(default)]
    pub withscores: bool,

    /// Return payloads with suggestions
    #[serde(default)]
    pub withpayloads: bool,

    /// Maximum number of suggestions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
}

/// A suggestion from autocomplete
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Suggestion {
    /// The suggestion string
    pub string: String,

    /// Score (if WITHSCORES)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    /// Payload (if WITHPAYLOADS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
}

// ==================== Synonym Types ====================

/// Synonym group
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SynonymGroup {
    /// Group ID
    pub group_id: String,

    /// Terms in the group
    pub terms: Vec<String>,
}

// ==================== Spellcheck Types ====================

/// Options for FT.SPELLCHECK command
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SpellcheckOptions {
    /// Maximum edit distance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<u32>,

    /// Include terms from dictionary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<String>,

    /// Exclude terms from dictionary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<String>,

    /// Dialect version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialect: Option<u32>,
}

/// Spellcheck suggestion
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SpellcheckSuggestion {
    /// Score of the suggestion
    pub score: f64,

    /// Suggested term
    pub suggestion: String,
}

/// Spellcheck result for a term
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SpellcheckTerm {
    /// Original term
    pub term: String,

    /// Suggestions for the term
    pub suggestions: Vec<SpellcheckSuggestion>,
}

/// Result of FT.SPELLCHECK operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SpellcheckResult {
    /// Spellcheck results by term
    pub results: Vec<SpellcheckTerm>,
}

// ==================== Profile Types ====================

/// Profile type for FT.PROFILE
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum ProfileType {
    /// Profile a search query
    Search,
    /// Profile an aggregate query
    Aggregate,
}

impl std::fmt::Display for ProfileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileType::Search => write!(f, "SEARCH"),
            ProfileType::Aggregate => write!(f, "AGGREGATE"),
        }
    }
}

/// Result of FT.PROFILE operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProfileResult {
    /// Query results (search or aggregate)
    pub results: serde_json::Value,

    /// Profile information
    pub profile: HashMap<String, serde_json::Value>,
}

// ==================== Explain Types ====================

/// Result of FT.EXPLAIN operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExplainResult {
    /// Query execution plan
    pub plan: String,
}

// ==================== Hybrid Search Types ====================

/// Vector similarity input for FT.HYBRID VSIM clause
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
pub enum VsimInput {
    /// Reference element (existing key)
    Ele {
        /// The element key reference
        element: String,
    },
    /// Raw vector values
    Values {
        /// Vector dimension
        dim: u32,
        /// Vector values
        values: Vec<f64>,
    },
}

/// Combination strategy for FT.HYBRID.
/// Controls how text search and vector similarity scores are merged.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "method", rename_all = "UPPERCASE")]
pub enum CombineStrategy {
    /// Reciprocal Rank Fusion
    Rrf {
        /// RRF constant (default 60)
        #[serde(skip_serializing_if = "Option::is_none")]
        constant: Option<u32>,
        /// Window size
        #[serde(skip_serializing_if = "Option::is_none")]
        window: Option<u32>,
        /// Name for the combined score
        #[serde(skip_serializing_if = "Option::is_none")]
        yield_score_as: Option<String>,
    },
    /// Linear combination
    Linear {
        /// Weight for text search score
        #[serde(skip_serializing_if = "Option::is_none")]
        alpha: Option<f64>,
        /// Weight for vector similarity score
        #[serde(skip_serializing_if = "Option::is_none")]
        beta: Option<f64>,
        /// Window size
        #[serde(skip_serializing_if = "Option::is_none")]
        window: Option<u32>,
        /// Name for the combined score
        #[serde(skip_serializing_if = "Option::is_none")]
        yield_score_as: Option<String>,
    },
}

/// Options for FT.HYBRID command
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HybridSearchOptions {
    /// Text search query for the SEARCH clause
    pub query: String,

    /// Optional scorer for the SEARCH clause
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_scorer: Option<String>,

    /// Optional YIELD_SCORE_AS name for text search score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_yield_score_as: Option<String>,

    /// Vector field name for the VSIM clause
    pub vsim_field: String,

    /// Vector similarity input (ELE or VALUES) — required
    pub vsim_input: VsimInput,

    /// Optional YIELD_SCORE_AS name for vector score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vsim_yield_score_as: Option<String>,

    /// Fields to LOAD from documents
    #[serde(default)]
    pub load: Vec<String>,

    /// APPLY expressions (expression, alias)
    #[serde(default)]
    pub apply: Vec<ApplyStep>,

    /// SORTBY specifications
    #[serde(default)]
    pub sortby: Vec<SortBy>,

    /// LIMIT offset
    #[serde(default)]
    pub offset: u64,

    /// LIMIT count (default 10)
    #[serde(default = "default_hybrid_limit")]
    pub limit: u64,

    /// Parameters for parameterized queries
    #[serde(default)]
    pub params: HashMap<String, String>,

    /// FILTER expressions
    #[serde(default)]
    pub filters: Vec<String>,

    /// Combination strategy (RRF or LINEAR with parameters)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub combine: Option<CombineStrategy>,

    /// Execution policy: ADHOC or BATCHES
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,

    /// Batch size (when policy is BATCHES)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u32>,
}

fn default_hybrid_limit() -> u64 {
    10
}

/// Result of FT.HYBRID operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HybridSearchResult {
    /// Total number of results
    pub total_results: u64,

    /// Returned documents (same format as FT.SEARCH)
    pub documents: Vec<SearchDocument>,
}

// ==================== Result Types ====================

/// Result of FT.CREATE operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IndexCreateResult {
    /// Index name
    pub index: String,

    /// Whether creation was successful
    pub success: bool,
}

/// Result of FT.DROPINDEX operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IndexDropResult {
    /// Index name
    pub index: String,

    /// Whether documents were deleted
    pub delete_docs: bool,

    /// Whether operation was successful
    pub success: bool,
}

/// Result of FT.ALTER operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IndexAlterResult {
    /// Index name
    pub index: String,

    /// Field that was added
    pub field: String,

    /// Whether operation was successful
    pub success: bool,
}

/// Result of alias operations
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AliasResult {
    /// Alias name
    pub alias: String,

    /// Target index
    pub index: String,

    /// Whether operation was successful
    pub success: bool,
}

/// Result of FT.SUGADD operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SugAddResult {
    /// Dictionary key
    pub key: String,

    /// Current dictionary size
    pub size: i64,
}

/// Result of FT.SUGDEL operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SugDelResult {
    /// Dictionary key
    pub key: String,

    /// Whether deletion was successful
    pub deleted: bool,
}

/// Result of FT.SUGLEN operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SugLenResult {
    /// Dictionary key
    pub key: String,

    /// Dictionary size
    pub size: i64,
}

/// Result of FT.SYNUPDATE operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SynonymUpdateResult {
    /// Index name
    pub index: String,

    /// Group ID
    pub group_id: String,

    /// Whether operation was successful
    pub success: bool,
}

/// Result of dictionary operations
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DictResult {
    /// Dictionary name
    pub dict: String,

    /// Number of terms added/deleted
    pub count: i64,
}

/// Result of FT.DICTDUMP operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DictDumpResult {
    /// Dictionary name
    pub dict: String,

    /// Terms in dictionary
    pub terms: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_field_type_display() {
        assert_eq!(SearchFieldType::Text.to_string(), "TEXT");
        assert_eq!(SearchFieldType::Tag.to_string(), "TAG");
        assert_eq!(SearchFieldType::Numeric.to_string(), "NUMERIC");
        assert_eq!(SearchFieldType::Geo.to_string(), "GEO");
        assert_eq!(SearchFieldType::Vector.to_string(), "VECTOR");
        assert_eq!(SearchFieldType::Geoshape.to_string(), "GEOSHAPE");
    }

    #[test]
    fn test_index_data_type_display() {
        assert_eq!(IndexDataType::Hash.to_string(), "HASH");
        assert_eq!(IndexDataType::Json.to_string(), "JSON");
    }

    #[test]
    fn test_sort_order_display() {
        assert_eq!(SortOrder::Asc.to_string(), "ASC");
        assert_eq!(SortOrder::Desc.to_string(), "DESC");
    }

    #[test]
    fn test_search_options_defaults() {
        let options = SearchOptions::default();
        assert!(!options.nocontent);
        assert!(!options.withscores);
        assert_eq!(options.offset, 0);
        // Note: Default derive gives 0; serde deserialization uses default_limit (10)
        assert_eq!(options.limit, 0);
    }

    #[test]
    fn test_search_options_deserialize_default_limit() {
        let options: SearchOptions = serde_json::from_str("{}").unwrap();
        assert_eq!(options.limit, 10);
    }

    #[test]
    fn test_phonetic_matcher_display() {
        assert_eq!(PhoneticMatcher::DmEn.to_string(), "dm:en");
        assert_eq!(PhoneticMatcher::DmFr.to_string(), "dm:fr");
        assert_eq!(PhoneticMatcher::DmPt.to_string(), "dm:pt");
        assert_eq!(PhoneticMatcher::DmEs.to_string(), "dm:es");
    }

    #[test]
    fn test_profile_type_display() {
        assert_eq!(ProfileType::Search.to_string(), "SEARCH");
        assert_eq!(ProfileType::Aggregate.to_string(), "AGGREGATE");
    }

    #[test]
    fn test_aggregate_step_serialization() {
        let step = AggregateStep::Groupby(GroupByStep {
            fields: vec!["category".to_string()],
            reducers: vec![Reducer {
                function: "COUNT".to_string(),
                args: vec![],
                alias: Some("count".to_string()),
            }],
        });

        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("GROUPBY"));
        assert!(json.contains("category"));
    }

    #[test]
    fn test_suggestion_with_optional_fields() {
        let suggestion = Suggestion {
            string: "hello".to_string(),
            score: Some(1.5),
            payload: None,
        };

        let json = serde_json::to_string(&suggestion).unwrap();
        assert!(json.contains("hello"));
        assert!(json.contains("1.5"));
        assert!(!json.contains("payload"));
    }

    #[test]
    fn test_vector_algorithm_display() {
        assert_eq!(VectorAlgorithm::Flat.to_string(), "FLAT");
        assert_eq!(VectorAlgorithm::Hnsw.to_string(), "HNSW");
    }

    #[test]
    fn test_hybrid_search_options_deserialize_default_limit() {
        let options: HybridSearchOptions = serde_json::from_str(
            r#"{
                "query": "*",
                "vsim_field": "embedding",
                "vsim_input": {"type": "ELE", "element": "doc-1"}
            }"#,
        )
        .unwrap();

        assert_eq!(options.limit, 10);
        assert!(options.params.is_empty());
        assert!(options.filters.is_empty());
    }

    #[test]
    fn test_distance_metric_display() {
        assert_eq!(DistanceMetric::L2.to_string(), "L2");
        assert_eq!(DistanceMetric::Ip.to_string(), "IP");
        assert_eq!(DistanceMetric::Cosine.to_string(), "COSINE");
    }

    #[test]
    fn test_field_schema_serialization() {
        let field = SearchFieldSchema {
            name: "title".to_string(),
            alias: None,
            field_type: SearchFieldType::Text,
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
            missing_field_policy: None,
        };

        let json = serde_json::to_string(&field).unwrap();
        assert!(json.contains("title"));
        assert!(json.contains("TEXT"));
        assert!(json.contains("2.0"));
    }
}
