use chrono::{DateTime, Utc};

use super::label_fields::story_type_from_labels;
use crate::product_management::board::events::LabelDerivedFields;
use crate::product_management::story::{Story, StoryState, StoryType, Tag, story_type_from_title};

pub fn story_type_from_labels_or_title(labels: &[String], title: &str) -> StoryType {
    story_type_from_labels(labels).unwrap_or_else(|| story_type_from_title(title))
}

pub fn project_labels(labels: &[String]) -> LabelDerivedFields {
    LabelDerivedFields {
        raw_labels: labels.to_vec(),
    }
}

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
        crate::diag::log_diag(&format!(
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
    story.story_type = story_type_from_labels_or_title(&labels, &title);
    story.is_closed = is_closed;
    story.raw_labels = labels;
    if let Some(body) = v.get("body").and_then(|x| x.as_str()) {
        story.body = Some(body.to_string());
        story.acceptance_test_path = crate::bridge::parse_acceptance_test_path(body);
    }
    Some(story)
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
