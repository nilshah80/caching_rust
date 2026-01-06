//! Repository Traits
//!
//! Abstract interfaces for data access.

mod admin_repository;
mod bloom_repository;
mod hash_repository;
mod json_repository;
mod key_repository;
mod list_repository;
mod probabilistic_repository;
mod search_repository;
mod set_repository;
mod sorted_set_repository;
mod stream_repository;
mod string_repository;

pub use admin_repository::AdminRepository;
pub use bloom_repository::BloomRepository;
pub use hash_repository::HashRepository;
pub use json_repository::JsonRepository;
pub use key_repository::KeyRepository;
pub use list_repository::{
    BlockingPopResult, InsertPosition, ListDirection, ListRepository, LPosOptions,
};
pub use probabilistic_repository::ProbabilisticRepository;
pub use search_repository::SearchRepository;
pub use set_repository::{SetRepository, SetScanResult};
pub use sorted_set_repository::{
    LexRange, ScoreRange, ScoredMember, SortedSetRepository, ZAddOptions, ZAddResult,
    ZAggregate, ZPopDirection, ZPopResult, ZRangeOptions, ZRangeType, ZScanResult,
    ZSetAlgebraOptions,
};
pub use stream_repository::StreamRepository;
pub use string_repository::StringRepository;
