//! Bridge tracker module — composition shell. Decomposed in #1416
//! per the #1411 INVESTIGATE layout: `tracker/trait_def.rs` holds
//! the `StoryTracker` trait + cross-tracker types
//! (`WorkflowRunEntry`, `ConditionalIssue`, `ConditionalRunEntry`),
//! plus the shared `log_labels_fetch_failure` helper and the
//! `ensure_crypto_provider_installed` re-export. `tracker/octocrab.rs`
//! holds the sole production `OctocrabStoryTracker` impl plus sister
//! env-decisioning helpers and the REST/CI/JSON glue. External call-
//! sites (37 caller-files) reach trait + impl through the re-exports
//! below; the `bridge::tracker::*` path stays stable across the
//! decompose.

mod octocrab;
mod trait_def;

pub use octocrab::OctocrabStoryTracker;
pub use trait_def::{ConditionalIssue, ConditionalRunEntry, StoryTracker, WorkflowRunEntry};

#[cfg(test)]
pub(crate) use octocrab::{FallbackReason, TrackerSelection, select_from_env};
#[cfg(test)]
pub(crate) use trait_def::{ensure_crypto_provider_installed, log_labels_fetch_failure};
