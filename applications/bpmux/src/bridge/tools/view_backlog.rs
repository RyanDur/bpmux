use crate::product_management::iteration::Iteration;
use crate::product_management::scheduling::projected_state;
use crate::product_management::story::{BoardState, Story};

pub const VIEW_BACKLOG_MARKER: &str = "__BPMUX_VIEW_BACKLOG__";

const SAMPLE_LIMIT: usize = 5;

pub(in crate::bridge) fn build_backlog_summary(
    stories: &[Story],
    iterations: &[Iteration],
) -> serde_json::Value {
    let mut ready: Vec<String> = Vec::new();
    let mut in_progress: Vec<String> = Vec::new();
    let mut acceptance: Vec<String> = Vec::new();
    let mut accepted: Vec<String> = Vec::new();
    let mut icebox: Vec<String> = Vec::new();
    let mut retired: Vec<String> = Vec::new();

    for story in stories {
        let entry = format!("{} {}", story.id, story.title);
        match projected_state(story, iterations) {
            BoardState::Icebox => icebox.push(entry),
            BoardState::Ready => ready.push(entry),
            BoardState::InProgress => in_progress.push(entry),
            BoardState::Acceptance => acceptance.push(entry),
            BoardState::Accepted => accepted.push(entry),
            BoardState::Retired => retired.push(entry),
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
