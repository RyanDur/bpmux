use chrono::{DateTime, Utc};

use super::label_fields::story_type_from_labels;
use super::state_labels::resolve_state_and_placement;
use product_management::epic::EpicId;
use product_management::story::events::LabelDerivedFields;
use product_management::story::{Story, StoryId, StoryKind, Tag, story_type_from_title};
use workflow::StageId;

pub fn story_type_from_labels_or_title(labels: &[String], title: &str) -> StoryKind {
    story_type_from_labels(labels).unwrap_or_else(|| story_type_from_title(title))
}

pub fn project_labels(labels: &[String]) -> LabelDerivedFields {
    LabelDerivedFields {
        acceptor: super::label_fields::acceptor_from_labels(labels),
        parent_epic: super::label_fields::parent_epic_from_labels(labels).map(EpicId::new),
        tags: tags_from_labels(labels),
        raw_labels: labels.to_vec(),
    }
}

pub fn decode_story_metadata(v: &serde_json::Value) -> Option<Story> {
    decode_story_with_placement(v).map(|(story, _)| story)
}

pub fn decode_story_with_placement(v: &serde_json::Value) -> Option<(Story, Option<StageId>)> {
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
    let (state, placement) = resolve_state_and_placement(&labels, is_closed);
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
    let remote_updated_at = v
        .get("updated_at")
        .and_then(|x| x.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));
    let mut story = Story::new_minimal(
        StoryId::new(number),
        title.clone(),
        state,
        filed_at,
        filed_by,
    );
    story.remote_updated_at = remote_updated_at;
    story.story_type = story_type_from_labels_or_title(&labels, &title);
    story.acceptor = super::label_fields::acceptor_from_labels(&labels);
    story.parent_epic = super::label_fields::parent_epic_from_labels(&labels).map(EpicId::new);
    story.tags = tags_from_labels(&labels);
    if let Some(body) = v.get("body").and_then(|x| x.as_str()) {
        story.body = Some(body.to_string());
    }
    Some((story, placement))
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
    use crate::gh_label_codec::GhLabelCodec;
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
