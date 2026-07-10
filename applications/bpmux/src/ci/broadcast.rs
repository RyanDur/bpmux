use std::collections::HashMap;

use super::{CiEvent, CiState};
use crate::message::Message;
use crate::message::{DeepDetail, MultiDepthPayload};

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
    let role_name_for = |purpose: &str| {
        workflow
            .role_name_for_purpose(purpose)
            .map(str::to_string)
            .unwrap_or_else(|| purpose.to_string())
    };
    let mut role_default: HashMap<String, String> = HashMap::new();
    role_default.insert(role_name_for("pm"), summary.clone());
    role_default.insert(role_name_for("designer"), summary.clone());
    role_default.insert(role_name_for("dev"), dev_detail);

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

pub(crate) fn build_dev_ci_signal(text: &str) -> Message {
    let dev = crate::workflow::current_workflow()
        .role_name_for_purpose("dev")
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
