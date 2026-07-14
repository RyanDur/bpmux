use serde::Deserialize;

use super::{CiEvent, CiState, WorkflowSummary};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RunEntry {
    pub status: String,
    pub conclusion: Option<String>,
    #[serde(alias = "headSha", default)]
    pub head_sha: Option<String>,
    #[serde(alias = "displayTitle", default)]
    pub display_title: Option<String>,
    #[serde(rename = "id", alias = "databaseId", default)]
    pub database_id: Option<u64>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, skip)]
    pub annotation_message: Option<String>,
}

pub(crate) fn annotation_indicates_quota_exhaustion(msg: &str) -> bool {
    msg.contains("job was not started because")
        || msg.contains("spending limit")
        || msg.contains("billing")
}

fn classify_single_run(run: &RunEntry) -> CiState {
    if run.status == "in_progress" || run.status == "queued" {
        return CiState::Running;
    }
    match run.conclusion.as_deref() {
        Some("success") => CiState::Green,
        Some(_) => match run.annotation_message.as_deref() {
            Some(msg) if annotation_indicates_quota_exhaustion(msg) => CiState::QuotaExhausted,
            _ => CiState::Red,
        },
        None => CiState::Unknown,
    }
}

fn ci_state_str(state: CiState) -> &'static str {
    match state {
        CiState::Green => "success",
        CiState::Red => "failure",
        CiState::Running => "in_progress",
        CiState::QuotaExhausted => "quota_exhausted",
        CiState::Unknown => "unknown",
    }
}

pub fn classify_runs(runs: &[RunEntry]) -> CiEvent {
    let Some(latest) = runs.first() else {
        return CiEvent::default();
    };
    let head_sha = latest.head_sha.clone();
    let filtered: Vec<&RunEntry> = if head_sha.is_some() {
        runs.iter().filter(|r| r.head_sha == head_sha).collect()
    } else {
        vec![latest]
    };
    let per_workflow: Vec<(CiState, &RunEntry)> = filtered
        .iter()
        .map(|r| (classify_single_run(r), *r))
        .collect();
    let aggregate = if per_workflow.iter().any(|(s, _)| *s == CiState::Running) {
        CiState::Running
    } else if per_workflow.iter().any(|(s, _)| *s == CiState::Red) {
        CiState::Red
    } else if per_workflow
        .iter()
        .any(|(s, _)| *s == CiState::QuotaExhausted)
    {
        CiState::QuotaExhausted
    } else if !per_workflow.is_empty() && per_workflow.iter().all(|(s, _)| *s == CiState::Green) {
        CiState::Green
    } else {
        CiState::Unknown
    };
    let workflows: Vec<WorkflowSummary> = per_workflow
        .iter()
        .map(|(s, r)| WorkflowSummary {
            name: r.name.clone(),
            run_id: r.database_id,
            conclusion: ci_state_str(*s),
        })
        .collect();
    CiEvent {
        state: aggregate,
        sha: latest.head_sha.clone(),
        title: latest.display_title.clone(),
        run_id: latest.database_id,
        workflows,
    }
}
