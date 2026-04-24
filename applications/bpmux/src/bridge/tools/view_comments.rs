use std::sync::Arc;

use crate::board::{BoardStore, RefreshDriver};
use crate::bridge::tracker::StoryTracker;

pub(in crate::bridge) async fn handle_view_comments(
    args: &serde_json::Map<String, serde_json::Value>,
    tracker: &impl StoryTracker,
    board: Option<&Arc<BoardStore>>,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    if let (Some(b), Some(r)) = (board, refresh)
        && b.story(number).is_some()
    {
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
        return serde_json::to_string_pretty(&shaped)
            .map_err(|e| format!("serialize comments: {e}"));
    }
    let comments = tracker.list_story_comments(number).await?;
    serde_json::to_string_pretty(&comments).map_err(|e| format!("serialize comments: {e}"))
}
