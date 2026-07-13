use std::sync::Arc;

use crate::sync::RefreshDriver;
use product_management::scheduling::ProjectedState;
use product_management::story::{CastVerdict, Stage, Verdict, most_recent_needs_work};
use workflow::StageId;

use super::get_story_view::{
    NOT_HYDRATED_PLACEHOLDER, render_epic_detail, render_iteration_detail,
};
use super::rework_context::enrich_body;

fn state_label(state: &ProjectedState, held: Option<&StageId>) -> String {
    match state {
        ProjectedState::Icebox => "icebox".to_string(),
        ProjectedState::Ready => "ready".to_string(),
        ProjectedState::InProgress => match held.map(StageId::as_str) {
            Some("in-progress") | None => "in progress".to_string(),
            Some(stage) => stage.to_string(),
        },
        ProjectedState::Acceptance => "acceptance".to_string(),
        ProjectedState::Accepted => "accepted".to_string(),
        ProjectedState::Retired => "retired".to_string(),
    }
}

fn rework_context_from_gates(gates: &[CastVerdict]) -> Option<String> {
    let rejection = most_recent_needs_work(gates)?.payload.clone();
    let prior_delivery = gates
        .iter()
        .rev()
        .find(|g| matches!(&g.verdict, Verdict::Succeed { step } if step.as_str() == "in-progress"))
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

fn latest_reject_json(gates: &[CastVerdict]) -> Option<serde_json::Value> {
    let gc = super::rework_context::latest_reject(gates)?;
    let reason = gc.payload.clone()?;
    Some(serde_json::json!({
        "reason": reason,
        "by": gc.actor,
        "at": gc.ts.to_rfc3339(),
        "gate": super::rework_context::reject_label(&gc.verdict),
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
    if let Some(iteration) = r.iteration(product_management::iteration::IterationId::new(id)) {
        return render_iteration_detail(r, &iteration).await;
    }
    if let Some(epic) = r.epic(product_management::epic::EpicId::new(id)) {
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
                diag::log_diag(&format!(
                    "get_story #{id} read-through fetch_body failed: {e}"
                ));
            }
        }
    }
    let fine = r.story_fine_stage(&story);
    let iterations = r.iterations();
    let projected = product_management::scheduling::projected_state(&story, &iterations);
    let needs_gates = matches!(projected, ProjectedState::Ready)
        || fine == Some(Stage::Dev)
        || story.is_concluded();
    if needs_gates && story.verdicts.is_none() {
        match r.fetch_verdicts(id).await {
            Ok(_) => {
                story = r
                    .story(id)
                    .ok_or_else(|| format!("story #{id} not found in board after gate fetch"))?;
            }
            Err(e) => {
                diag::log_diag(&format!(
                    "get_story #{id} read-through fetch_verdicts failed: {e}"
                ));
            }
        }
    }
    let body_cached = story.body.as_deref();
    let mut body_out = body_cached.unwrap_or(NOT_HYDRATED_PLACEHOLDER).to_string();
    let latest_reject = story.verdicts.as_deref().and_then(latest_reject_json);
    let held = r.story_held_stage(&story);
    let workflow_state = story
        .verdicts
        .as_deref()
        .and_then(most_recent_needs_work)
        .map(|g| match &g.verdict {
            Verdict::Fail { step } if step.as_str() == "desk-check" => "in progress".to_string(),
            _ => "ready".to_string(),
        })
        .unwrap_or_else(|| state_label(&projected, held.as_ref()));
    if needs_gates
        && let Some(gates) = story.verdicts.as_ref()
        && let Some(ctx) = rework_context_from_gates(gates)
    {
        body_out = enrich_body(&body_out, &ctx);
    }
    let codec = crate::integrations::github::gh_label_codec::GhLabelCodec;
    let labels: Vec<serde_json::Value> = view::board::wire_labels(&codec, &story)
        .iter()
        .map(|name| serde_json::json!({"name": name}))
        .collect();
    let out = serde_json::json!({
        "id": story.id,
        "title": story.title,
        "body": body_out,
        "is_closed": matches!(workflow_state.as_str(), "accepted" | "retired"),
        "workflow_state": workflow_state,
        "labels": labels,
        "parent_epic": story.parent_epic(),
        "body_hydrated": body_cached.is_some(),
        "gate_comments_hydrated": story.verdicts.is_some(),
        "latest_reject": latest_reject,
    });
    serde_json::to_string_pretty(&out).map_err(|e| format!("serialize story: {e}"))
}
