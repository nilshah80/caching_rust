//! Domain Errors
//!
//! Error types for the caching service.

mod cache_error;

pub use cache_error::{CacheError, ErrorDetail, ErrorResponse};
