use chrono::{DateTime, Utc};

use super::label_fields::story_type_from_labels;
use crate::product_management::board::events::LabelDerivedFields;
use crate::product_management::epic::EpicId;
use crate::product_management::story::{
    BoardState, Stage, Story, StoryId, StoryKind, Tag, story_type_from_title,
};

pub fn story_type_from_labels_or_title(labels: &[String], title: &str) -> StoryKind {
    story_type_from_labels(labels).unwrap_or_else(|| story_type_from_title(title))
}

pub fn project_labels(labels: &[String]) -> LabelDerivedFields {
    LabelDerivedFields {
        acceptor: super::label_fields::acceptor_from_labels(labels),
        parent_epic: super::label_fields::parent_epic_from_labels(labels).map(EpicId::new),
        tags: tags_from_labels(labels),
        blocked_by: super::label_fields::blocked_by_from_labels(labels).map(StoryId::new),
        ready_order: super::label_fields::ready_order_from_labels(labels),
        iteration_slot: iteration_slot_from_labels(labels),
        raw_labels: labels.to_vec(),
    }
}

pub fn iteration_slot_from_labels(
    labels: &[String],
) -> Option<crate::product_management::story::IterationSlot> {
    let codec = super::super::gh_label_codec::GhLabelCodec;
    labels.iter().find_map(|label| {
        crate::product_management::iteration::label::parse_iteration_label(&codec, label).map(
            |parts| crate::product_management::story::IterationSlot {
                name: parts.name,
                position: parts.position,
            },
        )
    })
}

pub fn warn_on_inconsistent_state_labels(labels: &[String]) {
    let state_labels = [
        BoardState::Ready.github_label(),
        BoardState::InProgress(Stage::Dev).github_label(),
        BoardState::InProgress(Stage::DeskCheck).github_label(),
        BoardState::Acceptance.github_label(),
        BoardState::Accepted.github_label(),
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
    let mut story = Story::new_minimal(
        StoryId::new(number),
        title.clone(),
        state,
        filed_at,
        filed_by,
    );
    story.is_closed = is_closed;
    story.story_type = story_type_from_labels_or_title(&labels, &title);
    story.acceptor = super::label_fields::acceptor_from_labels(&labels);
    story.parent_epic = super::label_fields::parent_epic_from_labels(&labels).map(EpicId::new);
    story.tags = tags_from_labels(&labels);
    story.blocked_by = super::label_fields::blocked_by_from_labels(&labels).map(StoryId::new);
    story.ready_order = super::label_fields::ready_order_from_labels(&labels);
    story.iteration_slot = iteration_slot_from_labels(&labels);
    if let Some(body) = v.get("body").and_then(|x| x.as_str()) {
        story.body = Some(body.to_string());
    }
    Some(story)
}

pub(super) fn extract_label_names(v: &serde_json::Value) -> Vec<String> {
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
    use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
    let mut out = Vec::new();
    for l in labels {
        if let Some(tag) = GhLabelCodec.decode_tag(l)
            && !out.contains(&tag)
        {
            out.push(tag);
        }
    }
    out
}

pub fn state_from_labels(labels: &[String], is_closed: bool) -> BoardState {
    let has = |needle: &str| labels.iter().any(|l| l.eq_ignore_ascii_case(needle));
    warn_on_inconsistent_state_labels(labels);
    if has(BoardState::Accepted.github_label()) {
        return BoardState::Accepted;
    }
    if has(BoardState::Retired.github_label()) {
        return BoardState::Retired;
    }
    if is_closed {
        return BoardState::Accepted;
    }
    if has("rejected") {
        return BoardState::Ready;
    }
    if has("needs fixes") {
        return BoardState::InProgress(Stage::Dev);
    }
    if has("ready for accept") {
        return BoardState::Acceptance;
    }
    if has(BoardState::Acceptance.github_label()) {
        return BoardState::Acceptance;
    }
    if has(BoardState::InProgress(Stage::DeskCheck).github_label()) {
        return BoardState::InProgress(Stage::DeskCheck);
    }
    if has(BoardState::InProgress(Stage::Dev).github_label()) {
        return BoardState::InProgress(Stage::Dev);
    }
    if has(BoardState::Ready.github_label()) {
        return BoardState::Ready;
    }
    BoardState::Icebox
}

pub fn state_to_label(state: BoardState) -> &'static str {
    state.github_label()
}
