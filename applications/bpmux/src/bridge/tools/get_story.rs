use std::sync::Arc;

use crate::product_management::board::RefreshDriver;
use crate::product_management::story::{
    BoardState, Gate, GateComment, Stage, most_recent_backward_transition, replay_transitions,
};

use super::get_story_view::{
    NOT_HYDRATED_PLACEHOLDER, render_epic_detail, render_iteration_detail,
};
use super::rework_context::enrich_body;

fn state_label(state: BoardState) -> &'static str {
    match state {
        BoardState::Icebox => "icebox",
        BoardState::Ready => "ready",
        BoardState::InProgress(Stage::Dev) => "in progress",
        BoardState::InProgress(Stage::DeskCheck) => "desk-check",
        BoardState::Acceptance => "acceptance",
        BoardState::Accepted => "accepted",
        BoardState::Retired => "retired",
    }
}

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

fn latest_reject_json(gates: &[GateComment]) -> Option<serde_json::Value> {
    let gc = super::rework_context::latest_reject(gates)?;
    let reason = gc.payload.clone()?;
    Some(serde_json::json!({
        "reason": reason,
        "by": gc.actor,
        "at": gc.ts.to_rfc3339(),
        "gate": super::rework_context::reject_gate_label(gc.gate),
    }))
}

pub(in crate::bridge) async fn handle_get_story(
    args: &serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let r = refresh.ok_or("get_story requires refresh (canonical-store)")?;
    if let Some(iteration) = r.iteration(crate::product_management::iteration::IterationId::new(id))
    {
        return render_iteration_detail(r, &iteration).await;
    }
    if let Some(epic) = r.epic(crate::product_management::epic::EpicId::new(id)) {
        return render_epic_detail(refresh, &epic).await;
    }
    let mut story = r
        .story(id)
        .ok_or_else(|| format!("story #{id} not found in board"))?;
    if story.body.is_none() {
        match r.fetch_body(id).await {
            Ok(_) => {
                story = r
                    .story(id)
                    .ok_or_else(|| format!("story #{id} not found in board after fetch"))?;
            }
            Err(e) => {
                crate::diag::log_diag(&format!(
                    "get_story #{id} read-through fetch_body failed: {e}"
                ));
            }
        }
    }
    let needs_gates = matches!(
        story.state,
        BoardState::Ready | BoardState::InProgress(Stage::Dev)
    ) || story.is_closed
        || matches!(story.state, BoardState::Accepted);
    if needs_gates && story.gate_comments.is_none() {
        match r.fetch_gate_comments(id).await {
            Ok(_) => {
                story = r
                    .story(id)
                    .ok_or_else(|| format!("story #{id} not found in board after gate fetch"))?;
            }
            Err(e) => {
                crate::diag::log_diag(&format!(
                    "get_story #{id} read-through fetch_gate_comments failed: {e}"
                ));
            }
        }
    }
    let body_cached = story.body.as_deref();
    let mut body_out = body_cached.unwrap_or(NOT_HYDRATED_PLACEHOLDER).to_string();
    let latest_reject = story.gate_comments.as_deref().and_then(latest_reject_json);
    let workflow_state = story
        .gate_comments
        .as_deref()
        .and_then(|gates| {
            let transitions = replay_transitions(gates);
            most_recent_backward_transition(&transitions).map(|t| state_label(t.to))
        })
        .unwrap_or_else(|| state_label(story.state));
    if needs_gates
        && let Some(gates) = story.gate_comments.as_ref()
        && let Some(ctx) = rework_context_from_gates(gates)
    {
        body_out = enrich_body(&body_out, &ctx);
    }
    let codec = crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
    let labels: Vec<serde_json::Value> =
        crate::product_management::board::view_model::wire_labels(&codec, &story)
            .iter()
            .map(|name| serde_json::json!({"name": name}))
            .collect();
    let out = serde_json::json!({
        "id": story.id,
        "title": story.title,
        "body": body_out,
        "workflow_state": workflow_state,
        "is_closed": story.is_closed,
        "labels": labels,
        "parent_epic": story.parent_epic(),
        "body_hydrated": body_cached.is_some(),
        "gate_comments_hydrated": story.gate_comments.is_some(),
        "latest_reject": latest_reject,
    });
    serde_json::to_string_pretty(&out).map_err(|e| format!("serialize story: {e}"))
}
