use serde::Serialize;

use crate::product_management::board::label_codec::LabelCodec;
use crate::product_management::epic::EpicId;
use crate::product_management::iteration::IterationId;
use crate::product_management::story::{
    Story, Tag, most_recent_backward_transition, replay_transitions,
};

#[derive(Debug, Clone, Serialize)]
pub struct IterationMembershipView {
    pub iteration_id: IterationId,
    pub position: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoryView {
    pub id: u64,
    pub title: String,
    pub state: &'static str,
    pub gh_state: &'static str,
    pub labels: Vec<String>,
    pub parent_epic: Option<EpicId>,
    pub iteration_membership: Option<IterationMembershipView>,
    pub acceptor: Option<&'static str>,
    pub assignees: Vec<String>,
    pub created_at: String,
    pub body: String,
    pub rework_comment: Option<String>,
}

impl StoryView {
    pub fn project<C: LabelCodec>(codec: &C, story: &Story) -> Self {
        Self {
            id: story.id.value(),
            title: story.title().to_string(),
            state: story.state().as_str(),
            gh_state: if story.is_closed { "closed" } else { "open" },
            labels: wire_labels(codec, story),
            parent_epic: story.parent_epic(),
            iteration_membership: story
                .iteration_membership()
                .map(|(iteration_id, position)| IterationMembershipView {
                    iteration_id,
                    position,
                }),
            acceptor: story.acceptor().map(|a| a.as_str()),
            assignees: Vec::new(),
            created_at: story.filed_at().to_rfc3339(),
            body: story.body().unwrap_or_default().to_string(),
            rework_comment: rework_comment(story),
        }
    }
}

fn is_renderer_badge_label(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower == "desk-check-fail" || lower.starts_with("blocked-by:")
}

fn wire_labels<C: LabelCodec>(codec: &C, story: &Story) -> Vec<String> {
    let mut labels: Vec<String> = Vec::new();
    labels.push(story.story_type().as_label().to_string());
    for tag in story.tags() {
        match tag {
            Tag::Blocked => labels.push("blocked".to_string()),
        }
    }
    if let Some(epic) = story.parent_epic() {
        labels.push(codec.encode_parent_epic(epic.value()));
    }
    if let Some(position) = story.ready_order() {
        labels.push(codec.encode_ready_order(position));
    }
    for raw in story.raw_labels() {
        if codec.is_iteration_label_loose(raw) && !labels.iter().any(|l| l == raw) {
            labels.push(raw.clone());
        }
        if is_renderer_badge_label(raw) && !labels.iter().any(|l| l == raw) {
            labels.push(raw.clone());
        }
    }
    labels
}

fn rework_comment(story: &Story) -> Option<String> {
    story
        .gate_comments()
        .map(replay_transitions)
        .as_deref()
        .and_then(|ts| most_recent_backward_transition(ts).cloned())
        .and_then(|t| t.comment)
}
