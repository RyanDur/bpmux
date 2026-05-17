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
//!
//! Slice 2 (this commit) extends the surface from slice 1's
//! ready-order-only foundation to include iteration-label
//! decode/predicate, used by `board::iteration_label` and
//! `board::ready_order_label` after their migration to take a codec
//! parameter. Sister-slices 3 + 4 cover the remaining encode-side
//! leaks in `board::domain`, `board::orchestrator`,
//! `bridge::tools_stories`, and `lib`.

use crate::board::iteration_label::IterationLabelParts;

pub trait LabelCodec {
    fn decode_ready_order(&self, label: &str) -> Option<u32>;

    fn is_ready_order_label(&self, label: &str) -> bool {
        self.decode_ready_order(label).is_some()
    }

    fn decode_iteration(&self, label: &str) -> Option<IterationLabelParts>;

    fn is_iteration_label(&self, label: &str) -> bool {
        self.decode_iteration(label).is_some()
    }
}
