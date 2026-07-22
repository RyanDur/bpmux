use std::sync::Arc;

use product_management::story::{CastVerdict, Story, StoryId, Verdict};
use sync::SyncOrchestrator;

pub fn verdict_as_str(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::Succeed { step } => match step.as_str() {
            "ready" => "pickup",
            "in-progress" => "delivered",
            "desk-check" => "pass_desk_check",
            _ => "accepted",
        },
        Verdict::Fail { step } => match step.as_str() {
            "desk-check" => "fail_desk_check",
            _ => "rejected",
        },
    }
}

pub fn build_timeline_from_board(story: &Story, gates: &[CastVerdict]) -> Vec<serde_json::Value> {
    let mut events: Vec<serde_json::Value> = Vec::new();
    events.push(serde_json::json!({
        "gate": "filed",
        "ts": story.filed_at.to_rfc3339(),
        "actor": story.filed_by,
        "payload": serde_json::Value::Null,
    }));
    for g in gates {
        events.push(serde_json::json!({
            "gate": verdict_as_str(&g.verdict),
            "ts": g.ts.to_rfc3339(),
            "actor": g.actor,
            "payload": g.payload.clone().map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
        }));
    }
    events.sort_by(|a, b| {
        let ak = a.get("ts").and_then(|v| v.as_str()).unwrap_or("");
        let bk = b.get("ts").and_then(|v| v.as_str()).unwrap_or("");
        ak.cmp(bk)
    });
    events
}

pub async fn handle_view_story_timeline(
    args: &serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<SyncOrchestrator>>,
    select: &store::Select,
) -> Result<String, String> {
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let r = refresh.ok_or("view_story_timeline requires refresh (canonical-store)")?;
    let mut story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(id)))
        .ok_or_else(|| format!("story #{id} not found in board"))?;
    if story.verdicts.is_none() {
        match r.fetch_verdicts(id).await {
            Ok(_) => {
                story = select
                    .select(|state| {
                        product_management::story::slice::get(&state.stories, StoryId::new(id))
                    })
                    .ok_or_else(|| format!("story #{id} not found in board after gate fetch"))?;
            }
            Err(e) => {
                diag::log_diag(&format!(
                    "view_story_timeline #{id} read-through fetch_verdicts failed: {e}"
                ));
            }
        }
    }
    let hydrated = story.verdicts.is_some();
    let gates = story.verdicts.clone().unwrap_or_default();
    let events = build_timeline_from_board(&story, &gates);
    let response = serde_json::json!({
        "events": events,
        "gate_comments_hydrated": hydrated,
    });
    serde_json::to_string_pretty(&response).map_err(|e| format!("serialize timeline: {e}"))
}
