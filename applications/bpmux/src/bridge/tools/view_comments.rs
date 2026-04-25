use std::sync::Arc;

use crate::board::{BoardStore, RefreshDriver};
use crate::bridge::tracker::StoryTracker;

pub(in crate::bridge) async fn handle_view_comments(
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
        .ok_or("view_comments requires board + refresh — tracker fallback retired in slice 5l")?;
    if b.story(number).is_none() {
        return Err(format!("story #{number} not found in board"));
    }
    let free = match b.story(number).and_then(|s| s.free_comments) {
        Some(list) => list,
        None => r
            .fetch_free_comments(number)
            .await
            .map_err(|e| format!("fetch free_comments: {e}"))?,
    };
    let shaped: Vec<serde_json::Value> = free
        .iter()
        .map(|c| {
            serde_json::json!({
                "author": c.author,
                "body": c.body,
                "created_at": c.ts.to_rfc3339(),
            })
        })
        .collect();
    serde_json::to_string_pretty(&shaped).map_err(|e| format!("serialize comments: {e}"))
}
