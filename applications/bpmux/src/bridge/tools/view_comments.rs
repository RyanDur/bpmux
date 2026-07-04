use std::sync::Arc;

use crate::sync::RefreshDriver;

pub(in crate::bridge) async fn handle_view_comments(
    args: &serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let r = refresh.ok_or("view_comments requires refresh (canonical-store)")?;
    let story = r
        .story(id)
        .ok_or_else(|| format!("story #{id} not found in board"))?;
    let hydrated = story.free_comments.is_some();
    let shaped: Vec<serde_json::Value> = story
        .free_comments
        .unwrap_or_default()
        .iter()
        .map(|c| {
            serde_json::json!({
                "author": c.author,
                "body": c.body,
                "created_at": c.ts.to_rfc3339(),
            })
        })
        .collect();
    let response = serde_json::json!({
        "comments": shaped,
        "free_comments_hydrated": hydrated,
    });
    serde_json::to_string_pretty(&response).map_err(|e| format!("serialize comments: {e}"))
}
