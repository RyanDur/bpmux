pub mod adapters;
pub mod sync;

#[cfg(test)]
mod canonical_tests;

pub use sync::{SourceAdapter, SyncError};
