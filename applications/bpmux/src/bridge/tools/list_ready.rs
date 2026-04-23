//! #403 MCP list_ready — full ready-queue enumeration. Pairs with
//! `get_story` (read-side bypass closure). `view_backlog` returns
//! counts + samples; when the queue runs longer than the sample
//! window, agents fell back to `gh issue list --label ready`. This
//! closes that fallback.

use crate::bridge::tracker::StoryTracker;

pub(in crate::bridge) async fn handle_list_ready(
    tracker: &impl StoryTracker,
) -> Result<String, String> {
    let stories = tracker.fetch_board_stories().await?;
    // Filter to state == "ready". The state field is computed at
    // tracker.fetch time via `derive_renderer_state`, which maps
    // the "ready" label to state="ready". Return only the fields
    // pickup-gate paraphrase needs — number, title, labels — so
    // body doesn't bloat the enumeration payload (agents call
    // get_story(N) for body when they pick a specific story).
    let ready: Vec<serde_json::Value> = stories
        .into_iter()
        .filter(|s| s.get("state").and_then(|v| v.as_str()) == Some("ready"))
        .map(|s| {
            serde_json::json!({
                "number": s.get("number"),
                "title": s.get("title"),
                "labels": s.get("labels"),
            })
        })
        .collect();
    serde_json::to_string_pretty(&ready).map_err(|e| format!("serialize: {e}"))
}
