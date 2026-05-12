// Integration test crate root.
// Run with: cargo test --test integration --features test-utils

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod docker_helper;

mod core_features;
mod error_scenarios;
mod phase11_features;
mod phase5_features;
