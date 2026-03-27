//! Redis Caching Service
//!
//! A high-performance Redis caching service built with Rust, providing comprehensive
//! Redis operations through a clean REST API interface.

// Allow common test patterns that clippy restricts in production code.
// Tests legitimately use unwrap/expect/panic for assertions and setup.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_in_result,
        clippy::indexing_slicing,
        clippy::unreadable_literal,
        clippy::approx_constant,
        clippy::if_same_then_else,
        clippy::field_reassign_with_default,
    )
)]

pub mod api;
pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod shared;

#[cfg(test)]
mod test_support;
