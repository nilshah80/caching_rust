//! Application Services
//!
//! Service layer containing business logic for each domain.

mod admin_service;
mod string_service;

pub use admin_service::AdminService;
pub use string_service::StringService;
