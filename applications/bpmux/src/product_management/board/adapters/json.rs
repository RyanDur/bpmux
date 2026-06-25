use crate::product_management::board::adapters::github::{
    acceptor_from_labels, iteration_dev_ok_by_from_labels, iteration_order_from_labels,
    iteration_review_closers_from_labels,
};
use crate::product_management::board::label_codec::LabelCodec;
use crate::product_management::story::{
    Story, StoryType, Tag, most_recent_backward_transition, replay_transitions,
};

fn is_renderer_badge_label(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower == "desk-check-fail" || lower.starts_with("blocked-by:")
}

pub fn story_to_board_json<C: LabelCodec>(codec: &C, story: &Story) -> serde_json::Value {
    let mut labels: Vec<String> = Vec::new();
    labels.push(
        match story.story_type {
            StoryType::Feature => "feature",
            StoryType::Bug => "bug",
            StoryType::Chore => "chore",
            StoryType::Epic => "epic",
            StoryType::Iteration => "iteration",
            StoryType::Outcome => "outcome",
        }
        .to_string(),
    );
    for tag in &story.tags {
        match tag {
            Tag::Blocked => labels.push("blocked".to_string()),
        }
    }
    if let Some(epic) = story.parent_epic {
        labels.push(codec.encode_parent_epic(epic));
    }
    for raw in &story.raw_labels {
        if codec.is_iteration_label_loose(raw) && !labels.iter().any(|l| l == raw) {
            labels.push(raw.clone());
        }
        if codec.is_ready_order_label(raw) && !labels.iter().any(|l| l == raw) {
            labels.push(raw.clone());
        }
        if is_renderer_badge_label(raw) && !labels.iter().any(|l| l == raw) {
            labels.push(raw.clone());
        }
    }
    let rework_comment: Option<String> = story
        .gate_comments
        .as_ref()
        .map(|gates| replay_transitions(gates))
        .as_deref()
        .and_then(|ts| most_recent_backward_transition(ts).cloned())
        .and_then(|t| t.comment);
    let iteration_membership = story
        .iteration_membership
        .map(|(iteration_number, position)| {
            serde_json::json!({
                "iteration_number": iteration_number,
                "position": position,
            })
        });
    let outcome_membership = story.outcome_membership.map(|(outcome_number, position)| {
        serde_json::json!({
            "outcome_number": outcome_number,
            "position": position,
        })
    });
    serde_json::json!({
        "number": story.number,
        "title": story.title,
        "state": story.state.as_str(),
        "gh_state": if story.is_closed { "closed" } else { "open" },
        "labels": labels,
        "parent_epic": story.parent_epic,
        "iteration_membership": iteration_membership,
        "outcome_membership": outcome_membership,
        "iteration_order": iteration_order_from_labels(&story.raw_labels),
        "iteration_review_closed_by": iteration_review_closers_from_labels(&story.raw_labels),
        "iteration_dev_ok_by": iteration_dev_ok_by_from_labels(&story.raw_labels),
        "horizon": story.horizon.map(|h| h.as_str()),
        "is_current": story.is_current,
        "acceptor": acceptor_from_labels(&story.raw_labels).map(|a| a.as_str()),
        "assignees": Vec::<String>::new(),
        "created_at": story.filed_at.to_rfc3339(),
        "body": story.body.clone().unwrap_or_default(),
        "rework_comment": rework_comment,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "json_tests.rs"]
mod tests;
