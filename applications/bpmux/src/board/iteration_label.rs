//! Symmetric Rust counterpart to the renderer's iteration-label helpers.
//!
//! `iteration-{ThemeName}.{position}` labels are the canonical-via-GH
//! substrate for iteration-membership and member-ordering (#1300).
//! Theme-name and label parsing must match the renderer's
//! `Iteration.tsx` helpers exactly — drift here means a label that
//! renders correctly may be invisible to the close-iteration walk
//! (or vice versa).
//!
//! Renderer references:
//!   - `themeNameFromTitle` → mirrored by [`theme_name_from_title`]
//!   - `parseIterationLabel` regex `^iteration-(.+)\.(\d+)$` →
//!     [`parse_iteration_label`]

use crate::board::domain::Story;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterationLabelParts {
    pub theme_name: String,
    pub position: u32,
}

pub fn theme_name_from_title(title: &str) -> String {
    let raw = title.strip_prefix("Iteration ").unwrap_or(title).trim();
    raw.split_whitespace().collect::<Vec<_>>().join("-")
}

pub fn parse_iteration_label(label: &str) -> Option<IterationLabelParts> {
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

pub fn story_iteration_label_for_theme(story: &Story, theme_name: &str) -> Option<String> {
    for raw in &story.raw_labels {
        if let Some(parts) = parse_iteration_label(raw)
            && parts.theme_name == theme_name
        {
            return Some(raw.clone());
        }
    }
    None
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

pub fn members_with_iteration_label(stories: &[Story], theme_name: &str) -> Vec<u64> {
    let mut members: Vec<(u64, u32)> = stories
        .iter()
        .filter(|s| !s.is_closed)
        .filter_map(|s| {
            for raw in &s.raw_labels {
                if let Some(parts) = parse_iteration_label(raw)
                    && parts.theme_name == theme_name
                {
                    return Some((s.number, parts.position));
                }
            }
            None
        })
        .collect();
    members.sort_by_key(|(_, pos)| *pos);
    members.into_iter().map(|(n, _)| n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};

    fn seed(number: u64, raw_labels: Vec<String>) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("member-{number}"),
            StoryState::Icebox,
            Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        s.raw_labels = raw_labels;
        s
    }

    #[test]
    fn theme_name_strips_iteration_prefix() {
        assert_eq!(theme_name_from_title("Iteration Pegasus"), "Pegasus");
    }

    #[test]
    fn theme_name_hyphenates_internal_whitespace() {
        assert_eq!(theme_name_from_title("Iteration Big Dipper"), "Big-Dipper");
    }

    #[test]
    fn theme_name_returns_trimmed_title_when_prefix_absent() {
        assert_eq!(theme_name_from_title("  Pegasus  "), "Pegasus");
    }

    #[test]
    fn parse_label_simple_position() {
        assert_eq!(
            parse_iteration_label("iteration-Pegasus.100"),
            Some(IterationLabelParts {
                theme_name: "Pegasus".to_string(),
                position: 100
            })
        );
    }

    #[test]
    fn parse_label_hyphenated_theme() {
        assert_eq!(
            parse_iteration_label("iteration-Big-Dipper.200"),
            Some(IterationLabelParts {
                theme_name: "Big-Dipper".to_string(),
                position: 200
            })
        );
    }

    #[test]
    fn parse_label_rejects_legacy_pegasus_format() {
        assert_eq!(parse_iteration_label("iteration-#1400"), None);
    }

    #[test]
    fn parse_label_rejects_nondigit_position() {
        assert_eq!(parse_iteration_label("iteration-Pegasus.foo"), None);
    }

    #[test]
    fn parse_label_rejects_missing_position() {
        assert_eq!(parse_iteration_label("iteration-Pegasus"), None);
    }

    #[test]
    fn members_walk_returns_only_matching_theme() {
        let stories = vec![
            seed(200, vec!["iteration-Pegasus.100".to_string()]),
            seed(201, vec!["iteration-Pegasus.200".to_string()]),
            seed(300, vec!["iteration-Cassiopeia.100".to_string()]),
        ];
        assert_eq!(
            members_with_iteration_label(&stories, "Pegasus"),
            vec![200, 201]
        );
    }

    #[test]
    fn members_walk_sorts_by_label_position_ascending() {
        let stories = vec![
            seed(200, vec!["iteration-Pegasus.300".to_string()]),
            seed(201, vec!["iteration-Pegasus.100".to_string()]),
            seed(202, vec!["iteration-Pegasus.200".to_string()]),
        ];
        assert_eq!(
            members_with_iteration_label(&stories, "Pegasus"),
            vec![201, 202, 200]
        );
    }

    #[test]
    fn members_walk_skips_closed_stories() {
        let mut s = seed(200, vec!["iteration-Pegasus.100".to_string()]);
        s.is_closed = true;
        let stories = vec![s];
        assert!(members_with_iteration_label(&stories, "Pegasus").is_empty());
    }

    #[test]
    fn story_iteration_label_returns_member_carried_label() {
        let s = seed(
            200,
            vec![
                "icebox".to_string(),
                "iteration-Pegasus.250".to_string(),
                "feature".to_string(),
            ],
        );
        assert_eq!(
            story_iteration_label_for_theme(&s, "Pegasus"),
            Some("iteration-Pegasus.250".to_string())
        );
    }

    #[test]
    fn story_iteration_label_returns_none_when_theme_absent() {
        let s = seed(200, vec!["iteration-Cassiopeia.100".to_string()]);
        assert_eq!(story_iteration_label_for_theme(&s, "Pegasus"), None);
    }

    #[test]
    fn compute_insert_position_empty_container_returns_default_first() {
        assert_eq!(compute_insert_position(0, &[]), DEFAULT_FIRST_POSITION);
        assert_eq!(compute_insert_position(5, &[]), DEFAULT_FIRST_POSITION);
    }

    #[test]
    fn compute_insert_position_at_head_returns_midpoint_below_first() {
        assert_eq!(compute_insert_position(0, &[100, 200, 300]), 50);
        assert_eq!(compute_insert_position(0, &[200]), 100);
    }

    #[test]
    fn compute_insert_position_at_tail_appends_default_step() {
        assert_eq!(
            compute_insert_position(3, &[100, 200, 300]),
            300 + DEFAULT_STEP
        );
        assert_eq!(
            compute_insert_position(99, &[100, 200, 300]),
            300 + DEFAULT_STEP
        );
    }

    #[test]
    fn compute_insert_position_between_existing_returns_midpoint() {
        assert_eq!(compute_insert_position(1, &[100, 300]), 200);
        assert_eq!(compute_insert_position(2, &[100, 200, 400]), 300);
    }

    #[test]
    fn compute_insert_position_cascade_when_gap_too_tight() {
        assert_eq!(compute_insert_position(1, &[100, 101]), 100);
    }

    #[test]
    fn compute_insert_position_sorts_input_before_computing() {
        assert_eq!(compute_insert_position(1, &[300, 100, 200]), 150);
    }
}
