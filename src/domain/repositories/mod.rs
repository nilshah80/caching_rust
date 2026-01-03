//! Repository Traits
//!
//! Abstract interfaces for data access.

mod admin_repository;
mod hash_repository;
mod key_repository;
mod list_repository;
mod set_repository;
mod string_repository;

pub use admin_repository::AdminRepository;
pub use hash_repository::HashRepository;
pub use key_repository::KeyRepository;
pub use list_repository::{
    BlockingPopResult, InsertPosition, ListDirection, ListRepository, LPosOptions,
};
pub use set_repository::{SetRepository, SetScanResult};
pub use string_repository::StringRepository;
