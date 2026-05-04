use std::collections::HashMap;

use serde::Deserialize;

use crate::session::Message;
use crate::session::message::{DeepDetail, MultiDepthPayload};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

    let mut role_default: HashMap<String, String> = HashMap::new();
    role_default.insert("pm".to_string(), summary.clone());
    role_default.insert("designer".to_string(), summary.clone());
    role_default.insert("dev".to_string(), dev_detail);

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
    Message {
        from: "bpmux".to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        to: Some(vec!["dev".to_string()]),
        addressee: Some("dev".to_string()),
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
            // Truncate long titles so the one-line format stays scannable.
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

#[derive(Debug, Deserialize, Default)]
pub struct RunEntry {
    pub status: String,
    pub conclusion: Option<String>,
    #[serde(alias = "headSha", default)]
    pub head_sha: Option<String>,
    #[serde(alias = "displayTitle", default)]
    pub display_title: Option<String>,
    #[serde(rename = "id", alias = "databaseId", default)]
    pub database_id: Option<u64>,
    #[serde(default, skip)]
    pub annotation_message: Option<String>,
}

pub(crate) fn annotation_indicates_quota_exhaustion(msg: &str) -> bool {
    msg.contains("job was not started because")
        || msg.contains("spending limit")
        || msg.contains("billing")
}

pub fn classify_runs(runs: &[RunEntry]) -> CiEvent {
    let Some(latest) = runs.first() else {
        return CiEvent::default();
    };
    // "Running wins over stale success or stale failure — the user cares
    // that a fresh answer is coming." (spec)
    let state = if latest.status == "in_progress" || latest.status == "queued" {
        CiState::Running
    } else {
        match latest.conclusion.as_deref() {
            Some("success") => CiState::Green,
            // #397: distinguish quota-exhausted / infrastructure-
            // blocked failures from genuine code failures. The
            // annotation message (populated upstream by the tracker
            // for failed runs) carries GitHub's reason; if it matches
            // the quota-exhaustion pattern, map to QuotaExhausted
            // (team-gate pass-with-citation). Otherwise map to Red
            // (genuine failure — block gates).
            Some(_) => match latest.annotation_message.as_deref() {
                Some(msg) if annotation_indicates_quota_exhaustion(msg) => CiState::QuotaExhausted,
                _ => CiState::Red,
            },
            None => CiState::Unknown,
        }
    };
    CiEvent {
        state,
        sha: latest.head_sha.clone(),
        title: latest.display_title.clone(),
        run_id: latest.database_id,
    }
}
