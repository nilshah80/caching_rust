//! Repository Traits
//!
//! Abstract interfaces for data access.

mod admin_repository;
mod string_repository;

pub use admin_repository::AdminRepository;
pub use string_repository::StringRepository;
