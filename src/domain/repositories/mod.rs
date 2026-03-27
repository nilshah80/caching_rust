//! Repository Traits
//!
//! Abstract interfaces for data access.

mod admin_repository;
mod bitmap_repository;
mod bloom_repository;
mod geo_repository;
mod hash_repository;
mod json_repository;
mod key_repository;
mod list_repository;
mod probabilistic_repository;
mod pubsub_repository;
mod search_repository;
mod set_repository;
mod sorted_set_repository;
mod stream_repository;
mod string_repository;

pub use admin_repository::AdminRepository;
pub use bitmap_repository::{
    BitMapRepository, BitOperation, BitfieldCommand, BitfieldEncoding, BitfieldOverflow,
    BitfieldResult,
};
pub use bloom_repository::BloomRepository;
pub use geo_repository::{
    GeoAddOptions, GeoAddResult, GeoMember, GeoPosition, GeoRepository, GeoSearchCenter,
    GeoSearchOptions, GeoSearchResult, GeoSearchShape, GeoSearchStoreResult, GeoSortOrder, GeoUnit,
};
pub use hash_repository::{ExpireCondition, HashRepository};
pub use json_repository::JsonRepository;
pub use key_repository::{KeyRepository, SortOptions, SortOrder};
pub use list_repository::{
    BlockingPopResult, InsertPosition, LMPopResult, LPosOptions, ListDirection, ListRepository,
};
pub use probabilistic_repository::ProbabilisticRepository;
pub use pubsub_repository::{NumSubResult, PubSubRepository, PublishResult};
pub use search_repository::SearchRepository;
pub use set_repository::{SetRepository, SetScanResult};
pub use sorted_set_repository::{
    LexRange, ScoreRange, ScoredMember, SortedSetRepository, ZAddOptions, ZAddResult, ZAggregate,
    ZPopDirection, ZPopResult, ZRangeOptions, ZRangeType, ZScanResult, ZSetAlgebraOptions,
};
pub use stream_repository::StreamRepository;
pub use string_repository::{LcsMatch, LcsMatchResult, LcsOptions, LcsResult, StringRepository};
