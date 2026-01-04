//! Redis Search Repository Implementation
//!
//! Concrete implementation of SearchRepository using RediSearch module.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::entities::{
    AggregateOptions, AggregateResult, AggregateStep, AliasResult, DictDumpResult, DictResult,
    ExplainResult, IndexAlterResult, IndexCreateOptions, IndexCreateResult, IndexDropResult,
    IndexInfo, ProfileResult, ProfileType, SearchDocument, SearchFieldSchema, SearchFieldType,
    SearchOptions, SearchResult, SpellcheckOptions, SpellcheckResult, SpellcheckSuggestion,
    SpellcheckTerm, SugAddOptions, SugAddResult, SugDelResult, SugGetOptions, SugLenResult,
    Suggestion, SynonymGroup, SynonymUpdateResult,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::SearchRepository;
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Redis implementation of SearchRepository using RediSearch module
pub struct RedisSearchRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisSearchRepository {
    /// Create a new RedisSearchRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }

    /// Extract string from redis Value
    fn extract_string(value: &redis::Value) -> Option<String> {
        match value {
            redis::Value::BulkString(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
            redis::Value::SimpleString(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Extract i64 from redis Value
    fn extract_i64(value: &redis::Value) -> Option<i64> {
        match value {
            redis::Value::Int(i) => Some(*i),
            redis::Value::BulkString(bytes) => {
                String::from_utf8_lossy(bytes).parse().ok()
            }
            _ => None,
        }
    }

    /// Extract f64 from redis Value
    fn extract_f64(value: &redis::Value) -> Option<f64> {
        match value {
            redis::Value::Int(i) => Some(*i as f64),
            redis::Value::BulkString(bytes) => {
                String::from_utf8_lossy(bytes).parse().ok()
            }
            redis::Value::Double(d) => Some(*d),
            _ => None,
        }
    }

    /// Convert redis Value to serde_json Value
    fn to_json_value(value: redis::Value) -> serde_json::Value {
        match value {
            redis::Value::Nil => serde_json::Value::Null,
            redis::Value::Int(i) => serde_json::Value::Number(i.into()),
            redis::Value::Double(d) => {
                serde_json::Number::from_f64(d)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            redis::Value::BulkString(bytes) => {
                let s = String::from_utf8_lossy(&bytes).to_string();
                // Try to parse as JSON first
                serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s))
            }
            redis::Value::SimpleString(s) => serde_json::Value::String(s),
            redis::Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Self::to_json_value).collect())
            }
            redis::Value::Map(map) => {
                let obj: serde_json::Map<String, serde_json::Value> = map
                    .into_iter()
                    .filter_map(|(k, v)| {
                        Self::extract_string(&k).map(|key| (key, Self::to_json_value(v)))
                    })
                    .collect();
                serde_json::Value::Object(obj)
            }
            _ => serde_json::Value::Null,
        }
    }

    /// Build field schema arguments for FT.CREATE
    fn build_field_args(field: &SearchFieldSchema) -> Vec<String> {
        let mut args = Vec::new();

        // Field name
        args.push(field.name.clone());

        // Alias
        if let Some(alias) = &field.alias {
            args.push("AS".to_string());
            args.push(alias.clone());
        }

        // Field type
        args.push(field.field_type.to_string());

        // Type-specific options
        match field.field_type {
            SearchFieldType::Text => {
                if let Some(weight) = field.weight {
                    args.push("WEIGHT".to_string());
                    args.push(weight.to_string());
                }
                if field.no_stem == Some(true) {
                    args.push("NOSTEM".to_string());
                }
                if let Some(phonetic) = &field.phonetic {
                    args.push("PHONETIC".to_string());
                    args.push(phonetic.to_string());
                }
            }
            SearchFieldType::Tag => {
                if let Some(sep) = &field.separator {
                    args.push("SEPARATOR".to_string());
                    args.push(sep.clone());
                }
                if field.case_sensitive == Some(true) {
                    args.push("CASESENSITIVE".to_string());
                }
                if field.index_empty == Some(true) {
                    args.push("INDEXEMPTY".to_string());
                }
            }
            SearchFieldType::Vector => {
                if let Some(opts) = &field.vector_options {
                    // Algorithm is required for VECTOR fields
                    let algo = opts.algorithm.unwrap_or(crate::domain::entities::VectorAlgorithm::Flat);
                    args.push(algo.to_string());

                    // Count parameters: TYPE is mandatory, DIM and DISTANCE_METRIC are required
                    // TYPE is always added (2 args), plus optional params
                    let mut param_count = 2; // TYPE FLOAT32 (mandatory)

                    // DIM is required but we'll use a default if not provided
                    param_count += 2;

                    // DISTANCE_METRIC is required but we'll use a default if not provided
                    param_count += 2;

                    if opts.initial_cap.is_some() {
                        param_count += 2;
                    }
                    if opts.block_size.is_some() {
                        param_count += 2;
                    }
                    if opts.m.is_some() {
                        param_count += 2;
                    }
                    if opts.ef_construction.is_some() {
                        param_count += 2;
                    }
                    if opts.ef_runtime.is_some() {
                        param_count += 2;
                    }
                    args.push(param_count.to_string());

                    // TYPE is mandatory - default to FLOAT32
                    args.push("TYPE".to_string());
                    args.push("FLOAT32".to_string());

                    // DIM is required - use provided or default to 128
                    args.push("DIM".to_string());
                    args.push(opts.dim.unwrap_or(128).to_string());

                    // DISTANCE_METRIC is required - use provided or default to L2
                    args.push("DISTANCE_METRIC".to_string());
                    args.push(opts.distance_metric.as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| "L2".to_string()));

                    if let Some(cap) = opts.initial_cap {
                        args.push("INITIAL_CAP".to_string());
                        args.push(cap.to_string());
                    }
                    if let Some(bs) = opts.block_size {
                        args.push("BLOCK_SIZE".to_string());
                        args.push(bs.to_string());
                    }
                    if let Some(m) = opts.m {
                        args.push("M".to_string());
                        args.push(m.to_string());
                    }
                    if let Some(ef) = opts.ef_construction {
                        args.push("EF_CONSTRUCTION".to_string());
                        args.push(ef.to_string());
                    }
                    if let Some(ef) = opts.ef_runtime {
                        args.push("EF_RUNTIME".to_string());
                        args.push(ef.to_string());
                    }
                }
            }
            _ => {}
        }

