//! E2E for #1192 move_story → retired distinct-state at canonical-store.
//! Per `feedback_falsifier_load_bearing_test_at_e2e_layer`: the
//! falsifier names the bug-class — "move_story → retired collapses
//! to Accepted at canonical-store; no distinct terminal-closed-not-
//! Accepted state". This test drives the cascade end-to-end:
//!
//! 1. pm creates a story (lands in icebox).
//! 2. pm move_story → retired with rationale.
//! 3. get_story returns state field that is NOT "accepted" (the bug
//!    was: pre-fix, state collapsed to Accepted because state_from_
//!    labels' closed-no-label fallback returned Accepted).
//! 4. view_board surfaces the story in the RETIRED column, not the
//!    ACCEPTED column — the renderer's column_for already supports
//!    Retired (closed && state != Accepted) but only if the canonical
//!    store preserves the distinction; this test pins both ends.
//!
//! Sister-class to #1188 falsifier-c (move_story recovery from
//! Accepted-stuck) but on the inverse axis: the closed terminal
//! state must be distinguishable from Accepted *going in*, not just
//! *coming out*.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn move_story_retired_produces_state_distinct_from_accepted_at_canonical_store() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] move-story-retired-distinct-state-target",
            "reject if move_story → retired collapses to Accepted at canonical-store",
        )
        .await;

        pm.call_tool(
            "move_story",
            serde_json::json!({
                "number": target_number,
                "target_state": "retired",
                "rationale": "retire scope to drive #1192 distinct-state assertion"
            }),
        )
        .await
        .expect("move_story → retired should succeed");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let story = pm
            .call_tool("get_story", serde_json::json!({ "number": target_number }))
            .await
            .expect("get_story should succeed after retire");

        let state = story
            .get("workflow_state")
            .and_then(|v| v.as_str())
            .or_else(|| story.get("state").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        assert_ne!(
            state.to_ascii_lowercase(),
            "accepted",
            "retired story must NOT report state=accepted at canonical-store (#1192 falsifier-c); got state={state}, full={story}"
        );

        let is_closed = story
            .get("is_closed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(
            is_closed,
            "retired story must be is_closed=true (move_story → retired closes the issue); full={story}"
        );

        pm.close().await;
    });
}

#[test]
fn move_story_retired_then_view_board_buckets_in_retired_not_accepted_column() {
    // Sister assertion to the canonical-store check above: the renderer
    // path (view_board → column_for → ColumnBucket) must surface the
    // story under RETIRED, not ACCEPTED. column_for itself is unit-
    // tested elsewhere; this drives the full chain so a regression in
    // either layer (canonical or renderer) flags here.
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] move-story-retired-renderer-bucket-target",
            "reject if retired story shows in ACCEPTED column on view_board",
        )
        .await;

        pm.call_tool(
            "move_story",
            serde_json::json!({
                "number": target_number,
                "target_state": "retired",
                "rationale": "drive #1192 view_board renderer-bucket assertion"
            }),
        )
        .await
        .expect("move_story → retired should succeed");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let board = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");

        let board_text = board.as_str().unwrap_or("").to_string();
        let id_token = format!("#{target_number}");
        assert!(
            board_text.contains(&id_token),
            "view_board must surface the retired story somewhere; got board={board_text}"
        );

        let retired_section = section_after_header(&board_text, "RETIRED");
        let accepted_section = section_after_header(&board_text, "ACCEPTED");
        assert!(
            retired_section.contains(&id_token),
            "retired story must appear under the RETIRED header; retired_section={retired_section}\nfull board={board_text}"
        );
        assert!(
            !accepted_section.contains(&id_token),
            "retired story must NOT appear under the ACCEPTED header (#1192 falsifier-c); accepted_section={accepted_section}\nfull board={board_text}"
        );

        pm.close().await;
    });
}

/// Slice the board text from the line containing `header` up to the
/// next ALL-CAPS column header (or end). Defensive against future
/// header reorders — we anchor on the literal bucket name and stop at
/// the next bucket boundary so we don't accidentally pull entries
/// from neighbouring columns into the section we're checking.
fn section_after_header(board_text: &str, header: &str) -> String {
    let mut found = false;
    let mut out = String::new();
    let other_headers = [
        "ICEBOX",
        "READY",
        "IN PROGRESS",
        "DESK CHECK",
        "DELIVERED",
        "ACCEPTED",
        "RETIRED",
    ];
    for line in board_text.lines() {
        if !found {
            if line.contains(header) && line.chars().any(|c| c.is_ascii_uppercase()) {
                found = true;
                out.push_str(line);
                out.push('\n');
            }
            continue;
        }
        if other_headers.iter().any(|h| {
            *h != header && line.contains(h) && line.chars().any(|c| c.is_ascii_uppercase())
        }) {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}
