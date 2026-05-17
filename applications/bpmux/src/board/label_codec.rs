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
//! Slice 1 (this commit) covers only the decode/predicate surface
//! needed by `bridge::tools::list_ready`. Sister-slice (#1350 follow-
//! up) extends the trait to cover the remaining call-sites in
//! `domain.rs`, `orchestrator.rs`, `tools_stories.rs`, `lib.rs`, and
//! the dedicated `ready_order_label` + `iteration_label` modules.

pub trait LabelCodec {
    fn decode_ready_order(&self, label: &str) -> Option<u32>;

    fn is_ready_order_label(&self, label: &str) -> bool {
        self.decode_ready_order(label).is_some()
    }
}
