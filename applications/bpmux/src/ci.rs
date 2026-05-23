use std::collections::HashMap;

use serde::Deserialize;

use crate::session::Message;
use crate::session::message::{DeepDetail, MultiDepthPayload};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiState {
    #[default]
    Unknown,
    Green,
    Red,
    Running,
    QuotaExhausted,
}

#[derive(Debug, Clone, Default)]
pub struct CiEvent {
    pub state: CiState,
    pub sha: Option<String>,
    pub title: Option<String>,
    pub run_id: Option<u64>,
    pub workflows: Vec<WorkflowSummary>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct WorkflowSummary {
    pub name: Option<String>,
    pub run_id: Option<u64>,
    pub conclusion: &'static str,
}

pub(crate) fn compute_broadcast_text(next: &CiEvent) -> Option<String> {
    match next.state {
        CiState::Red => Some(format_failure_line(next)),
        CiState::Green => Some(format_success_line(next)),
        CiState::QuotaExhausted => Some(format_paused_line(next)),
        CiState::Running | CiState::Unknown => None,
    }
}

pub(crate) fn build_multi_depth_for_transition(event: &CiEvent) -> MultiDepthPayload {
    let (summary, dev_detail) = match event.state {
        CiState::Green => ("\u{1F7E2} CI green".to_string(), format_success_line(event)),
        CiState::Red => (
            "\u{1F534} CI failed".to_string(),
            format_failure_line(event),
        ),
        CiState::QuotaExhausted => ("\u{23F8} CI paused".to_string(), format_paused_line(event)),
        CiState::Running | CiState::Unknown => (
            "CI \u{2026}".to_string(),
            "CI in progress or unknown".to_string(),
        ),
    };

    let workflow = crate::workflow::current_workflow();
    let role_id_for = |purpose: &str| {
        workflow
            .role_id_for_purpose(purpose)
            .map(str::to_string)
            .unwrap_or_else(|| purpose.to_string())
    };
    let mut role_default: HashMap<String, String> = HashMap::new();
    role_default.insert(role_id_for("pm"), summary.clone());
    role_default.insert(role_id_for("designer"), summary.clone());
    role_default.insert(role_id_for("dev"), dev_detail);

    let deep = match event.run_id {
        Some(id) => DeepDetail::OnDemand {
            command: format!("gh run view {id}"),
            description: "full CI run output including all stages and logs".to_string(),
        },
        None => DeepDetail::Preloaded {
            text: "(no run id available)".to_string(),
        },
    };

    MultiDepthPayload {
        summary,
        role_default,
        deep,
    }
}

pub fn github_actions_url() -> &'static str {
    "https://github.com/RyanDur/bpmux/actions"
}

pub(crate) fn build_dev_ci_signal(text: &str) -> Message {
    let dev = crate::workflow::current_workflow()
        .role_id_for_purpose("dev")
        .map(str::to_string)
        .unwrap_or_else(|| "dev".to_string());
    Message {
        from: "bpmux".to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        to: Some(vec![dev.clone()]),
        addressee: Some(dev),
        ..Message::default()
    }
}

fn format_failure_line(event: &CiEvent) -> String {
    let mut line = String::from("\u{1F534} CI failed");
    append_meta(&mut line, event);
    line
}

fn format_success_line(event: &CiEvent) -> String {
    let mut line = String::from("\u{1F7E2} CI green");
    append_meta(&mut line, event);
    line
}

fn format_paused_line(event: &CiEvent) -> String {
    let mut line = String::from("\u{23F8} CI paused (quota exhausted)");
    append_meta(&mut line, event);
    line
}

fn append_meta(line: &mut String, event: &CiEvent) {
    if let Some(sha) = event.sha.as_deref() {
        let short: String = sha.chars().take(7).collect();
        line.push_str(" \u{2014} ");
        line.push_str(&short);
        if let Some(title) = event.title.as_deref() {
            line.push_str(" \"");
            const MAX_TITLE_LEN: usize = 60;
            if title.chars().count() > MAX_TITLE_LEN {
                let truncated: String = title.chars().take(MAX_TITLE_LEN - 1).collect();
                line.push_str(&truncated);
                line.push('\u{2026}');
            } else {
                line.push_str(title);
            }
            line.push('"');
        }
    }
    if let Some(run_id) = event.run_id {
        line.push_str(" \u{2192} gh run view ");
        line.push_str(&run_id.to_string());
    }
}

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
