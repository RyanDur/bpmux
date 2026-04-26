//! Shared helpers for agent-side E2E tests that need a target story.
//!
//! Most #476 Phase 1 backfill tests follow the pattern:
//!
//!   1. PM calls create_story → message contains "#N"
//!   2. Test extracts N and uses it for subsequent calls
//!
//! Centralising the extract + create-target boilerplate so per-tool
//! tests stay focused on the falsifiers specific to that tool.

use super::bridge::TestBridge;

pub fn extract_story_number(create_msg: &str) -> u64 {
    let tail = &create_msg[create_msg.find('#').expect("create message names #N") + 1..];
    let end = tail
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(tail.len());
    tail[..end].parse().expect("parse story number")
}

pub async fn create_target_story(pm: &TestBridge, title: &str, falsifier: &str) -> u64 {
    let create = pm
        .call_tool(
            "create_story",
            serde_json::json!({
                "title": title,
                "user_job": "When testing affordance E2E, dev needs a target story.",
                "observation_falsifier": falsifier
            }),
        )
        .await
        .expect("create_story should succeed");
    let create_msg = create.as_str().unwrap_or("").to_string();
    extract_story_number(&create_msg)
}
