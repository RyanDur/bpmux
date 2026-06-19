use chrono::{DateTime, Utc};

use crate::board::domain::{FreeComment, Gate, GateComment};
use crate::board::events::EventMeta;

pub(crate) fn decode_free_comment(v: &serde_json::Value) -> Option<FreeComment> {
    let body = v.get("body").and_then(|x| x.as_str())?.to_string();
    let author = v
        .get("author")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();
    let ts = v
        .get("created_at")
        .and_then(|x| x.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    Some(FreeComment { body, author, ts })
}

pub(crate) fn encode_gate_comment(gate: Gate, meta: &EventMeta, payload: Option<&str>) -> String {
    let (marker, tag) = match gate {
        Gate::Pickup => ("pickup_paraphrase", "dev pickup-gate"),
        Gate::Delivered => ("verified_observation", "dev deliver-gate"),
        Gate::PassDeskCheck => ("pass_desk_check", "desk-check-pass"),
        Gate::FailDeskCheck => ("rejection_reason", "desk-check-fail"),
        Gate::Rejected => ("rejection_reason", "reject-gate"),
        Gate::Accepted => ("accepted", "accept-gate"),
        Gate::Filed => ("filed", "file-gate"),
    };
    match payload {
        Some(body) => format!("**{marker}** ({tag} by {}):\n\n{body}", meta.actor),
        None => format!("**{marker}** ({tag} by {})", meta.actor),
    }
}

pub fn decode_gate_comments(comments: &[serde_json::Value]) -> Vec<GateComment> {
    comments
        .iter()
        .filter_map(|c| {
            let body = c.get("body").and_then(|x| x.as_str())?;
            let author = c
                .get("author")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string();
            let ts = c
                .get("created_at")
                .and_then(|x| x.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))?;
            classify_gate(body).map(|(gate, payload)| GateComment {
                gate,
                ts,
                actor: author,
                payload,
            })
        })
        .collect()
}

pub fn classify_gate(body: &str) -> Option<(Gate, Option<String>)> {
    if body.contains("**pickup_paraphrase**") {
        return Some((Gate::Pickup, extract_marker_payload(body)));
    }
    if body.contains("**verified_observation**") {
        return Some((Gate::Delivered, extract_marker_payload(body)));
    }
    if body.contains("**pass_desk_check**") {
        return Some((Gate::PassDeskCheck, None));
    }
    if body.contains("**accepted**") {
        return Some((Gate::Accepted, None));
    }
    if body.contains("**rejection_reason**") {
        let gate = if body.contains("desk-check-fail") {
            Gate::FailDeskCheck
        } else {
            Gate::Rejected
        };
        return Some((gate, extract_marker_payload(body)));
    }
    None
}

fn extract_marker_payload(body: &str) -> Option<String> {
    body.split_once(":\n\n").map(|(_, rest)| rest.to_string())
}
