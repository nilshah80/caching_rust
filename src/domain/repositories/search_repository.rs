//! Search Repository Trait
//!
//! Abstract interface for RediSearch operations.

use async_trait::async_trait;

use crate::domain::entities::{
    AggregateOptions, AggregateResult, AliasResult, DictDumpResult, DictResult, ExplainResult,
    IndexAlterResult, IndexCreateOptions, IndexCreateResult, IndexDropResult, IndexInfo,
    ProfileResult, ProfileType, SearchFieldSchema, SearchOptions, SearchResult, SpellcheckOptions,
    SpellcheckResult, SugAddOptions, SugAddResult, SugDelResult, SugGetOptions, SugLenResult,
    Suggestion, SynonymGroup, SynonymUpdateResult,
};
use crate::domain::errors::CacheError;

/// Repository trait for RediSearch operations
#[async_trait]
pub trait SearchRepository: Send + Sync {
    // ==================== Index Operations ====================

    /// FT.CREATE - Create a search index
    ///
    /// # Arguments
    /// * `index` - Index name
    /// * `options` - Index creation options
    /// * `schema` - Field schema definitions
    async fn ft_create(
        &self,
        index: &str,
        options: &IndexCreateOptions,
        schema: &[SearchFieldSchema],
    ) -> Result<IndexCreateResult, CacheError>;

    /// FT.DROPINDEX - Drop a search index
    ///
    /// # Arguments
    /// * `index` - Index name
    /// * `delete_docs` - Whether to delete indexed documents
    async fn ft_drop_index(
        &self,
        index: &str,
        delete_docs: bool,
    ) -> Result<IndexDropResult, CacheError>;

    /// FT.INFO - Get index information
    ///
    /// # Arguments
    /// * `index` - Index name
    async fn ft_info(&self, index: &str) -> Result<IndexInfo, CacheError>;

    /// FT._LIST - List all indices
    async fn ft_list(&self) -> Result<Vec<String>, CacheError>;

    /// FT.ALTER - Add field to existing index
    ///
    /// # Arguments
    /// * `index` - Index name
    /// * `field` - New field schema
    async fn ft_alter(
        &self,
        index: &str,
        field: &SearchFieldSchema,
    ) -> Result<IndexAlterResult, CacheError>;

    // ==================== Query Operations ====================

    /// FT.SEARCH - Execute a search query
    ///
    /// # Arguments
    /// * `index` - Index name
    /// * `query` - Query string
    /// * `options` - Search options
    async fn ft_search(
        &self,
        index: &str,
        query: &str,
        options: &SearchOptions,
    ) -> Result<SearchResult, CacheError>;

    /// FT.AGGREGATE - Execute an aggregation query
    ///
    /// # Arguments
    /// * `index` - Index name
    /// * `query` - Query string
    /// * `options` - Aggregation options
    async fn ft_aggregate(
        &self,
        index: &str,
        query: &str,
        options: &AggregateOptions,
    ) -> Result<AggregateResult, CacheError>;

    /// FT.EXPLAIN - Get query execution plan
    ///
    /// # Arguments
    /// * `index` - Index name
    /// * `query` - Query string
    /// * `dialect` - Query dialect version (optional)
    async fn ft_explain(
        &self,
        index: &str,
        query: &str,
        dialect: Option<u32>,
    ) -> Result<ExplainResult, CacheError>;

    /// FT.PROFILE - Profile a query
    ///
    /// # Arguments
    /// * `index` - Index name
    /// * `profile_type` - SEARCH or AGGREGATE
    /// * `limited` - Whether to limit profiling output
    /// * `query` - Query string
    /// * `options` - Query-specific options (search or aggregate)
    async fn ft_profile(
        &self,
        index: &str,
        profile_type: ProfileType,
        limited: bool,
        query: &str,
        search_options: Option<&SearchOptions>,
        aggregate_options: Option<&AggregateOptions>,
    ) -> Result<ProfileResult, CacheError>;

    // ==================== Alias Operations ====================

