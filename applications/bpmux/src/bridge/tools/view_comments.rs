use crate::bridge::tracker::StoryTracker;

pub(in crate::bridge) async fn handle_view_comments(
    args: &serde_json::Map<String, serde_json::Value>,
    tracker: &impl StoryTracker,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let comments = tracker.list_story_comments(number).await?;
    serde_json::to_string_pretty(&comments).map_err(|e| format!("serialize comments: {e}"))
}
