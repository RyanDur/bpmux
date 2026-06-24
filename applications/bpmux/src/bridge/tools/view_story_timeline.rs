use std::sync::Arc;

use crate::board::domain::{Gate, GateComment, Story};
use crate::board::{BoardStore, RefreshDriver};

pub fn gate_as_str(gate: Gate) -> &'static str {
    match gate {
        Gate::Filed => "filed",
        Gate::Pickup => "pickup",
        Gate::Delivered => "delivered",
        Gate::PassDeskCheck => "pass_desk_check",
        Gate::FailDeskCheck => "fail_desk_check",
        Gate::Rejected => "rejected",
        Gate::Accepted => "accepted",
    }
}

pub fn build_timeline_from_board(story: &Story, gates: &[GateComment]) -> Vec<serde_json::Value> {
    let mut events: Vec<serde_json::Value> = Vec::new();
    events.push(serde_json::json!({
        "gate": "filed",
        "ts": story.filed_at.to_rfc3339(),
        "actor": story.filed_by,
        "payload": serde_json::Value::Null,
    }));
    for g in gates {
        events.push(serde_json::json!({
            "gate": gate_as_str(g.gate),
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

pub(in crate::bridge) async fn handle_view_story_timeline(
    args: &serde_json::Map<String, serde_json::Value>,
    board: Option<&Arc<BoardStore>>,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let b = board.ok_or("view_story_timeline requires board (canonical-store)")?;
    let mut story = b
        .story(number)
        .ok_or_else(|| format!("story #{number} not found in board"))?;
    if story.gate_comments.is_none()
        && let Some(r) = refresh
    {
        match r.fetch_gate_comments(number).await {
            Ok(_) => {
                story = b.story(number).ok_or_else(|| {
                    format!("story #{number} not found in board after gate fetch")
                })?;
            }
            Err(e) => {
                crate::diag::log_diag(&format!(
                    "view_story_timeline #{number} read-through fetch_gate_comments failed: {e}"
                ));
            }
        }
    }
    let hydrated = story.gate_comments.is_some();
    let gates = story.gate_comments.clone().unwrap_or_default();
    let events = build_timeline_from_board(&story, &gates);
    let response = serde_json::json!({
        "events": events,
        "gate_comments_hydrated": hydrated,
    });
    serde_json::to_string_pretty(&response).map_err(|e| format!("serialize timeline: {e}"))
}