    /// FT.ALIASADD - Create an index alias
    ///
    /// # Arguments
    /// * `alias` - Alias name
    /// * `index` - Target index name
    async fn ft_aliasadd(&self, alias: &str, index: &str) -> Result<AliasResult, CacheError>;

    /// FT.ALIASDEL - Delete an index alias
    ///
    /// # Arguments
    /// * `alias` - Alias name
    async fn ft_aliasdel(&self, alias: &str) -> Result<AliasResult, CacheError>;

    /// FT.ALIASUPDATE - Update an index alias
    ///
    /// # Arguments
    /// * `alias` - Alias name
    /// * `index` - New target index name
    async fn ft_aliasupdate(&self, alias: &str, index: &str) -> Result<AliasResult, CacheError>;

    // ==================== Autocomplete Operations ====================

    /// FT.SUGADD - Add a suggestion to a dictionary
    ///
    /// # Arguments
    /// * `key` - Dictionary key
    /// * `string` - Suggestion string
    /// * `score` - Suggestion score
    /// * `options` - Add options
    async fn ft_sugadd(
        &self,
        key: &str,
        string: &str,
        score: f64,
        options: &SugAddOptions,
    ) -> Result<SugAddResult, CacheError>;

    /// FT.SUGGET - Get suggestions for a prefix
    ///
    /// # Arguments
    /// * `key` - Dictionary key
    /// * `prefix` - Prefix to search for
    /// * `options` - Get options
    async fn ft_sugget(
        &self,
        key: &str,
        prefix: &str,
        options: &SugGetOptions,
    ) -> Result<Vec<Suggestion>, CacheError>;

    /// FT.SUGDEL - Delete a suggestion from a dictionary
    ///
    /// # Arguments
    /// * `key` - Dictionary key
    /// * `string` - Suggestion string to delete
    async fn ft_sugdel(&self, key: &str, string: &str) -> Result<SugDelResult, CacheError>;

    /// FT.SUGLEN - Get dictionary size
    ///
    /// # Arguments
    /// * `key` - Dictionary key
    async fn ft_suglen(&self, key: &str) -> Result<SugLenResult, CacheError>;

    // ==================== Synonym Operations ====================

    /// FT.SYNDUMP - Dump all synonym groups
    ///
    /// # Arguments
    /// * `index` - Index name
    async fn ft_syndump(&self, index: &str) -> Result<Vec<SynonymGroup>, CacheError>;

    /// FT.SYNUPDATE - Update a synonym group
    ///
    /// # Arguments
    /// * `index` - Index name
    /// * `group_id` - Synonym group ID
    /// * `skip_initial_scan` - Whether to skip initial scan
    /// * `terms` - Synonym terms
    async fn ft_synupdate(
        &self,
        index: &str,
        group_id: &str,
        skip_initial_scan: bool,
        terms: &[String],
    ) -> Result<SynonymUpdateResult, CacheError>;

    // ==================== Spellcheck Operations ====================

    /// FT.SPELLCHECK - Check spelling in query
    ///
    /// # Arguments
    /// * `index` - Index name
    /// * `query` - Query to spellcheck
    /// * `options` - Spellcheck options
    async fn ft_spellcheck(
        &self,
        index: &str,
        query: &str,
        options: &SpellcheckOptions,
    ) -> Result<SpellcheckResult, CacheError>;

    // ==================== Dictionary Operations ====================

    /// FT.DICTADD - Add terms to a dictionary
    ///
    /// # Arguments
    /// * `dict` - Dictionary name
    /// * `terms` - Terms to add
    async fn ft_dictadd(&self, dict: &str, terms: &[String]) -> Result<DictResult, CacheError>;

    /// FT.DICTDEL - Delete terms from a dictionary
    ///
    /// # Arguments
    /// * `dict` - Dictionary name
    /// * `terms` - Terms to delete
    async fn ft_dictdel(&self, dict: &str, terms: &[String]) -> Result<DictResult, CacheError>;

    /// FT.DICTDUMP - Dump all terms in a dictionary
    ///
    /// # Arguments
    /// * `dict` - Dictionary name
    async fn ft_dictdump(&self, dict: &str) -> Result<DictDumpResult, CacheError>;
}
