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
        StoryState::ReadyForAccept => "ready for accept",
        StoryState::NeedsFixes => "needs fixes",
        StoryState::Rejected => "rejected",
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

pub(in crate::bridge) async fn handle_get_story(
    args: &serde_json::Map<String, serde_json::Value>,
    board: Option<&Arc<BoardStore>>,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let (b, r) = board
        .zip(refresh)
        .ok_or("get_story requires board + refresh — tracker fallback retired in slice 5l")?;
    let mut story = b
        .story(number)
        .ok_or_else(|| format!("story #{number} not found in board"))?;
    if story.body.is_none() {
        let body = r
            .fetch_body(number)
            .await
            .map_err(|e| format!("fetch body: {e}"))?;
        story.body = Some(body);
    }
    let mut body_out = story.body.clone().unwrap_or_default();
    let needs_gates = matches!(
        story.state,
        StoryState::Ready | StoryState::InProgress | StoryState::Rejected | StoryState::NeedsFixes
    );
    if needs_gates {
        let gates = match story.gate_comments.clone() {
            Some(g) => g,
            None => r
                .fetch_gate_comments(number)
                .await
                .map_err(|e| format!("fetch gates: {e}"))?,
        };
        if let Some(ctx) = rework_context_from_gates(&gates) {
            body_out = enrich_body(&body_out, &ctx);
        }
    }
    let out = serde_json::json!({
        "number": story.number,
        "title": story.title,
        "body": body_out,
        "state": if story.state == StoryState::Accepted { "closed" } else { "open" },
        "labels": [{"name": state_label(story.state)}],
    });
    serde_json::to_string_pretty(&out).map_err(|e| format!("serialize story: {e}"))
}
