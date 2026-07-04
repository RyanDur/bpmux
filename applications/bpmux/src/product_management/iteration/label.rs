use crate::product_management::iteration::IterationId;
use crate::product_management::label_codec::LabelCodec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterationLabelParts {
    pub name: String,
    pub position: u32,
}

pub fn iteration_name_from_title(title: &str) -> String {
    let raw = title.strip_prefix("Iteration ").unwrap_or(title).trim();
    raw.split_whitespace().collect::<Vec<_>>().join("-")
}

pub fn parse_iteration_label<C: LabelCodec>(codec: &C, label: &str) -> Option<IterationLabelParts> {
    codec.decode_iteration(label)
}

pub const DEFAULT_FIRST_POSITION: u32 = 100;
pub const DEFAULT_STEP: u32 = 100;

pub fn compute_insert_position(insert_index: usize, existing_positions: &[u32]) -> u32 {
    let mut positions = existing_positions.to_vec();
    positions.sort_unstable();
    if positions.is_empty() {
        return DEFAULT_FIRST_POSITION;
    }
    if insert_index == 0 {
        let first = positions[0];
        if first > 1 {
            return first / 2;
        }
        return 0;
    }
    if insert_index >= positions.len() {
        let last = positions[positions.len() - 1];
        return last + DEFAULT_STEP;
    }
    let left = positions[insert_index - 1];
    let right = positions[insert_index];
    let gap = right.saturating_sub(left);
    if gap >= 2 {
        return left + gap / 2;
    }
    left
}

pub fn current_iteration_id(iterations: &[super::Iteration]) -> Option<IterationId> {
    iterations
        .iter()
        .find(|iteration| !iteration.complete && iteration.is_current)
        .map(|iteration| iteration.id)
}
