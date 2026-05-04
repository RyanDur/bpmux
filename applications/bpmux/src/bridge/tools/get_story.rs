use std::sync::Arc;

use crate::board::domain::{
    Gate, GateComment, StoryState, most_recent_backward_transition, replay_transitions,
};
use crate::board::{BoardStore, RefreshDriver};

use super::rework_context::enrich_body;

fn state_label(state: StoryState) -> &'static str {
    match state {
        StoryState::Icebox => "icebox",
        StoryState::Ready => "ready",
        StoryState::InProgress => "in progress",
        StoryState::DeskCheck => "desk-check",
        StoryState::Delivered => "delivered",
        StoryState::Accepted => "accepted",
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

const NOT_HYDRATED_PLACEHOLDER: &str = "(body not yet hydrated; background poller will hydrate. \
     Use refresh_story to force live-fetch when needed for high-stake operations.)";

pub(in crate::bridge) async fn handle_get_story(
    args: &serde_json::Map<String, serde_json::Value>,
    board: Option<&Arc<BoardStore>>,
    _refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let b = board.ok_or("get_story requires board (canonical-store)")?;
    let story = b
        .story(number)
        .ok_or_else(|| format!("story #{number} not found in board"))?;
    let body_cached = story.body.as_deref();
    let mut body_out = body_cached.unwrap_or(NOT_HYDRATED_PLACEHOLDER).to_string();
    let needs_gates = matches!(story.state, StoryState::Ready | StoryState::InProgress);
    if needs_gates
        && let Some(gates) = story.gate_comments.as_ref()
        && let Some(ctx) = rework_context_from_gates(gates)
    {
        body_out = enrich_body(&body_out, &ctx);
    }
    let out = serde_json::json!({
        "number": story.number,
        "title": story.title,
        "body": body_out,
        "workflow_state": state_label(story.state),
        "is_closed": story.is_closed,
        "labels": [{"name": state_label(story.state)}],
        "body_hydrated": body_cached.is_some(),
        "gate_comments_hydrated": story.gate_comments.is_some(),
    });
    serde_json::to_string_pretty(&out).map_err(|e| format!("serialize story: {e}"))
}
