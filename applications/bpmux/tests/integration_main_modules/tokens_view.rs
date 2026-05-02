//! End-to-end tests for the tokens overlay render (#179 behavior).
//!
//! The overlay used to open via a `/tokens` slash command; that was
//! removed in #232 now that agents trigger it via the `view_tokens`
//! MCP tool (#225). These tests drive the real `bpmux` binary in a
//! PTY, pre-populate a fake token log in a tempdir via
//! `BPMUX_TOKENS_DIR`, invoke `view_tokens` through a `TestBridge`,
//! and assert the rendered screen buffer.
//!
//! Name stays behavior-oriented: key press → overlay open → log
//! read → overlay render → frame. The key-press step is now an MCP
//! call rather than a slash command, but the observable behavior
//! (what renders) is unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

/// Write two fake token log rows to the given tokens dir. Returns
/// the path for cleanup. Schema mirrors the docs/token-log-schema.md
/// contract so any drift between producer and consumer shows up here.
fn write_fake_log(tokens_dir: &std::path::Path) {
    std::fs::create_dir_all(tokens_dir).unwrap();
    let rows = [
        r#"{"ts":"2026-04-11T22:30:00.000Z","role":"dev","harness":"claude-code","model":"claude-opus-4-6","input_tokens":12,"cache_creation_input_tokens":0,"cache_read_input_tokens":8000,"output_tokens":450,"tool_call_count":3,"session_id":"s1"}"#,
        r#"{"ts":"2026-04-11T22:31:00.000Z","role":"pm","harness":"claude-code","model":"claude-opus-4-6","input_tokens":8,"cache_creation_input_tokens":0,"cache_read_input_tokens":4000,"output_tokens":200,"tool_call_count":1,"session_id":"s2"}"#,
    ];
    let content = rows.join("\n") + "\n";
    std::fs::write(tokens_dir.join("log.jsonl"), content).unwrap();
}

fn open_tokens_overlay(bpmux: &TestBpmux) {
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        pm.call_tool("view_tokens", serde_json::json!({}))
            .await
            .expect("view_tokens should succeed");
        pm.close().await;
    });
}

/// GIVEN a token log with rows for multiple roles
/// WHEN an agent invokes view_tokens
/// THEN the overlay renders with role totals and the close hint
#[test]
fn view_tokens_renders_overlay_with_role_breakdown() {
    let tmp = tempfile::tempdir().unwrap();
    write_fake_log(tmp.path());

    let bpmux = TestBpmux::start_with_env("ryan", "BPMUX_TOKENS_DIR", tmp.path().to_str().unwrap());
    open_tokens_overlay(&bpmux);

    assert!(
        bpmux.wait_for_tui("Press any key to close", Duration::from_secs(5)),
        "tokens overlay should render. screen:\n{}",
        bpmux.tui()
    );
    let screen = bpmux.tui();
    assert!(screen.contains("tokens"), "title missing:\n{screen}");
    assert!(
        screen.contains("By role:"),
        "role section missing:\n{screen}"
    );
    assert!(screen.contains("dev"), "dev role missing:\n{screen}");
    assert!(screen.contains("pm"), "pm role missing:\n{screen}");
}

/// GIVEN no token log exists yet
/// WHEN an agent invokes view_tokens
/// THEN the overlay shows the empty-state placeholder
#[test]
fn view_tokens_shows_empty_state_when_log_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    // Note: no file written — the tempdir is empty.
    let bpmux = TestBpmux::start_with_env("ryan", "BPMUX_TOKENS_DIR", tmp.path().to_str().unwrap());
    open_tokens_overlay(&bpmux);

    assert!(
        bpmux.wait_for_tui("No token rows yet", Duration::from_secs(5)),
        "empty-state placeholder should render. screen:\n{}",
        bpmux.tui()
    );
}

/// GIVEN a token log with multiple rows
/// WHEN an agent invokes view_tokens in a full-sized terminal
/// THEN the overlay renders the chart section (labelled "per turn")
#[test]
fn view_tokens_renders_chart_section_when_rows_are_present() {
    let tmp = tempfile::tempdir().unwrap();
    // Seed enough rows so the chart path renders (>= 2 rows).
    std::fs::create_dir_all(tmp.path()).unwrap();
    let mut content = String::new();
    for i in 0..6 {
        let out = 100 + i * 30;
        content.push_str(&format!(
            r#"{{"ts":"2026-04-11T22:3{i}:00.000Z","role":"dev","harness":"claude-code","model":"claude-opus-4-6","input_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":1000,"output_tokens":{out},"tool_call_count":0,"session_id":"s"}}"#
        ));
        content.push('\n');
    }
    std::fs::write(tmp.path().join("log.jsonl"), content).unwrap();

    let bpmux = TestBpmux::start_with_env("ryan", "BPMUX_TOKENS_DIR", tmp.path().to_str().unwrap());
    open_tokens_overlay(&bpmux);

    assert!(
        bpmux.wait_for_tui("Press any key to close", Duration::from_secs(5)),
        "overlay should render. screen:\n{}",
        bpmux.tui()
    );
    let screen = bpmux.tui();
    assert!(
        screen.contains("per turn"),
        "chart title missing — chart path didn't render. screen:\n{screen}"
    );
}

/// GIVEN the tokens overlay is visible
/// WHEN the user presses any key
/// THEN the overlay closes
#[test]
fn tokens_overlay_dismisses_on_any_key() {
    let tmp = tempfile::tempdir().unwrap();
    write_fake_log(tmp.path());
    let bpmux = TestBpmux::start_with_env("ryan", "BPMUX_TOKENS_DIR", tmp.path().to_str().unwrap());
    open_tokens_overlay(&bpmux);
    assert!(
        bpmux.wait_for_tui("Press any key to close", Duration::from_secs(5)),
        "overlay should be visible. screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_chars("x");
    bpmux
        .wait_for_tui_absent("Press any key to close", Duration::from_secs(5))
        .expect("overlay should be dismissed");
    assert!(
        !bpmux.tui().contains("Press any key to close"),
        "overlay should be dismissed. screen:\n{}",
        bpmux.tui()
    );
}
