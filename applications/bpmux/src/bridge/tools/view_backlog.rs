use crate::product_management::board::arrangement::{ColumnBucket, column_for};
use crate::product_management::story::Story;

pub const VIEW_BACKLOG_MARKER: &str = "__BPMUX_VIEW_BACKLOG__";

const SAMPLE_LIMIT: usize = 5;

pub(in crate::bridge) fn build_backlog_summary(stories: &[Story]) -> serde_json::Value {
    let mut ready: Vec<String> = Vec::new();
    let mut in_progress: Vec<String> = Vec::new();
    let mut acceptance: Vec<String> = Vec::new();
    let mut accepted: Vec<String> = Vec::new();
    let mut icebox: Vec<String> = Vec::new();
    let mut retired: Vec<String> = Vec::new();

    for story in stories {
        let entry = format!("{} {}", story.id, story.title);
        match column_for(story.state.clone(), story.is_closed) {
            ColumnBucket::Icebox => icebox.push(entry),
            ColumnBucket::Ready => ready.push(entry),
            ColumnBucket::Middle(_) => in_progress.push(entry),
            ColumnBucket::Acceptance => acceptance.push(entry),
            ColumnBucket::Accepted => accepted.push(entry),
            ColumnBucket::Retired => retired.push(entry),
        }
    }

    let ready_samples: Vec<&String> = ready.iter().take(SAMPLE_LIMIT).collect();
    let acceptance_samples: Vec<&String> = acceptance.iter().take(SAMPLE_LIMIT).collect();

    serde_json::json!({
        "ready_count": ready.len(),
        "in_progress_count": in_progress.len(),
        "acceptance_count": acceptance.len(),
        "accepted_count": accepted.len(),
        "icebox_count": icebox.len(),
        "retired_count": retired.len(),
        "ready_samples": ready_samples,
        "acceptance_samples": acceptance_samples,
    })
}
