//! HTTP Middleware
//!
//! Custom middleware for request processing.

pub mod admin_auth;
mod error_handler;
mod logging;
pub mod metrics;
pub mod rate_limit;
mod request_id;

pub use admin_auth::ADMIN_API_KEY_HEADER;
pub use error_handler::error_handler;
pub use logging::logging_middleware;
pub use metrics::metrics_middleware;
pub use request_id::request_id_middleware;
