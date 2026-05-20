use std::sync::Arc;

use crate::board::adapters::gh_label_codec::GhLabelCodec;
use crate::board::domain::{
    Gate, GateComment, StoryState, StoryType, derive_iteration_member_section,
    most_recent_backward_transition, replay_transitions, substitute_member_stories_section,
};
use crate::board::iteration_label::{members_with_iteration_label, theme_name_from_title};
use crate::board::{BoardStore, RefreshDriver};

use super::rework_context::enrich_body;

fn state_label(state: StoryState) -> &'static str {
    match state {
        StoryState::Icebox => "icebox",
        StoryState::Ready => "ready",
        StoryState::InProgress => "in progress",
        StoryState::DeskCheck => "desk-check",
        StoryState::Delivered => "delivered",
        StoryState::Acceptance => "acceptance",
        StoryState::Accepted => "accepted",
        StoryState::Retired => "retired",
    }
}

/// Build the "## Rework context" body section when the story's most-
/// recent transition is backward. The rejection reason is the backward
/// transition's comment; the verified-observation is the most-recent
/// Gate::Delivered comment (which describes what's being reworked).
fn rework_context_from_gates(gates: &[GateComment]) -> Option<String> {
    let transitions = replay_transitions(gates);
    let backward = most_recent_backward_transition(&transitions)?;
    let rejection = backward.comment.clone();
    let prior_delivery = gates
        .iter()
        .rev()
        .find(|g| g.gate == Gate::Delivered)
        .and_then(|g| g.payload.clone());
    let mut out = String::from("## Rework context\n\n");
    if let Some(r) = rejection {
        out.push_str(&format!("**rejection_reason** (reject-gate):\n\n{r}\n\n"));
    }
    if let Some(p) = prior_delivery {
        out.push_str(&format!(
            "**verified_observation** (dev deliver-gate):\n\n{p}\n\n"
        ));
    }
    out.push_str("---\n");
    Some(out)
}

const NOT_HYDRATED_PLACEHOLDER: &str =
    "(body not yet hydrated; no refresh driver available to live-fetch.)";

pub(in crate::bridge) async fn handle_get_story(
    args: &serde_json::Map<String, serde_json::Value>,
    board: Option<&Arc<BoardStore>>,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let b = board.ok_or("get_story requires board (canonical-store)")?;
    let mut story = b
        .story(number)
        .ok_or_else(|| format!("story #{number} not found in board"))?;
    if story.body.is_none()
        && let Some(r) = refresh
    {
        match r.fetch_body(number).await {
            Ok(_) => {
                story = b
                    .story(number)
                    .ok_or_else(|| format!("story #{number} not found in board after fetch"))?;
            }
            Err(e) => {
                crate::session::diag::log_diag(&format!(
                    "get_story #{number} read-through fetch_body failed: {e}"
                ));
            }
        }
    }
    let body_cached = story.body.as_deref();
    let mut body_out = body_cached.unwrap_or(NOT_HYDRATED_PLACEHOLDER).to_string();
    let needs_gates = matches!(story.state, StoryState::Ready | StoryState::InProgress);
    if needs_gates
        && let Some(gates) = story.gate_comments.as_ref()
        && let Some(ctx) = rework_context_from_gates(gates)
    {
        body_out = enrich_body(&body_out, &ctx);
    }
    if story.story_type == StoryType::Iteration && body_cached.is_some() {
        let theme_name = theme_name_from_title(&story.title);
        let codec = GhLabelCodec;
        let all_stories = b.all_stories();
        let member_numbers = members_with_iteration_label(&codec, &all_stories, &theme_name);
        let members: Vec<(u64, String)> = member_numbers
            .iter()
            .filter_map(|n| b.story(*n).map(|s| (*n, s.title.clone())))
            .collect();
        let members_refs: Vec<(u64, &str)> =
            members.iter().map(|(n, t)| (*n, t.as_str())).collect();
        let derived = derive_iteration_member_section(&members_refs);
        body_out = substitute_member_stories_section(&body_out, &derived);
    }
    let labels: Vec<serde_json::Value> = story
        .raw_labels
        .iter()
        .map(|name| serde_json::json!({"name": name}))
        .collect();
    let out = serde_json::json!({
        "number": story.number,
        "title": story.title,
        "body": body_out,
        "workflow_state": state_label(story.state),
        "is_closed": story.is_closed,
        "labels": labels,
        "parent_epic": story.parent_epic,
        "body_hydrated": body_cached.is_some(),
        "gate_comments_hydrated": story.gate_comments.is_some(),
    });
    serde_json::to_string_pretty(&out).map_err(|e| format!("serialize story: {e}"))
}
