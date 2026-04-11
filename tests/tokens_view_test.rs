//! End-to-end test for the `/tokens` TUI overlay (#179).
//!
//! This is the BEHAVIOR layer of the `/tokens` feature. The unit
//! tests on `format_tokens_overlay` (in `src/terminal/commands_tokens.rs`)
//! exercise the pure string-building function; this test drives the
//! real `bpmux` binary in a PTY, pre-populates a fake token log in a
//! tempdir via `BPMUX_TOKENS_DIR`, types `/tokens`, and asserts on
//! the rendered screen buffer.
//!
//! Named in behavior terms because the content actually IS behavior —
//! key press → command dispatch → log read → overlay render → frame —
//! per the test-name discipline in
//! `.claude/memory/feedback_test_name_matches_content.md`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

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

/// GIVEN a token log with rows for multiple roles
/// WHEN the user types `/tokens` in the TUI
/// THEN the overlay renders with role totals and the close hint
#[test]
fn tokens_command_renders_overlay_with_role_breakdown() {
    let tmp = tempfile::tempdir().unwrap();
    write_fake_log(tmp.path());

    let bpmux =
        TestBpmux::start_with_env("human", "BPMUX_TOKENS_DIR", tmp.path().to_str().unwrap());
    bpmux.type_line("/tokens");

    assert!(
        bpmux.wait_for_tui("Press any key to close", Duration::from_secs(5)),
        "tokens overlay should render. screen:\n{}",
        bpmux.tui()
    );
    let screen = bpmux.tui();
    // Overlay frame and role breakdown.
    assert!(screen.contains("tokens"), "title missing:\n{screen}");
    assert!(
        screen.contains("By role:"),
        "role section missing:\n{screen}"
    );
    assert!(screen.contains("dev"), "dev role missing:\n{screen}");
    assert!(screen.contains("pm"), "pm role missing:\n{screen}");
}

/// GIVEN no token log exists yet
/// WHEN the user types `/tokens`
/// THEN the overlay shows the empty-state placeholder
#[test]
fn tokens_command_shows_empty_state_when_log_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    // Note: no file written — the tempdir is empty.
    let bpmux =
        TestBpmux::start_with_env("human", "BPMUX_TOKENS_DIR", tmp.path().to_str().unwrap());
    bpmux.type_line("/tokens");

    assert!(
        bpmux.wait_for_tui("No token rows yet", Duration::from_secs(5)),
        "empty-state placeholder should render. screen:\n{}",
        bpmux.tui()
    );
}

/// GIVEN the `/tokens` overlay is visible
/// WHEN the user presses any key
/// THEN the overlay closes
#[test]
fn tokens_overlay_dismisses_on_any_key() {
    let tmp = tempfile::tempdir().unwrap();
    write_fake_log(tmp.path());
    let bpmux =
        TestBpmux::start_with_env("human", "BPMUX_TOKENS_DIR", tmp.path().to_str().unwrap());
    bpmux.type_line("/tokens");
    assert!(
        bpmux.wait_for_tui("Press any key to close", Duration::from_secs(5)),
        "overlay should be visible. screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_chars("x");
    // Small settle window before asserting the overlay is gone.
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !bpmux.tui().contains("Press any key to close"),
        "overlay should be dismissed. screen:\n{}",
        bpmux.tui()
    );
}
