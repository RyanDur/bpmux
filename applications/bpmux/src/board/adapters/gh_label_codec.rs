//! GitHub-backed `LabelCodec` impl (#1350).
//!
//! Owns the GH-specific label syntax: `ready-order.{N}`,
//! `iteration-{Theme}.{N}`, `parent-epic-#{NNNN}`, `unscheduled-{N}`.
//! Domain and bridge code routes label encode/decode through this
//! adapter so the syntax can be swapped (in tests, or in a future
//! non-GH backend) without rewriting consumers.

use crate::board::label_codec::LabelCodec;

pub struct GhLabelCodec;

impl LabelCodec for GhLabelCodec {
    fn decode_ready_order(&self, label: &str) -> Option<u32> {
        label.strip_prefix("ready-order.")?.parse().ok()
    }
}
