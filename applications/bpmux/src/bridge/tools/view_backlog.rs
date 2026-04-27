use std::sync::Arc;

use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};
use crate::board::{BoardStore, Story, StoryState};

pub(in crate::bridge) const VIEW_BACKLOG_MARKER: &str = "__BPMUX_VIEW_BACKLOG__";

const SAMPLE_LIMIT: usize = 5;

pub(in crate::bridge) async fn handle_view_backlog(
    ws_tx: &Mutex<Option<WsSink>>,
    store: &Arc<BoardStore>,
) -> Result<String, String> {
    let marker_msg = crate::transport::protocol::ClientMessage::Send {
        text: VIEW_BACKLOG_MARKER.to_string(),
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
    };
    match serde_json::to_string(&marker_msg) {
        Ok(json) => {
            if let Err(e) = ws_send(ws_tx, json).await {
                crate::session::diag::log_diag(&format!("view_backlog marker send failed: {e}"));
            }
        }
        Err(e) => {
            crate::session::diag::log_diag(&format!("view_backlog marker serialize failed: {e}"))
        }
    }

    let stories = store.all_stories();
    let summary = build_backlog_summary(&stories);
    serde_json::to_string(&summary).map_err(|e| format!("serialize summary: {e}"))
}

pub fn build_backlog_summary(stories: &[Story]) -> serde_json::Value {
    let mut ready: Vec<String> = Vec::new();
    let mut in_progress: Vec<String> = Vec::new();
    let mut delivered: Vec<String> = Vec::new();
    let mut accepted: Vec<String> = Vec::new();
    let mut icebox: Vec<String> = Vec::new();

    for story in stories {
        let entry = format!("#{} {}", story.number, story.title);
        match story.state {
            StoryState::Accepted => accepted.push(entry),
            StoryState::Delivered
            | StoryState::ReadyForAccept
            | StoryState::DeskCheck
            | StoryState::NeedsFixes => delivered.push(entry),
            StoryState::InProgress => in_progress.push(entry),
            StoryState::Ready => ready.push(entry),
            StoryState::Rejected | StoryState::Icebox => icebox.push(entry),
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
        "ready_samples": ready_samples,
        "delivered_samples": delivered_samples,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
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
    fn delivered_readyforaccept_needsfixes_collapse_into_delivered_count() {
        let stories = vec![
            story(104, "d", StoryState::Delivered),
            story(105, "r", StoryState::ReadyForAccept),
            story(106, "n", StoryState::NeedsFixes),
        ];
        assert_eq!(build_backlog_summary(&stories)["delivered_count"], 3);
    }

    #[test]
    fn icebox_state_counted_as_icebox() {
        let stories = vec![story(107, "icy", StoryState::Icebox)];
        assert_eq!(build_backlog_summary(&stories)["icebox_count"], 1);
    }

    #[test]
    fn rejected_state_folds_into_icebox() {
        let stories = vec![story(108, "rej", StoryState::Rejected)];
        assert_eq!(build_backlog_summary(&stories)["icebox_count"], 1);
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
