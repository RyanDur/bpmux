//! #403 MCP get_story — read-side bypass closure. Before this
//! shipped, agents read story bodies via raw `gh issue view <N>`,
//! invisible to the bridge + teammates. Now the same read goes
//! through the tracker + lands in the shared chat stream.
//!
//! #419 extends this with rework-context enrichment: when the
//! story is in a rework state (rejected / needs fixes), the latest
//! rejection_reason + prior verified_observation are prepended to
//! the body so pickup + desk-check + re-audit all start with
//! context-in-hand, not comment-archaeology.

use std::sync::Arc;

use crate::board::domain::{Gate, StoryState};
use crate::board::{BoardStore, RefreshDriver};
use crate::bridge::tracker::StoryTracker;

use super::rework_context::enrich_body;

fn state_label(state: StoryState) -> &'static str {
    match state {
        StoryState::Icebox => "icebox",
        StoryState::Ready => "ready",
        StoryState::InProgress => "in progress",
        StoryState::Delivered => "delivered",
        StoryState::ReadyForAccept => "ready for accept",
        StoryState::NeedsFixes => "needs fixes",
        StoryState::Rejected => "rejected",
        StoryState::Accepted => "accepted",
    }
}

fn rework_context_from_gates(gates: &[crate::board::domain::GateComment]) -> Option<String> {
    let rejection = gates
        .iter()
        .rev()
        .find(|g| matches!(g.gate, Gate::Rejected | Gate::FailDeskCheck))?
        .payload
        .clone();
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
    _tracker: &impl StoryTracker,
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
    if matches!(story.state, StoryState::Rejected | StoryState::NeedsFixes) {
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
