use serde::Serialize;

use crate::product_management::board::adapters::github::{
    horizon_from_labels, is_current_from_labels, iteration_dev_ok_by_from_labels,
    iteration_order_from_labels, iteration_review_closers_from_labels,
    outcome_membership_from_labels,
};
use crate::product_management::board::label_codec::LabelCodec;
use crate::product_management::outcome::Outcome;
use crate::product_management::story::{
    Story, StoryState, Tag, most_recent_backward_transition, replay_transitions,
};

#[derive(Debug, Clone, Serialize)]
pub struct IterationMembershipView {
    pub iteration_number: u64,
    pub position: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutcomeMembershipView {
    pub outcome_number: u64,
    pub position: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoryView {
    pub number: u64,
    pub title: String,
    pub state: &'static str,
    pub gh_state: &'static str,
    pub labels: Vec<String>,
    pub parent_epic: Option<u64>,
    pub iteration_membership: Option<IterationMembershipView>,
    pub outcome_membership: Option<OutcomeMembershipView>,
    pub iteration_order: Option<u32>,
    pub iteration_review_closed_by: Vec<String>,
    pub iteration_dev_ok_by: Vec<String>,
    pub horizon: Option<&'static str>,
    pub is_current: bool,
    pub acceptor: Option<&'static str>,
    pub assignees: Vec<String>,
    pub created_at: String,
    pub body: String,
    pub rework_comment: Option<String>,
}

impl StoryView {
    pub fn project<C: LabelCodec>(codec: &C, story: &Story) -> Self {
        Self {
            number: story.number,
            title: story.title().to_string(),
            state: story.state().as_str(),
            gh_state: if story.is_closed { "closed" } else { "open" },
            labels: wire_labels(codec, story),
            parent_epic: story.parent_epic(),
            iteration_membership: story.iteration_membership().map(
                |(iteration_number, position)| IterationMembershipView {
                    iteration_number,
                    position,
                },
            ),
            outcome_membership: outcome_membership_from_labels(story.raw_labels()).map(
                |(outcome_number, position)| OutcomeMembershipView {
                    outcome_number,
                    position,
                },
            ),
            iteration_order: iteration_order_from_labels(story.raw_labels()),
            iteration_review_closed_by: iteration_review_closers_from_labels(story.raw_labels()),
            iteration_dev_ok_by: iteration_dev_ok_by_from_labels(story.raw_labels()),
            horizon: horizon_from_labels(story.raw_labels()).map(|h| h.as_str()),
            is_current: is_current_from_labels(story.raw_labels()),
            acceptor: story.acceptor().map(|a| a.as_str()),
            assignees: Vec::new(),
            created_at: story.filed_at().to_rfc3339(),
            body: story.body().unwrap_or_default().to_string(),
            rework_comment: rework_comment(story),
        }
    }

    pub fn from_outcome(outcome: &Outcome) -> Self {
        let retired = outcome.is_retired();
        Self {
            number: outcome.id().saved_number().unwrap_or(0),
            title: outcome.content.title.clone(),
            state: if retired {
                StoryState::Retired.as_str()
            } else {
                StoryState::Icebox.as_str()
            },
            gh_state: if retired { "closed" } else { "open" },
            labels: vec![
                crate::product_management::story::StoryType::Outcome
                    .as_label()
                    .to_string(),
            ],
            parent_epic: None,
            iteration_membership: None,
            outcome_membership: None,
            iteration_order: None,
            iteration_review_closed_by: Vec::new(),
            iteration_dev_ok_by: Vec::new(),
            horizon: outcome.horizon().map(|h| h.as_str()),
            is_current: false,
            acceptor: None,
            assignees: Vec::new(),
            created_at: outcome.content.filed_at.to_rfc3339(),
            body: outcome.content.body.clone().unwrap_or_default(),
            rework_comment: None,
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
        labels.push(codec.encode_parent_epic(epic));
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
