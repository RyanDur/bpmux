//! #403 MCP get_story — read-side bypass closure. Before this
//! shipped, agents read story bodies via raw `gh issue view <N>`,
//! invisible to the bridge + teammates. Now the same read goes
//! through the tracker + lands in the shared chat stream.

use crate::bridge::tracker::StoryTracker;

pub(in crate::bridge) async fn handle_get_story(
    args: &serde_json::Map<String, serde_json::Value>,
    tracker: &impl StoryTracker,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    match tracker.get_story(number).await? {
        Some(story) => {
            serde_json::to_string_pretty(&story).map_err(|e| format!("serialize story: {e}"))
        }
        None => Err(format!("story #{number} not found")),
    }
}
