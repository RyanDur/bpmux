use crate::board::Story;
use crate::board::column::{ColumnBucket, column_for};

pub const VIEW_BACKLOG_MARKER: &str = "__BPMUX_VIEW_BACKLOG__";

const SAMPLE_LIMIT: usize = 5;

pub(in crate::bridge) fn build_backlog_summary(stories: &[Story]) -> serde_json::Value {
    let mut ready: Vec<String> = Vec::new();
    let mut in_progress: Vec<String> = Vec::new();
    let mut delivered: Vec<String> = Vec::new();
    let mut accepted: Vec<String> = Vec::new();
    let mut icebox: Vec<String> = Vec::new();
    let mut retired: Vec<String> = Vec::new();

    for story in stories {
        let entry = format!("#{} {}", story.number, story.title);
        match column_for(story.state, story.is_closed) {
            ColumnBucket::Icebox => icebox.push(entry),
            ColumnBucket::Ready => ready.push(entry),
            ColumnBucket::InProgress => in_progress.push(entry),
            ColumnBucket::DeskCheck | ColumnBucket::Delivered => delivered.push(entry),
            ColumnBucket::Accepted => accepted.push(entry),
            ColumnBucket::Retired => retired.push(entry),
        }
    }

    let ready_samples: Vec<&String> = ready.iter().take(SAMPLE_LIMIT).collect();
    let delivered_samples: Vec<&String> = delivered.iter().take(SAMPLE_LIMIT).collect();

    serde_json::json!({
        "ready_count": ready.len(),
        "in_progress_count": in_progress.len(),
        "delivered_count": delivered.len(),
        "accepted_count": accepted.len(),
        "icebox_count": icebox.len(),
        "retired_count": retired.len(),
        "ready_samples": ready_samples,
        "delivered_samples": delivered_samples,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::board::StoryState;
    use chrono::{TimeZone, Utc};

    fn story(num: u64, title: &str, state: StoryState) -> Story {
        Story::new_minimal(
            num,
            title.to_string(),
            state,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        )
    }

    #[test]
    fn empty_stories_returns_all_zero_counts() {
        let summary = build_backlog_summary(&[]);
        assert_eq!(summary["ready_count"], 0);
        assert_eq!(summary["in_progress_count"], 0);
        assert_eq!(summary["delivered_count"], 0);
        assert_eq!(summary["accepted_count"], 0);
        assert_eq!(summary["icebox_count"], 0);
    }

    #[test]
    fn accepted_state_counted_as_accepted() {
        let stories = vec![story(100, "done", StoryState::Accepted)];
        assert_eq!(build_backlog_summary(&stories)["accepted_count"], 1);
    }

    #[test]
    fn ready_state_populates_samples() {
        let stories = vec![story(101, "ready story", StoryState::Ready)];
        let summary = build_backlog_summary(&stories);
        assert_eq!(summary["ready_count"], 1);
        assert_eq!(summary["ready_samples"][0], "#101 ready story");
    }

    #[test]
    fn in_progress_state_counted_correctly() {
        let stories = vec![
            story(102, "a", StoryState::InProgress),
            story(103, "b", StoryState::InProgress),
        ];
        assert_eq!(build_backlog_summary(&stories)["in_progress_count"], 2);
    }

    #[test]
    fn delivered_and_desk_check_collapse_into_delivered_count() {
        let stories = vec![
            story(104, "d", StoryState::Delivered),
            story(105, "dc", StoryState::DeskCheck),
        ];
        assert_eq!(build_backlog_summary(&stories)["delivered_count"], 2);
    }

    #[test]
    fn icebox_state_counted_as_icebox() {
        let stories = vec![story(107, "icy", StoryState::Icebox)];
        assert_eq!(build_backlog_summary(&stories)["icebox_count"], 1);
    }

    #[test]
    fn closed_non_accepted_story_lands_in_retired_count() {
        let mut s = story(109, "retired", StoryState::Ready);
        s.is_closed = true;
        let summary = build_backlog_summary(&[s]);
        assert_eq!(summary["retired_count"], 1);
        assert_eq!(summary["ready_count"], 0);
        assert_eq!(summary["accepted_count"], 0);
    }

    #[test]
    fn samples_cap_at_sample_limit() {
        let stories: Vec<Story> = (0..(SAMPLE_LIMIT as u64 + 3))
            .map(|i| story(200 + i, &format!("ready {i}"), StoryState::Ready))
            .collect();
        let summary = build_backlog_summary(&stories);
        assert_eq!(summary["ready_count"], serde_json::json!(SAMPLE_LIMIT + 3));
        assert_eq!(
            summary["ready_samples"].as_array().unwrap().len(),
            SAMPLE_LIMIT
        );
    }
}
