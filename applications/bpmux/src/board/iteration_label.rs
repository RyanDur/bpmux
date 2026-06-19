use crate::board::domain::Story;
use crate::board::label_codec::LabelCodec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterationLabelParts {
    pub theme_name: String,
    pub position: u32,
}

pub fn theme_name_from_title(title: &str) -> String {
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

pub fn current_iteration_number(stories: &[Story]) -> Option<u64> {
    stories
        .iter()
        .find(|s| {
            !s.is_closed
                && s.is_current
                && s.story_type == crate::board::domain::StoryType::Iteration
        })
        .map(|s| s.number)
}

pub fn members_of_iteration(stories: &[Story], iteration_number: u64) -> Vec<u64> {
    let mut members: Vec<(u64, u32)> = stories
        .iter()
        .filter(|s| !s.is_closed)
        .filter_map(|s| match s.iteration_membership {
            Some((it, pos)) if it == iteration_number => Some((s.number, pos)),
            _ => None,
        })
        .collect();
    members.sort_by_key(|(_, pos)| *pos);
    members.into_iter().map(|(n, _)| n).collect()
}
