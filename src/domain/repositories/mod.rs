//! Repository Traits
//!
//! Abstract interfaces for data access.

mod admin_repository;
mod hash_repository;
mod key_repository;
mod list_repository;
mod set_repository;
mod sorted_set_repository;
mod stream_repository;
mod string_repository;

pub use admin_repository::AdminRepository;
pub use hash_repository::HashRepository;
pub use key_repository::KeyRepository;
pub use list_repository::{
    BlockingPopResult, InsertPosition, ListDirection, ListRepository, LPosOptions,
};
pub use set_repository::{SetRepository, SetScanResult};
pub use sorted_set_repository::{
    LexRange, ScoreRange, ScoredMember, SortedSetRepository, ZAddOptions, ZAddResult,
    ZAggregate, ZPopDirection, ZPopResult, ZRangeOptions, ZRangeType, ZScanResult,
    ZSetAlgebraOptions,
};
pub use stream_repository::StreamRepository;
pub use string_repository::StringRepository;
