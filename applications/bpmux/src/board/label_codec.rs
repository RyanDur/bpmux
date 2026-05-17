//! Port for label-encoding/decoding (#1350).
//!
//! Label syntax (`ready-order.{N}`, `iteration-{Theme}.{N}`,
//! `parent-epic-#{NNNN}`, `unscheduled-{N}`) is GH-specific substrate
//! and should not leak into domain or bridge logic. This trait
//! abstracts the encode/decode operations so:
//!
//! - Domain code can be tested against a stub codec.
//! - Alternative repo-backends (GitLab, in-memory) can supply their
//!   own encoding without rewriting consumers.

use crate::board::iteration_label::IterationLabelParts;

pub trait LabelCodec {
    fn decode_ready_order(&self, label: &str) -> Option<u32>;

    fn is_ready_order_label(&self, label: &str) -> bool {
        self.decode_ready_order(label).is_some()
    }

    fn encode_ready_order(&self, position: u32) -> String;

    fn decode_iteration(&self, label: &str) -> Option<IterationLabelParts>;

    fn is_iteration_label(&self, label: &str) -> bool {
        self.decode_iteration(label).is_some()
    }

    /// Looser predicate than `is_iteration_label`: returns true for
    /// any label that looks like a member-position iteration label
    /// (has the iteration prefix AND embeds a position-separator),
    /// even if the position is non-numeric.
    fn is_iteration_label_loose(&self, label: &str) -> bool;

    fn encode_iteration(&self, theme_name: &str, position: u32) -> String;

    /// Returns the iteration-prefix class of a member-position label,
    /// without the trailing position digits. Used by the orchestrator
    /// coalescer to detect "same iteration class, different position"
    /// updates and absorb them into a single tag-remove + tag-add
    /// pair on the outbox.
    fn iteration_label_class(&self, label: &str) -> Option<String>;

    fn encode_parent_epic(&self, issue_number: u64) -> String;
}