        // Common options
        if field.sortable {
            args.push("SORTABLE".to_string());
            if field.unf {
                args.push("UNF".to_string());
            }
        }
        if field.no_index {
            args.push("NOINDEX".to_string());
        }

        args
    }

    /// Build search options arguments
    fn build_search_args(options: &SearchOptions) -> Vec<String> {
        let mut args = Vec::new();

        if options.nocontent {
            args.push("NOCONTENT".to_string());
        }
        if options.verbatim {
            args.push("VERBATIM".to_string());
        }
        if options.nostopwords {
            args.push("NOSTOPWORDS".to_string());
        }
        if options.withscores {
            args.push("WITHSCORES".to_string());
        }
        if options.withpayloads {
            args.push("WITHPAYLOADS".to_string());
        }
        if options.withsortkeys {
            args.push("WITHSORTKEYS".to_string());
        }

        // Numeric filters
        for filter in &options.filters {
            args.push("FILTER".to_string());
            args.push(filter.field.clone());

            // For min: exclusive only applies when there's an actual bound value
            // -inf cannot be exclusive in RediSearch syntax
            let min = match (filter.min, filter.exclusive_min) {
                (Some(v), true) => format!("({}", v),
                (Some(v), false) => v.to_string(),
                (None, _) => "-inf".to_string(),
            };

            // For max: exclusive only applies when there's an actual bound value
            // +inf cannot be exclusive in RediSearch syntax
            let max = match (filter.max, filter.exclusive_max) {
                (Some(v), true) => format!("({}", v),
                (Some(v), false) => v.to_string(),
                (None, _) => "+inf".to_string(),
            };

            args.push(min);
            args.push(max);
        }

        // Geo filter
        if let Some(geo) = &options.geofilter {
            args.push("GEOFILTER".to_string());
            args.push(geo.field.clone());
            args.push(geo.lon.to_string());
            args.push(geo.lat.to_string());
            args.push(geo.radius.to_string());
            args.push(geo.unit.clone());
        }

        // In keys
        if !options.inkeys.is_empty() {
            args.push("INKEYS".to_string());
            args.push(options.inkeys.len().to_string());
            args.extend(options.inkeys.clone());
        }

        // In fields
        if !options.infields.is_empty() {
            args.push("INFIELDS".to_string());
            args.push(options.infields.len().to_string());
            args.extend(options.infields.clone());
        }

        // Return fields
        if !options.return_fields.is_empty() {
            args.push("RETURN".to_string());
            args.push(options.return_fields.len().to_string());
            args.extend(options.return_fields.clone());
        }

        // Summarize
        if let Some(sum) = &options.summarize {
            args.push("SUMMARIZE".to_string());
            if !sum.fields.is_empty() {
                args.push("FIELDS".to_string());
                args.push(sum.fields.len().to_string());
                args.extend(sum.fields.clone());
            }
            if let Some(frags) = sum.frags {
                args.push("FRAGS".to_string());
                args.push(frags.to_string());
            }
            if let Some(len) = sum.len {
                args.push("LEN".to_string());
                args.push(len.to_string());
            }
            if let Some(sep) = &sum.separator {
                args.push("SEPARATOR".to_string());
                args.push(sep.clone());
            }
        }

        // Highlight
        if let Some(hl) = &options.highlight {
            args.push("HIGHLIGHT".to_string());
            if !hl.fields.is_empty() {
                args.push("FIELDS".to_string());
                args.push(hl.fields.len().to_string());
                args.extend(hl.fields.clone());
            }
            if let (Some(open), Some(close)) = (&hl.open_tag, &hl.close_tag) {
                args.push("TAGS".to_string());
                args.push(open.clone());
                args.push(close.clone());
            }
        }

        // Slop
        if let Some(slop) = options.slop {
            args.push("SLOP".to_string());
            args.push(slop.to_string());
        }

        // Timeout
        if let Some(timeout) = options.timeout {
            args.push("TIMEOUT".to_string());
            args.push(timeout.to_string());
        }

        // In order
        if options.inorder {
            args.push("INORDER".to_string());
        }

        // Language
        if let Some(lang) = &options.language {
            args.push("LANGUAGE".to_string());
            args.push(lang.clone());
        }

        // Scorer
        if let Some(scorer) = &options.scorer {
            args.push("SCORER".to_string());
            args.push(scorer.clone());
        }

        // Explain score
        if options.explainscore {
            args.push("EXPLAINSCORE".to_string());
        }

        // Sort by
        if let Some(sort) = &options.sortby {
            args.push("SORTBY".to_string());
            args.push(sort.field.clone());
            args.push(sort.order.to_string());
        }

        // Limit
        args.push("LIMIT".to_string());
        args.push(options.offset.to_string());
        args.push(options.limit.to_string());

        // Params
        if !options.params.is_empty() {
            args.push("PARAMS".to_string());
            args.push((options.params.len() * 2).to_string());
            for (k, v) in &options.params {
                args.push(k.clone());
                args.push(v.clone());
            }
        }

        // Dialect
        if let Some(dialect) = options.dialect {
            args.push("DIALECT".to_string());
            args.push(dialect.to_string());
        }

        args
    }

    /// Build aggregate options arguments
    fn build_aggregate_args(options: &AggregateOptions) -> Vec<String> {
        let mut args = Vec::new();

        if options.verbatim {
            args.push("VERBATIM".to_string());
        }

        // Load
        if options.load_all {
            args.push("LOAD".to_string());
            args.push("*".to_string());
        } else if !options.load.is_empty() {
            args.push("LOAD".to_string());
            args.push(options.load.len().to_string());
            args.extend(options.load.clone());
        }

        // Timeout
        if let Some(timeout) = options.timeout {
            args.push("TIMEOUT".to_string());
            args.push(timeout.to_string());
        }

        // Pipeline steps
        for step in &options.pipeline {
            match step {
                AggregateStep::Groupby(gb) => {
                    args.push("GROUPBY".to_string());
                    args.push(gb.fields.len().to_string());
                    args.extend(gb.fields.iter().map(|f| format!("@{}", f)));

                    for reducer in &gb.reducers {
                        args.push("REDUCE".to_string());
                        args.push(reducer.function.clone());
                        args.push(reducer.args.len().to_string());
                        args.extend(reducer.args.clone());
                        if let Some(alias) = &reducer.alias {
                            args.push("AS".to_string());
                            args.push(alias.clone());
                        }
                    }
                }
                AggregateStep::Sortby(sb) => {
                    args.push("SORTBY".to_string());
                    args.push((sb.fields.len() * 2).to_string());
                    for f in &sb.fields {
                        args.push(format!("@{}", f.field));
                        args.push(f.order.to_string());
                    }
                    if let Some(max) = sb.max {
                        args.push("MAX".to_string());
                        args.push(max.to_string());
                    }
                }
                AggregateStep::Apply(ap) => {
                    args.push("APPLY".to_string());
                    args.push(ap.expression.clone());
                    args.push("AS".to_string());
                    args.push(ap.alias.clone());
                }
                AggregateStep::Limit(lm) => {
                    args.push("LIMIT".to_string());
                    args.push(lm.offset.to_string());
                    args.push(lm.num.to_string());
                }
                AggregateStep::Filter(fl) => {
                    args.push("FILTER".to_string());
                    args.push(fl.expression.clone());
                }
            }
        }

        // Params
        if !options.params.is_empty() {
            args.push("PARAMS".to_string());
            args.push((options.params.len() * 2).to_string());
            for (k, v) in &options.params {
                args.push(k.clone());
                args.push(v.clone());
            }
        }

        // Dialect
        if let Some(dialect) = options.dialect {
            args.push("DIALECT".to_string());
            args.push(dialect.to_string());
        }

        args
    }

    /// Parse FT.INFO response into IndexInfo
    fn parse_index_info(result: redis::Value) -> Result<IndexInfo, CacheError> {
        let mut info = IndexInfo {
            index_name: String::new(),
            index_options: vec![],
            index_definition: HashMap::new(),
            attributes: vec![],
            num_docs: 0,
            max_doc_id: None,
            num_terms: 0,
            num_records: None,
            inverted_sz_mb: None,
            vector_index_sz_mb: None,
            total_inverted_index_blocks: None,
            offset_vectors_sz_mb: None,
            doc_table_size_mb: None,
            sortable_values_size_mb: None,
            key_table_size_mb: None,
            records_per_doc_avg: None,
            bytes_per_record_avg: None,
            offsets_per_term_avg: None,
            offset_bits_per_record_avg: None,
            indexing: false,
            percent_indexed: None,
            hash_indexing_failures: None,
            gc_stats: HashMap::new(),
            cursor_stats: HashMap::new(),
        };

        if let redis::Value::Array(arr) = result {
            let mut iter = arr.into_iter();
            while let Some(key) = iter.next() {
                let key_str = Self::extract_string(&key).unwrap_or_default();
                if let Some(val) = iter.next() {
                    match key_str.as_str() {
                        "index_name" => {
                            info.index_name = Self::extract_string(&val).unwrap_or_default()
                        }
                        "index_options" => {
                            if let redis::Value::Array(opts) = val {
                                info.index_options = opts
                                    .iter()
                                    .filter_map(Self::extract_string)
                                    .collect();
                            }
                        }
                        "index_definition" => {
                            if let redis::Value::Array(def) = val {
                                let mut def_iter = def.into_iter();
                                while let Some(k) = def_iter.next() {
                                    if let Some(v) = def_iter.next() {
                                        if let Some(key) = Self::extract_string(&k) {
                                            info.index_definition.insert(key, Self::to_json_value(v));
                                        }
                                    }
                                }
                            }
                        }
                        "attributes" => {
                            if let redis::Value::Array(attrs) = val {
                                for attr in attrs {
                                    if let redis::Value::Array(attr_arr) = attr {
                                        let mut attr_map = HashMap::new();
                                        let mut attr_iter = attr_arr.into_iter();
                                        while let Some(k) = attr_iter.next() {
                                            if let Some(v) = attr_iter.next() {
                                                if let Some(key) = Self::extract_string(&k) {
                                                    attr_map.insert(key, Self::to_json_value(v));
                                                }
                                            }
                                        }
                                        info.attributes.push(attr_map);
                                    }
                                }
                            }
                        }
                        "num_docs" => info.num_docs = Self::extract_i64(&val).unwrap_or(0) as u64,
                        "max_doc_id" => info.max_doc_id = Self::extract_i64(&val).map(|v| v as u64),
                        "num_terms" => info.num_terms = Self::extract_i64(&val).unwrap_or(0) as u64,
                        "num_records" => {
                            info.num_records = Self::extract_i64(&val).map(|v| v as u64)
                        }
                        "inverted_sz_mb" => info.inverted_sz_mb = Self::extract_f64(&val),
                        "vector_index_sz_mb" => info.vector_index_sz_mb = Self::extract_f64(&val),
                        "total_inverted_index_blocks" => {
                            info.total_inverted_index_blocks =
                                Self::extract_i64(&val).map(|v| v as u64)
                        }
                        "offset_vectors_sz_mb" => info.offset_vectors_sz_mb = Self::extract_f64(&val),
                        "doc_table_size_mb" => info.doc_table_size_mb = Self::extract_f64(&val),
                        "sortable_values_size_mb" => {
                            info.sortable_values_size_mb = Self::extract_f64(&val)
                        }
                        "key_table_size_mb" => info.key_table_size_mb = Self::extract_f64(&val),
                        "records_per_doc_avg" => info.records_per_doc_avg = Self::extract_f64(&val),
                        "bytes_per_record_avg" => info.bytes_per_record_avg = Self::extract_f64(&val),
                        "offsets_per_term_avg" => info.offsets_per_term_avg = Self::extract_f64(&val),
                        "offset_bits_per_record_avg" => {
                            info.offset_bits_per_record_avg = Self::extract_f64(&val)
                        }
                        "indexing" => {
                            info.indexing = Self::extract_i64(&val).map(|v| v != 0).unwrap_or(false)
                        }
                        "percent_indexed" => info.percent_indexed = Self::extract_f64(&val),
                        "hash_indexing_failures" => {
                            info.hash_indexing_failures = Self::extract_i64(&val).map(|v| v as u64)
                        }
                        "gc_stats" => {
                            if let redis::Value::Array(stats) = val {
                                let mut stats_iter = stats.into_iter();
                                while let Some(k) = stats_iter.next() {
                                    if let Some(v) = stats_iter.next() {
                                        if let Some(key) = Self::extract_string(&k) {
                                            info.gc_stats.insert(key, Self::to_json_value(v));
                                        }
                                    }
                                }
                            }
                        }
                        "cursor_stats" => {
                            if let redis::Value::Array(stats) = val {
                                let mut stats_iter = stats.into_iter();
                                while let Some(k) = stats_iter.next() {
                                    if let Some(v) = stats_iter.next() {
                                        if let Some(key) = Self::extract_string(&k) {
                                            info.cursor_stats.insert(key, Self::to_json_value(v));
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(info)
    }

    /// Parse FT.SEARCH response
    fn parse_search_result(result: redis::Value, options: &SearchOptions) -> SearchResult {
        let mut search_result = SearchResult {
            total_results: 0,
            documents: vec![],
        };

        if let redis::Value::Array(arr) = result {
            let mut iter = arr.into_iter();

            // First element is total count
            if let Some(total) = iter.next() {
                search_result.total_results = Self::extract_i64(&total).unwrap_or(0) as u64;
            }

            // Rest are documents (id, [score], [payload], [sortkey], fields...)
            while let Some(id_val) = iter.next() {
                let id = Self::extract_string(&id_val).unwrap_or_default();

                let mut doc = SearchDocument {
                    id,
                    score: None,
                    payload: None,
                    sortkey: None,
                    fields: HashMap::new(),
                    score_explanation: None,
                };

                // Score (if WITHSCORES) - comes after ID regardless of NOCONTENT
                if options.withscores {
                    if let Some(score_val) = iter.next() {
                        doc.score = Self::extract_f64(&score_val);
                    }
                }

                // Payload (if WITHPAYLOADS) - comes after score regardless of NOCONTENT
                if options.withpayloads {
                    if let Some(payload_val) = iter.next() {
                        doc.payload = Self::extract_string(&payload_val);
                    }
                }

                // Sort key (if WITHSORTKEYS) - comes after payload regardless of NOCONTENT
                if options.withsortkeys {
                    if let Some(sortkey_val) = iter.next() {
                        doc.sortkey = Self::extract_string(&sortkey_val);
                    }
                }

                // Handle NOCONTENT - no fields array, just ID + optional score/payload/sortkey
                if options.nocontent {
                    search_result.documents.push(doc);
                    continue;
                }

                // Fields array (only present when NOT nocontent)
                if let Some(fields_val) = iter.next() {
                    if let redis::Value::Array(fields_arr) = fields_val {
                        let mut fields_iter = fields_arr.into_iter();
                        while let Some(k) = fields_iter.next() {
                            if let Some(v) = fields_iter.next() {
                                if let Some(key) = Self::extract_string(&k) {
                                    doc.fields.insert(key, Self::to_json_value(v));
                                }
                            }
                        }
                    }
                }

                search_result.documents.push(doc);
            }
        }

        search_result
    }

    /// Parse FT.AGGREGATE response
    fn parse_aggregate_result(result: redis::Value) -> AggregateResult {
        let mut agg_result = AggregateResult {
            total_results: 0,
            rows: vec![],
        };

        if let redis::Value::Array(arr) = result {
            let mut iter = arr.into_iter();

            // First element is total count
            if let Some(total) = iter.next() {
                agg_result.total_results = Self::extract_i64(&total).unwrap_or(0) as u64;
            }

            // Rest are rows
            for row_val in iter {
                if let redis::Value::Array(row_arr) = row_val {
                    let mut row = HashMap::new();
                    let mut row_iter = row_arr.into_iter();
                    while let Some(k) = row_iter.next() {
                        if let Some(v) = row_iter.next() {
                            if let Some(key) = Self::extract_string(&k) {
                                row.insert(key, Self::to_json_value(v));
                            }
                        }
                    }
                    agg_result.rows.push(row);
                }
            }
        }

        agg_result
    }

    /// Parse FT.SUGGET response
    fn parse_suggestions(result: redis::Value, options: &SugGetOptions) -> Vec<Suggestion> {
        let mut suggestions = vec![];

        if let redis::Value::Array(arr) = result {
            let mut iter = arr.into_iter();

            while let Some(string_val) = iter.next() {
                let string = Self::extract_string(&string_val).unwrap_or_default();
                let mut suggestion = Suggestion {
                    string,
                    score: None,
                    payload: None,
                };

                if options.withscores {
                    if let Some(score_val) = iter.next() {
                        suggestion.score = Self::extract_f64(&score_val);
                    }
                }

                if options.withpayloads {
                    if let Some(payload_val) = iter.next() {
                        suggestion.payload = Self::extract_string(&payload_val);
                    }
                }

                suggestions.push(suggestion);
            }
        }

        suggestions
    }

    /// Parse FT.SYNDUMP response
    fn parse_synonyms(result: redis::Value) -> Vec<SynonymGroup> {
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();

        if let redis::Value::Array(arr) = result {
            let mut iter = arr.into_iter();
            while let Some(term_val) = iter.next() {
                if let Some(group_ids_val) = iter.next() {
                    let term = Self::extract_string(&term_val).unwrap_or_default();
                    if let redis::Value::Array(group_ids) = group_ids_val {
                        for gid in group_ids {
                            if let Some(group_id) = Self::extract_string(&gid) {
                                groups.entry(group_id).or_default().push(term.clone());
                            }
                        }
                    }
                }
            }
        }

        groups
            .into_iter()
            .map(|(group_id, terms)| SynonymGroup { group_id, terms })
            .collect()
    }

    /// Parse FT.SPELLCHECK response
    fn parse_spellcheck(result: redis::Value) -> SpellcheckResult {
        let mut spellcheck_result = SpellcheckResult { results: vec![] };

        if let redis::Value::Array(arr) = result {
            for term_result in arr {
                if let redis::Value::Array(term_arr) = term_result {
                    let mut iter = term_arr.into_iter();

                    // Skip "TERM" marker
                    iter.next();

                    // Get term
                    let term = iter.next().and_then(|v| Self::extract_string(&v)).unwrap_or_default();

                    // Get suggestions
                    let mut suggestions = vec![];
                    if let Some(redis::Value::Array(sugg_arr)) = iter.next() {
                        for sugg in sugg_arr {
                            if let redis::Value::Array(sugg_pair) = sugg {
                                let mut sugg_iter = sugg_pair.into_iter();
                                let score = sugg_iter.next().and_then(|v| Self::extract_f64(&v)).unwrap_or(0.0);
                                let suggestion = sugg_iter.next().and_then(|v| Self::extract_string(&v)).unwrap_or_default();
                                suggestions.push(SpellcheckSuggestion { score, suggestion });
                            }
                        }
                    }

                    spellcheck_result.results.push(SpellcheckTerm { term, suggestions });
                }
            }
        }

        spellcheck_result
    }
}

#[async_trait]
impl SearchRepository for RedisSearchRepository {
    // ==================== Index Operations ====================

    async fn ft_create(
        &self,
        index: &str,
        options: &IndexCreateOptions,
        schema: &[SearchFieldSchema],
    ) -> Result<IndexCreateResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.CREATE");
        cmd.arg(index);

        // ON HASH/JSON
        cmd.arg("ON").arg(options.on.to_string());

        // Prefixes
        if !options.prefixes.is_empty() {
            cmd.arg("PREFIX").arg(options.prefixes.len());
            for prefix in &options.prefixes {
                cmd.arg(prefix);
            }
        }

        // Filter
        if let Some(filter) = &options.filter {
            cmd.arg("FILTER").arg(filter);
        }

        // Language
        if let Some(lang) = &options.language {
            cmd.arg("LANGUAGE").arg(lang);
        }

        // Language field
        if let Some(lang_field) = &options.language_field {
            cmd.arg("LANGUAGE_FIELD").arg(lang_field);
        }

        // Score
        if let Some(score) = options.score {
            cmd.arg("SCORE").arg(score);
        }

        // Score field
        if let Some(score_field) = &options.score_field {
            cmd.arg("SCORE_FIELD").arg(score_field);
        }

        // Payload field
        if let Some(payload_field) = &options.payload_field {
            cmd.arg("PAYLOAD_FIELD").arg(payload_field);
        }

        // Max text fields
        if options.maxtextfields == Some(true) {
            cmd.arg("MAXTEXTFIELDS");
        }

        // No offsets
        if options.no_offsets {
            cmd.arg("NOOFFSETS");
        }

        // Temporary
        if let Some(ttl) = options.temporary {
            cmd.arg("TEMPORARY").arg(ttl);
        }

        // No fields
        if options.no_fields {
            cmd.arg("NOFIELDS");
        }

        // No freqs
        if options.no_freqs {
            cmd.arg("NOFREQS");
        }

        // No highlight
        if options.no_hl {
            cmd.arg("NOHL");
        }

        // Skip initial scan
        if options.skip_initial_scan {
            cmd.arg("SKIPINITIALSCAN");
        }

        // Stopwords
        if !options.stopwords.is_empty() {
            cmd.arg("STOPWORDS").arg(options.stopwords.len());
            for word in &options.stopwords {
                cmd.arg(word);
            }
        }

        // Schema
        cmd.arg("SCHEMA");
        for field in schema {
            for arg in Self::build_field_args(field) {
                cmd.arg(&arg);
            }
        }

        let _: String = cmd.query_async(&mut conn).await?;

        Ok(IndexCreateResult {
            index: index.to_string(),
            success: true,
        })
    }

    async fn ft_drop_index(
        &self,
        index: &str,
        delete_docs: bool,
    ) -> Result<IndexDropResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.DROPINDEX");
        cmd.arg(index);

        if delete_docs {
            cmd.arg("DD");
        }

        let _: String = cmd.query_async(&mut conn).await?;

        Ok(IndexDropResult {
            index: index.to_string(),
            delete_docs,
            success: true,
        })
    }

    async fn ft_info(&self, index: &str) -> Result<IndexInfo, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("FT.INFO")
            .arg(index)
            .query_async(&mut conn)
            .await?;

        Self::parse_index_info(result)
    }

    async fn ft_list(&self) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("FT._LIST")
            .query_async(&mut conn)
            .await?;

        let indices = match result {
            redis::Value::Array(arr) => arr
                .iter()
                .filter_map(Self::extract_string)
                .collect(),
            _ => vec![],
        };

        Ok(indices)
    }

    async fn ft_alter(
        &self,
        index: &str,
        field: &SearchFieldSchema,
    ) -> Result<IndexAlterResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.ALTER");
        cmd.arg(index).arg("SCHEMA").arg("ADD");

        for arg in Self::build_field_args(field) {
            cmd.arg(&arg);
        }

        let _: String = cmd.query_async(&mut conn).await?;

        Ok(IndexAlterResult {
            index: index.to_string(),
            field: field.name.clone(),
            success: true,
        })
    }

    // ==================== Query Operations ====================

    async fn ft_search(
        &self,
        index: &str,
        query: &str,
        options: &SearchOptions,
    ) -> Result<SearchResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.SEARCH");
        cmd.arg(index).arg(query);

        for arg in Self::build_search_args(options) {
            cmd.arg(&arg);
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;

        Ok(Self::parse_search_result(result, options))
    }

    async fn ft_aggregate(
        &self,
        index: &str,
        query: &str,
        options: &AggregateOptions,
    ) -> Result<AggregateResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.AGGREGATE");
        cmd.arg(index).arg(query);

        for arg in Self::build_aggregate_args(options) {
            cmd.arg(&arg);
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;

        Ok(Self::parse_aggregate_result(result))
    }

    async fn ft_explain(
        &self,
        index: &str,
        query: &str,
        dialect: Option<u32>,
    ) -> Result<ExplainResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.EXPLAIN");
        cmd.arg(index).arg(query);

        if let Some(d) = dialect {
            cmd.arg("DIALECT").arg(d);
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;
        let plan = Self::extract_string(&result).unwrap_or_default();

        Ok(ExplainResult { plan })
    }

    async fn ft_profile(
        &self,
        index: &str,
        profile_type: ProfileType,
        limited: bool,
        query: &str,
        search_options: Option<&SearchOptions>,
        aggregate_options: Option<&AggregateOptions>,
    ) -> Result<ProfileResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.PROFILE");
        cmd.arg(index).arg(profile_type.to_string());

        if limited {
            cmd.arg("LIMITED");
        }

        cmd.arg("QUERY").arg(query);

        // Add search or aggregate options
        match profile_type {
            ProfileType::Search => {
                if let Some(opts) = search_options {
                    for arg in Self::build_search_args(opts) {
                        cmd.arg(&arg);
                    }
                }
            }
            ProfileType::Aggregate => {
                if let Some(opts) = aggregate_options {
                    for arg in Self::build_aggregate_args(opts) {
                        cmd.arg(&arg);
                    }
                }
            }
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;

        // Parse profile result - it's typically [results, profile_info]
        let (results, profile) = if let redis::Value::Array(arr) = result {
            let mut iter = arr.into_iter();
            let results = iter.next().map(Self::to_json_value).unwrap_or(serde_json::Value::Null);
            let profile_val = iter.next().unwrap_or(redis::Value::Nil);

            let mut profile_map = HashMap::new();
            if let redis::Value::Array(prof_arr) = profile_val {
                let mut prof_iter = prof_arr.into_iter();
                while let Some(k) = prof_iter.next() {
                    if let Some(v) = prof_iter.next() {
                        if let Some(key) = Self::extract_string(&k) {
                            profile_map.insert(key, Self::to_json_value(v));
                        }
                    }
                }
            }
            (results, profile_map)
        } else {
            (serde_json::Value::Null, HashMap::new())
        };

        Ok(ProfileResult { results, profile })
    }

    // ==================== Alias Operations ====================

    async fn ft_aliasadd(&self, alias: &str, index: &str) -> Result<AliasResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let _: String = redis::cmd("FT.ALIASADD")
            .arg(alias)
            .arg(index)
            .query_async(&mut conn)
            .await?;

        Ok(AliasResult {
            alias: alias.to_string(),
            index: index.to_string(),
            success: true,
        })
    }

    async fn ft_aliasdel(&self, alias: &str) -> Result<AliasResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let _: String = redis::cmd("FT.ALIASDEL")
            .arg(alias)
            .query_async(&mut conn)
            .await?;

        Ok(AliasResult {
            alias: alias.to_string(),
            index: String::new(),
            success: true,
        })
    }

    async fn ft_aliasupdate(&self, alias: &str, index: &str) -> Result<AliasResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let _: String = redis::cmd("FT.ALIASUPDATE")
            .arg(alias)
            .arg(index)
            .query_async(&mut conn)
            .await?;

        Ok(AliasResult {
            alias: alias.to_string(),
            index: index.to_string(),
            success: true,
        })
    }

    // ==================== Autocomplete Operations ====================

    async fn ft_sugadd(
        &self,
        key: &str,
        string: &str,
        score: f64,
        options: &SugAddOptions,
    ) -> Result<SugAddResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.SUGADD");
        cmd.arg(key).arg(string).arg(score);

        if options.incr {
            cmd.arg("INCR");
        }

        if let Some(payload) = &options.payload {
            cmd.arg("PAYLOAD").arg(payload);
        }

        let size: i64 = cmd.query_async(&mut conn).await?;

        Ok(SugAddResult {
            key: key.to_string(),
            size,
        })
    }

    async fn ft_sugget(
        &self,
        key: &str,
        prefix: &str,
        options: &SugGetOptions,
    ) -> Result<Vec<Suggestion>, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.SUGGET");
        cmd.arg(key).arg(prefix);

        if options.fuzzy {
            cmd.arg("FUZZY");
        }

        if options.withscores {
            cmd.arg("WITHSCORES");
        }

        if options.withpayloads {
            cmd.arg("WITHPAYLOADS");
        }

        if let Some(max) = options.max {
            cmd.arg("MAX").arg(max);
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;

        Ok(Self::parse_suggestions(result, options))
    }

    async fn ft_sugdel(&self, key: &str, string: &str) -> Result<SugDelResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let deleted: i64 = redis::cmd("FT.SUGDEL")
            .arg(key)
            .arg(string)
            .query_async(&mut conn)
            .await?;

        Ok(SugDelResult {
            key: key.to_string(),
            deleted: deleted == 1,
        })
    }

    async fn ft_suglen(&self, key: &str) -> Result<SugLenResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let size: i64 = redis::cmd("FT.SUGLEN")
            .arg(key)
            .query_async(&mut conn)
            .await?;

        Ok(SugLenResult {
            key: key.to_string(),
            size,
        })
    }

    // ==================== Synonym Operations ====================

    async fn ft_syndump(&self, index: &str) -> Result<Vec<SynonymGroup>, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("FT.SYNDUMP")
            .arg(index)
            .query_async(&mut conn)
            .await?;

        Ok(Self::parse_synonyms(result))
    }

    async fn ft_synupdate(
        &self,
        index: &str,
        group_id: &str,
        skip_initial_scan: bool,
        terms: &[String],
    ) -> Result<SynonymUpdateResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.SYNUPDATE");
        cmd.arg(index).arg(group_id);

        if skip_initial_scan {
            cmd.arg("SKIPINITIALSCAN");
        }

        for term in terms {
            cmd.arg(term);
        }

        let _: String = cmd.query_async(&mut conn).await?;

        Ok(SynonymUpdateResult {
            index: index.to_string(),
            group_id: group_id.to_string(),
            success: true,
        })
    }

    // ==================== Spellcheck Operations ====================

    async fn ft_spellcheck(
        &self,
        index: &str,
        query: &str,
        options: &SpellcheckOptions,
    ) -> Result<SpellcheckResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.SPELLCHECK");
        cmd.arg(index).arg(query);

        if let Some(distance) = options.distance {
            cmd.arg("DISTANCE").arg(distance);
        }

        if let Some(include) = &options.include {
            cmd.arg("TERMS").arg("INCLUDE").arg(include);
        }

        if let Some(exclude) = &options.exclude {
            cmd.arg("TERMS").arg("EXCLUDE").arg(exclude);
        }

        if let Some(dialect) = options.dialect {
            cmd.arg("DIALECT").arg(dialect);
        }

        let result: redis::Value = cmd.query_async(&mut conn).await?;

        Ok(Self::parse_spellcheck(result))
    }

    // ==================== Dictionary Operations ====================

    async fn ft_dictadd(&self, dict: &str, terms: &[String]) -> Result<DictResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.DICTADD");
        cmd.arg(dict);

        for term in terms {
            cmd.arg(term);
        }

        let count: i64 = cmd.query_async(&mut conn).await?;

        Ok(DictResult {
            dict: dict.to_string(),
            count,
        })
    }

    async fn ft_dictdel(&self, dict: &str, terms: &[String]) -> Result<DictResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let mut cmd = redis::cmd("FT.DICTDEL");
        cmd.arg(dict);

        for term in terms {
            cmd.arg(term);
        }

        let count: i64 = cmd.query_async(&mut conn).await?;

        Ok(DictResult {
            dict: dict.to_string(),
            count,
        })
    }

    async fn ft_dictdump(&self, dict: &str) -> Result<DictDumpResult, CacheError> {
        let mut conn = self.pool.get().await?;

        let result: redis::Value = redis::cmd("FT.DICTDUMP")
            .arg(dict)
            .query_async(&mut conn)
            .await?;

        let terms = match result {
            redis::Value::Array(arr) => arr
                .iter()
                .filter_map(Self::extract_string)
                .collect(),
            _ => vec![],
        };

        Ok(DictDumpResult {
            dict: dict.to_string(),
            terms,
        })
    }
}
