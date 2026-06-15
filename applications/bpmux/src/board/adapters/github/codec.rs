use chrono::{DateTime, Utc};

use crate::board::domain::{FreeComment, Gate, GateComment, Story, StoryState, Tag};
use crate::board::events::EventMeta;

pub fn warn_on_inconsistent_state_labels(labels: &[String]) {
    let state_labels = [
        StoryState::Ready.github_label(),
        StoryState::InProgress.github_label(),
        StoryState::DeskCheck.github_label(),
        StoryState::Delivered.github_label(),
        StoryState::Accepted.github_label(),
        "ready for accept",
        "needs fixes",
        "rejected",
    ];
    let present: Vec<&str> = state_labels
        .iter()
        .copied()
        .filter(|needle| labels.iter().any(|l| l.eq_ignore_ascii_case(needle)))
        .collect();
    if present.len() >= 2 {
        crate::session::diag::log_diag(&format!(
            "decoder: inconsistent state-labels present {present:?} — priority decoder picks first per spec; expected: only one state-label per story"
        ));
    }
}

pub fn decode_story_metadata(v: &serde_json::Value) -> Option<Story> {
    let number = v.get("number").and_then(|x| x.as_u64())?;
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let labels = extract_label_names(v);
    let is_closed = match v.get("gh_state").and_then(|x| x.as_str()) {
        Some(s) => matches!(s, "closed" | "CLOSED"),
        None => matches!(
            v.get("state").and_then(|x| x.as_str()),
            Some("closed" | "CLOSED")
        ),
    };
    let state = state_from_labels(&labels, is_closed);
    let filed_at = v
        .get("created_at")
        .and_then(|x| x.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let filed_by = v
        .get("user")
        .and_then(|u| u.get("login"))
        .and_then(|x| x.as_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".to_string());
    let mut story = Story::new_minimal(number, title.clone(), state, filed_at, filed_by);
    story.story_type = crate::board::domain::story_type_from_labels_or_title(&labels, &title);
    story.tags = tags_from_labels(&labels);
    story.is_closed = is_closed;
    story.parent_epic = parent_epic_from_labels(&labels);
    story.iteration_membership = iteration_membership_from_labels(&labels);
    story.outcome_membership = outcome_membership_from_labels(&labels);
    story.iteration_order = iteration_order_from_labels(&labels);
    story.iteration_review_closed_by = iteration_review_closers_from_labels(&labels);
    story.iteration_dev_ok_by = iteration_dev_ok_by_from_labels(&labels);
    story.horizon = horizon_from_labels(&labels);
    story.is_current = is_current_from_labels(&labels);
    story.acceptor = acceptor_from_labels(&labels);
    story.raw_labels = labels;
    if let Some(body) = v.get("body").and_then(|x| x.as_str()) {
        story.body = Some(body.to_string());
        story.acceptance_test_path = crate::bridge::parse_acceptance_test_path(body);
    }
    Some(story)
}

pub fn parent_epic_from_labels(labels: &[String]) -> Option<u64> {
    labels
        .iter()
        .find_map(|l| crate::board::adapters::gh_label_codec::GhLabelCodec.decode_parent_epic(l))
}

pub fn iteration_membership_from_labels(labels: &[String]) -> Option<(u64, u32)> {
    labels.iter().find_map(|l| {
        crate::board::adapters::gh_label_codec::GhLabelCodec.decode_parent_iteration(l)
    })
}

pub fn outcome_membership_from_labels(labels: &[String]) -> Option<(u64, u32)> {
    labels
        .iter()
        .find_map(|l| crate::board::adapters::gh_label_codec::GhLabelCodec.decode_parent_outcome(l))
}

pub fn iteration_order_from_labels(labels: &[String]) -> Option<u32> {
    labels
        .iter()
        .filter_map(|l| {
            crate::board::adapters::gh_label_codec::GhLabelCodec.decode_iteration_order(l)
        })
        .min()
}

pub fn iteration_review_closers_from_labels(labels: &[String]) -> Vec<String> {
    labels
        .iter()
        .filter_map(|l| {
            crate::board::adapters::gh_label_codec::GhLabelCodec.decode_iteration_review_closer(l)
        })
        .collect()
}

pub fn iteration_dev_ok_by_from_labels(labels: &[String]) -> Vec<String> {
    labels
        .iter()
        .filter_map(|l| {
            crate::board::adapters::gh_label_codec::GhLabelCodec.decode_iteration_dev_ok(l)
        })
        .collect()
}

pub fn is_current_from_labels(labels: &[String]) -> bool {
    labels
        .iter()
        .any(|l| crate::board::adapters::gh_label_codec::GhLabelCodec.is_current_iteration_label(l))
}

pub fn acceptor_from_labels(labels: &[String]) -> Option<crate::board::domain::Acceptor> {
    labels
        .iter()
        .find_map(|l| crate::board::adapters::gh_label_codec::GhLabelCodec.decode_acceptor(l))
}

pub fn horizon_from_labels(labels: &[String]) -> Option<crate::board::domain::Horizon> {
    labels
        .iter()
        .find_map(|l| crate::board::adapters::gh_label_codec::GhLabelCodec.decode_horizon(l))
}

fn is_iteration_position_label(label: &str) -> bool {
    crate::board::adapters::gh_label_codec::GhLabelCodec.is_iteration_position_label(label)
}

pub(crate) fn is_position_label(label: &str) -> bool {
    use crate::board::label_codec::LabelCodec;
    is_iteration_position_label(label)
        || crate::board::adapters::gh_label_codec::GhLabelCodec.is_ready_order_label(label)
}

#[cfg(test)]
pub(crate) fn is_iteration_position_label_for_test(label: &str) -> bool {
    is_iteration_position_label(label)
}

#[cfg(test)]
pub(crate) fn is_position_label_for_test(label: &str) -> bool {
    is_position_label(label)
}

fn extract_label_names(v: &serde_json::Value) -> Vec<String> {
    let Some(labels) = v.get("labels").and_then(|x| x.as_array()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = Vec::with_capacity(labels.len());
    for label in labels {
        if let Some(s) = label.as_str() {
            names.push(s.to_string());
        } else if let Some(n) = label.get("name").and_then(|x| x.as_str()) {
            names.push(n.to_string());
        }
    }
    names
}

pub fn tags_from_labels(labels: &[String]) -> Vec<Tag> {
    let mut out = Vec::new();
    for l in labels {
        if l.eq_ignore_ascii_case("blocked") && !out.contains(&Tag::Blocked) {
            out.push(Tag::Blocked);
        }
    }
    out
}

pub fn state_from_labels(labels: &[String], is_closed: bool) -> StoryState {
    let has = |needle: &str| labels.iter().any(|l| l.eq_ignore_ascii_case(needle));
    warn_on_inconsistent_state_labels(labels);
    if has(StoryState::Accepted.github_label()) {
        return StoryState::Accepted;
    }
    if has(StoryState::Retired.github_label()) {
        return StoryState::Retired;
    }
    if is_closed {
        return StoryState::Accepted;
    }
    if has("rejected") {
        return StoryState::Ready;
    }
    if has("needs fixes") {
        return StoryState::InProgress;
    }
    if has("ready for accept") {
        return StoryState::Delivered;
    }
    if has(StoryState::Delivered.github_label()) {
        return StoryState::Delivered;
    }
    if has(StoryState::DeskCheck.github_label()) {
        return StoryState::DeskCheck;
    }
    if has(StoryState::InProgress.github_label()) {
        return StoryState::InProgress;
    }
    if has(StoryState::Ready.github_label()) {
        return StoryState::Ready;
    }
    StoryState::Icebox
}

pub fn state_to_label(state: StoryState) -> &'static str {
    state.github_label()
}

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
