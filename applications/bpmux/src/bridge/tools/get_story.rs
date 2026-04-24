//! #403 MCP get_story — read-side bypass closure. Before this
//! shipped, agents read story bodies via raw `gh issue view <N>`,
//! invisible to the bridge + teammates. Now the same read goes
//! through the tracker + lands in the shared chat stream.
//!
//! #419 extends this with rework-context enrichment: when the
//! story is in a rework state (rejected / needs fixes), the latest
//! rejection_reason + prior verified_observation are prepended to
//! the body so pickup + desk-check + re-audit all start with
//! context-in-hand, not comment-archaeology.

use crate::bridge::tracker::StoryTracker;

use super::rework_context::{
    build_rework_context, enrich_body, is_rework_labeled, labels_from_story_json,
};

pub(in crate::bridge) async fn handle_get_story(
    args: &serde_json::Map<String, serde_json::Value>,
    tracker: &impl StoryTracker,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    match tracker.get_story(number).await? {
        Some(mut story) => {
            if is_rework_labeled(&labels_from_story_json(&story))
                && let Ok(comments) = tracker.list_story_comments(number).await
                && let Some(ctx) = build_rework_context(&comments)
            {
                let original = story
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                story["body"] = serde_json::Value::String(enrich_body(&original, &ctx));
            }
            serde_json::to_string_pretty(&story).map_err(|e| format!("serialize story: {e}"))
        }
        None => Err(format!("story #{number} not found")),
    }
}
