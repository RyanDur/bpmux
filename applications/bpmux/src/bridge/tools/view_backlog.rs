use crate::product_management::story::Story;
use crate::product_management::story::lifecycle::{Conclusion, Lifecycle, OnBoard};

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
        match Lifecycle::from_fields(&story.state, story.is_closed, None, story.acceptor) {
            Lifecycle::Backlog => icebox.push(entry),
            Lifecycle::OnBoard(OnBoard::Ready) => ready.push(entry),
            Lifecycle::OnBoard(OnBoard::InProgress(_)) => in_progress.push(entry),
            Lifecycle::OnBoard(OnBoard::Acceptance(_)) => acceptance.push(entry),
            Lifecycle::Concluded(Conclusion::Accepted(_)) => accepted.push(entry),
            Lifecycle::Concluded(Conclusion::Retired) => retired.push(entry),
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
