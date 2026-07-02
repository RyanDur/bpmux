use crate::product_management::board::label_codec::LabelCodec;
use crate::product_management::iteration::label::IterationLabelParts;

mod iteration;
mod outcome;
mod story;

pub struct GhLabelCodec;

impl LabelCodec for GhLabelCodec {
    fn decode_ready_order(&self, label: &str) -> Option<u32> {
        label.strip_prefix("ready-order.")?.parse().ok()
    }

    fn encode_ready_order(&self, position: u32) -> String {
        format!("ready-order.{position}")
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
            name: name.to_string(),
            position,
        })
    }

    fn is_iteration_label_loose(&self, label: &str) -> bool {
        label.starts_with("iteration-") && label.contains('.')
    }

    fn encode_iteration(&self, name: &str, position: u32) -> String {
        format!("iteration-{name}.{position}")
    }

    fn iteration_label_class(&self, label: &str) -> Option<String> {
        let parts = self.decode_iteration(label)?;
        Some(format!("iteration-{}.", parts.name))
    }

    fn encode_parent_epic(&self, issue_number: u64) -> String {
        format!("parent-epic-#{issue_number}")
    }
}

pub const CURRENT_ITERATION_LABEL: &str = "current-iteration";
pub const BLOCKED_LABEL: &str = "blocked";
pub const DESK_CHECK_FAIL_LABEL: &str = "desk-check-fail";
pub const BLOCKED_BY_PREFIX: &str = "blocked-by:";
pub const ACCEPTOR_TEAM_LABEL: &str = "awaiting team";
pub const ACCEPTOR_OWNER_LABEL: &str = "awaiting owner";
pub const ACCEPTOR_DEV_LABEL: &str = "awaiting dev";
