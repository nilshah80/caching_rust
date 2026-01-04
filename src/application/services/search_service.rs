//! Search Service
//!
//! Business logic layer for RediSearch operations.

use std::sync::Arc;

use crate::domain::entities::{
    AggregateOptions, AggregateResult, AliasResult, DictDumpResult, DictResult, ExplainResult,
    IndexAlterResult, IndexCreateOptions, IndexCreateResult, IndexDropResult, IndexInfo,
    ProfileResult, ProfileType, SearchFieldSchema, SearchOptions, SearchResult, SpellcheckOptions,
    SpellcheckResult, SugAddOptions, SugAddResult, SugDelResult, SugGetOptions, SugLenResult,
    Suggestion, SynonymGroup, SynonymUpdateResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::SearchRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisSearchRepository;

/// Service for RediSearch operations
pub struct SearchService {
    repository: Arc<dyn SearchRepository>,
}

impl SearchService {
    /// Create a new SearchService
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisSearchRepository::new(pool)))
    }

    /// Create a SearchService with a custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn SearchRepository>) -> Self {
        Self { repository }
    }

    // ==================== Index Operations ====================

    /// Create a search index
    pub async fn ft_create(
        &self,
        index: &str,
        options: IndexCreateOptions,
        schema: Vec<SearchFieldSchema>,
    ) -> Result<IndexCreateResult, CacheError> {
        self.validate_index_name(index)?;
        self.validate_schema(&schema)?;
        self.repository.ft_create(index, &options, &schema).await
    }

    /// Drop a search index
    pub async fn ft_drop_index(
        &self,
        index: &str,
        delete_docs: bool,
    ) -> Result<IndexDropResult, CacheError> {
        self.validate_index_name(index)?;
        self.repository.ft_drop_index(index, delete_docs).await
    }

    /// Get index information
    pub async fn ft_info(&self, index: &str) -> Result<IndexInfo, CacheError> {
        self.validate_index_name(index)?;
        self.repository.ft_info(index).await
    }

    /// List all indices
    pub async fn ft_list(&self) -> Result<Vec<String>, CacheError> {
        self.repository.ft_list().await
    }

    /// Add field to existing index
    pub async fn ft_alter(
        &self,
        index: &str,
        field: SearchFieldSchema,
    ) -> Result<IndexAlterResult, CacheError> {
        self.validate_index_name(index)?;
        self.validate_field(&field)?;
        self.repository.ft_alter(index, &field).await
    }

    // ==================== Query Operations ====================

    /// Execute a search query
    pub async fn ft_search(
        &self,
        index: &str,
        query: &str,
        options: SearchOptions,
    ) -> Result<SearchResult, CacheError> {
        self.validate_index_name(index)?;
        self.validate_query(query)?;
        self.repository.ft_search(index, query, &options).await
    }

    /// Execute an aggregation query
    pub async fn ft_aggregate(
        &self,
        index: &str,
        query: &str,
        options: AggregateOptions,
    ) -> Result<AggregateResult, CacheError> {
        self.validate_index_name(index)?;
        self.validate_query(query)?;
        self.repository.ft_aggregate(index, query, &options).await
    }

    /// Get query execution plan
    pub async fn ft_explain(
        &self,
        index: &str,
        query: &str,
        dialect: Option<u32>,
    ) -> Result<ExplainResult, CacheError> {
        self.validate_index_name(index)?;
        self.validate_query(query)?;
        self.repository.ft_explain(index, query, dialect).await
    }

    /// Profile a query
    pub async fn ft_profile(
        &self,
        index: &str,
        profile_type: ProfileType,
        limited: bool,
        query: &str,
        search_options: Option<SearchOptions>,
        aggregate_options: Option<AggregateOptions>,
    ) -> Result<ProfileResult, CacheError> {
        self.validate_index_name(index)?;
        self.validate_query(query)?;

        self.repository
            .ft_profile(
                index,
                profile_type,
                limited,
                query,
                search_options.as_ref(),
                aggregate_options.as_ref(),
            )
            .await
    }

    // ==================== Alias Operations ====================

    /// Create an index alias
    pub async fn ft_aliasadd(&self, alias: &str, index: &str) -> Result<AliasResult, CacheError> {
        self.validate_alias_name(alias)?;
        self.validate_index_name(index)?;
        self.repository.ft_aliasadd(alias, index).await
    }

    /// Delete an index alias
    pub async fn ft_aliasdel(&self, alias: &str) -> Result<AliasResult, CacheError> {
        self.validate_alias_name(alias)?;
        self.repository.ft_aliasdel(alias).await
    }

    /// Update an index alias
    pub async fn ft_aliasupdate(
        &self,
        alias: &str,
        index: &str,
    ) -> Result<AliasResult, CacheError> {
        self.validate_alias_name(alias)?;
        self.validate_index_name(index)?;
        self.repository.ft_aliasupdate(alias, index).await
    }

    // ==================== Autocomplete Operations ====================

    /// Add a suggestion to a dictionary
    pub async fn ft_sugadd(
        &self,
        key: &str,
        string: &str,
        score: f64,
        options: SugAddOptions,
    ) -> Result<SugAddResult, CacheError> {
        self.validate_key(key)?;
        self.validate_suggestion_string(string)?;
        self.validate_score(score)?;
        self.repository.ft_sugadd(key, string, score, &options).await
    }

    /// Get suggestions for a prefix
    pub async fn ft_sugget(
        &self,
        key: &str,
        prefix: &str,
        options: SugGetOptions,
    ) -> Result<Vec<Suggestion>, CacheError> {
        self.validate_key(key)?;
        // Prefix can be empty for getting all suggestions
        self.repository.ft_sugget(key, prefix, &options).await
    }

    /// Delete a suggestion from a dictionary
    pub async fn ft_sugdel(&self, key: &str, string: &str) -> Result<SugDelResult, CacheError> {
        self.validate_key(key)?;
        self.validate_suggestion_string(string)?;
        self.repository.ft_sugdel(key, string).await
    }

    /// Get dictionary size
    pub async fn ft_suglen(&self, key: &str) -> Result<SugLenResult, CacheError> {
        self.validate_key(key)?;
        self.repository.ft_suglen(key).await
    }

    // ==================== Synonym Operations ====================

    /// Dump all synonym groups
    pub async fn ft_syndump(&self, index: &str) -> Result<Vec<SynonymGroup>, CacheError> {
        self.validate_index_name(index)?;
        self.repository.ft_syndump(index).await
    }

    /// Update a synonym group
    pub async fn ft_synupdate(
        &self,
        index: &str,
        group_id: &str,
        skip_initial_scan: bool,
        terms: Vec<String>,
    ) -> Result<SynonymUpdateResult, CacheError> {
        self.validate_index_name(index)?;
        self.validate_group_id(group_id)?;
        if terms.is_empty() {
            return Err(CacheError::InvalidInput(
                "Terms list cannot be empty".to_string(),
            ));
        }
        self.repository
            .ft_synupdate(index, group_id, skip_initial_scan, &terms)
            .await
    }

    // ==================== Spellcheck Operations ====================

    /// Check spelling in query
    pub async fn ft_spellcheck(
        &self,
        index: &str,
        query: &str,
        options: SpellcheckOptions,
    ) -> Result<SpellcheckResult, CacheError> {
        self.validate_index_name(index)?;
        self.validate_query(query)?;
        self.repository.ft_spellcheck(index, query, &options).await
    }

    // ==================== Dictionary Operations ====================

    /// Add terms to a dictionary
    pub async fn ft_dictadd(
        &self,
        dict: &str,
        terms: Vec<String>,
    ) -> Result<DictResult, CacheError> {
        self.validate_dict_name(dict)?;
        if terms.is_empty() {
            return Err(CacheError::InvalidInput(
                "Terms list cannot be empty".to_string(),
            ));
        }
        self.repository.ft_dictadd(dict, &terms).await
    }

    /// Delete terms from a dictionary
    pub async fn ft_dictdel(
        &self,
        dict: &str,
        terms: Vec<String>,
    ) -> Result<DictResult, CacheError> {
        self.validate_dict_name(dict)?;
        if terms.is_empty() {
            return Err(CacheError::InvalidInput(
                "Terms list cannot be empty".to_string(),
            ));
        }
        self.repository.ft_dictdel(dict, &terms).await
    }

    /// Dump all terms in a dictionary
    pub async fn ft_dictdump(&self, dict: &str) -> Result<DictDumpResult, CacheError> {
        self.validate_dict_name(dict)?;
        self.repository.ft_dictdump(dict).await
    }

    // ==================== Validation Helpers ====================

    /// Validate index name
    fn validate_index_name(&self, index: &str) -> Result<(), CacheError> {
        if index.is_empty() {
            return Err(CacheError::InvalidInput(
                "Index name cannot be empty".to_string(),
            ));
        }
        if index.len() > 128 {
            return Err(CacheError::InvalidInput(
                "Index name cannot exceed 128 characters".to_string(),
            ));
        }
        // Index names should be alphanumeric with underscores and hyphens
        if !index
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ':')
        {
            return Err(CacheError::InvalidInput(
                "Index name can only contain alphanumeric characters, underscores, hyphens, and colons".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate alias name
    fn validate_alias_name(&self, alias: &str) -> Result<(), CacheError> {
        if alias.is_empty() {
            return Err(CacheError::InvalidInput(
                "Alias name cannot be empty".to_string(),
            ));
        }
        if alias.len() > 128 {
            return Err(CacheError::InvalidInput(
                "Alias name cannot exceed 128 characters".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate key
    fn validate_key(&self, key: &str) -> Result<(), CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        Ok(())
    }

    /// Validate query string
    fn validate_query(&self, query: &str) -> Result<(), CacheError> {
        if query.is_empty() {
            return Err(CacheError::InvalidInput(
                "Query cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate suggestion string
    fn validate_suggestion_string(&self, string: &str) -> Result<(), CacheError> {
        if string.is_empty() {
            return Err(CacheError::InvalidInput(
                "Suggestion string cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate score
    fn validate_score(&self, score: f64) -> Result<(), CacheError> {
        if score < 0.0 {
            return Err(CacheError::InvalidInput(
                "Score must be non-negative".to_string(),
            ));
        }
        if !score.is_finite() {
            return Err(CacheError::InvalidInput(
                "Score must be a finite number".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate group ID
    fn validate_group_id(&self, group_id: &str) -> Result<(), CacheError> {
        if group_id.is_empty() {
            return Err(CacheError::InvalidInput(
                "Group ID cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate dictionary name
    fn validate_dict_name(&self, dict: &str) -> Result<(), CacheError> {
        if dict.is_empty() {
            return Err(CacheError::InvalidInput(
                "Dictionary name cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate schema
    fn validate_schema(&self, schema: &[SearchFieldSchema]) -> Result<(), CacheError> {
        if schema.is_empty() {
            return Err(CacheError::InvalidInput(
                "Schema cannot be empty - at least one field is required".to_string(),
            ));
        }
        for field in schema {
            self.validate_field(field)?;
        }
        Ok(())
    }

    /// Validate a single field
    fn validate_field(&self, field: &SearchFieldSchema) -> Result<(), CacheError> {
        if field.name.is_empty() {
            return Err(CacheError::InvalidInput(
                "Field name cannot be empty".to_string(),
            ));
        }
        // Validate weight if present
        if let Some(weight) = field.weight {
            if weight < 0.0 {
                return Err(CacheError::InvalidInput(
                    "Field weight must be non-negative".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        AggregateOptions, IndexCreateOptions, ProfileType, SearchFieldType, SearchOptions,
        SugAddOptions, SugGetOptions, SpellcheckOptions,
    };
    use crate::infrastructure::redis::connection::InstrumentedPool;
    use crate::test_support::MockSearchRepository;

    fn create_service() -> SearchService {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        SearchService::new(pool)
    }

    fn create_mock_service() -> SearchService {
        SearchService::new_with_repository(Arc::new(MockSearchRepository::new()))
    }

    #[test]
    fn test_validate_index_name() {
        let service = create_service();

        // Valid names
        assert!(service.validate_index_name("myindex").is_ok());
        assert!(service.validate_index_name("my-index").is_ok());
        assert!(service.validate_index_name("my_index").is_ok());
        assert!(service.validate_index_name("my:index").is_ok());
        assert!(service.validate_index_name("MyIndex123").is_ok());

        // Invalid names
        assert!(service.validate_index_name("").is_err());
        assert!(service.validate_index_name("my index").is_err()); // space
        assert!(service.validate_index_name("my.index").is_err()); // dot
        assert!(service.validate_index_name(&"a".repeat(129)).is_err()); // too long
    }

    #[test]
    fn test_validate_query() {
        let service = create_service();

        assert!(service.validate_query("*").is_ok());
        assert!(service.validate_query("hello world").is_ok());
        assert!(service.validate_query("@title:hello").is_ok());

        assert!(service.validate_query("").is_err());
    }

    #[test]
    fn test_validate_score() {
        let service = create_service();

        assert!(service.validate_score(0.0).is_ok());
        assert!(service.validate_score(1.5).is_ok());
        assert!(service.validate_score(100.0).is_ok());

        assert!(service.validate_score(-1.0).is_err());
        assert!(service.validate_score(f64::INFINITY).is_err());
        assert!(service.validate_score(f64::NAN).is_err());
    }

    #[test]
    fn test_validate_schema() {
        let service = create_service();

        // Valid schema
        let valid_schema = vec![SearchFieldSchema {
            name: "title".to_string(),
            alias: None,
            field_type: SearchFieldType::Text,
            sortable: false,
            unf: false,
            no_index: false,
            weight: Some(1.0),
            no_stem: None,
            phonetic: None,
            separator: None,
            case_sensitive: None,
            index_empty: None,
            vector_options: None,
            missing_field_policy: None,
        }];
        assert!(service.validate_schema(&valid_schema).is_ok());

        // Empty schema
        assert!(service.validate_schema(&[]).is_err());

        // Schema with empty field name
        let invalid_schema = vec![SearchFieldSchema {
            name: "".to_string(),
            alias: None,
            field_type: SearchFieldType::Text,
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
            missing_field_policy: None,
        }];
        assert!(service.validate_schema(&invalid_schema).is_err());

        // Schema with negative weight
        let negative_weight_schema = vec![SearchFieldSchema {
            name: "title".to_string(),
            alias: None,
            field_type: SearchFieldType::Text,
            sortable: false,
            unf: false,
            no_index: false,
            weight: Some(-1.0),
            no_stem: None,
            phonetic: None,
            separator: None,
            case_sensitive: None,
            index_empty: None,
            vector_options: None,
            missing_field_policy: None,
        }];
        assert!(service.validate_schema(&negative_weight_schema).is_err());
    }

    #[test]
    fn test_validate_key() {
        let service = create_service();

        assert!(service.validate_key("mykey").is_ok());
        assert!(service.validate_key("suggest:products").is_ok());

        assert!(service.validate_key("").is_err());
    }

    #[test]
    fn test_validate_dict_name() {
        let service = create_service();

        assert!(service.validate_dict_name("mydict").is_ok());
        assert!(service.validate_dict_name("custom_dict").is_ok());

        assert!(service.validate_dict_name("").is_err());
    }

    #[tokio::test]
    async fn test_search_service_operations() {
        let service = create_mock_service();
        let field = SearchFieldSchema {
            name: "title".to_string(),
            alias: None,
            field_type: SearchFieldType::Text,
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
            missing_field_policy: None,
        };

        let result = service
            .ft_create("idx", IndexCreateOptions::default(), vec![field.clone()])
            .await;
        assert!(result.is_ok());

        let result = service.ft_drop_index("idx", true).await;
        assert!(result.is_ok());

        let result = service.ft_info("idx").await;
        assert!(result.is_ok());

        let result = service.ft_list().await;
        assert!(result.is_ok());

        let result = service.ft_alter("idx", field.clone()).await;
        assert!(result.is_ok());

        let result = service
            .ft_search("idx", "*", SearchOptions::default())
            .await;
        assert!(result.is_ok());

        let result = service
            .ft_aggregate("idx", "*", AggregateOptions::default())
            .await;
        assert!(result.is_ok());

        let result = service.ft_explain("idx", "*", Some(1)).await;
        assert!(result.is_ok());

        let result = service
            .ft_profile(
                "idx",
                ProfileType::Search,
                false,
                "*",
                Some(SearchOptions::default()),
                Some(AggregateOptions::default()),
            )
            .await;
        assert!(result.is_ok());

        let result = service.ft_aliasadd("alias", "idx").await;
        assert!(result.is_ok());

        let result = service.ft_aliasdel("alias").await;
        assert!(result.is_ok());

        let result = service.ft_aliasupdate("alias", "idx").await;
        assert!(result.is_ok());

        let result = service
            .ft_sugadd("dict", "term", 1.0, SugAddOptions::default())
            .await;
        assert!(result.is_ok());

        let result = service
            .ft_sugget(
                "dict",
                "te",
                SugGetOptions {
                    fuzzy: true,
                    withscores: true,
                    withpayloads: true,
                    max: Some(5),
                },
            )
            .await;
        assert!(result.is_ok());

        let result = service.ft_sugdel("dict", "term").await;
        assert!(result.is_ok());

        let result = service.ft_suglen("dict").await;
        assert!(result.is_ok());

        let result = service.ft_syndump("idx").await;
        assert!(result.is_ok());

        let result = service
            .ft_synupdate("idx", "1", true, vec!["term".to_string()])
            .await;
        assert!(result.is_ok());

        let result = service
            .ft_spellcheck(
                "idx",
                "hello",
                SpellcheckOptions {
                    distance: Some(1),
                    include: Some("dict".to_string()),
                    exclude: None,
                    dialect: Some(2),
                },
            )
            .await;
        assert!(result.is_ok());

        let result = service
            .ft_dictadd("dict", vec!["a".to_string(), "b".to_string()])
            .await;
        assert!(result.is_ok());

        let result = service
            .ft_dictdel("dict", vec!["a".to_string()])
            .await;
        assert!(result.is_ok());

        let result = service.ft_dictdump("dict").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_service_validation_errors() {
        let service = create_mock_service();
        let long_alias = "a".repeat(129);

        let result = service.ft_aliasadd("", "idx").await;
        assert!(result.is_err());

        let result = service.ft_aliasadd(&long_alias, "idx").await;
        assert!(result.is_err());

        let result = service
            .ft_sugadd("dict", "", 1.0, SugAddOptions::default())
            .await;
        assert!(result.is_err());

        let result = service.ft_synupdate("idx", "", false, vec![]).await;
        assert!(result.is_err());

        let result = service.ft_synupdate("idx", "1", false, vec![]).await;
        assert!(result.is_err());

        let result = service.ft_dictadd("dict", vec![]).await;
        assert!(result.is_err());

        let result = service.ft_dictdel("dict", vec![]).await;
        assert!(result.is_err());
    }
}
