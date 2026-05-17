//! GitHub-backed `LabelCodec` impl (#1350).
//!
//! Owns the GH-specific label syntax: `ready-order.{N}`,
//! `iteration-{Theme}.{N}`, `parent-epic-#{NNNN}`, `unscheduled-{N}`.
//! Domain and bridge code routes label encode/decode through this
//! adapter so the syntax can be swapped (in tests, or in a future
//! non-GH backend) without rewriting consumers.

use crate::board::iteration_label::IterationLabelParts;
use crate::board::label_codec::LabelCodec;

pub struct GhLabelCodec;

impl LabelCodec for GhLabelCodec {
    fn decode_ready_order(&self, label: &str) -> Option<u32> {
        label.strip_prefix("ready-order.")?.parse().ok()
    }

    fn decode_iteration(&self, label: &str) -> Option<IterationLabelParts> {
        let rest = label.strip_prefix("iteration-")?;
        let dot_idx = rest.rfind('.')?;
        let (name, dotted_pos) = rest.split_at(dot_idx);
        let position = dotted_pos.trim_start_matches('.').parse::<u32>().ok()?;
        if name.is_empty() {
            return None;
        }
        Some(IterationLabelParts {
            theme_name: name.to_string(),
            position,
        })
    }
}
