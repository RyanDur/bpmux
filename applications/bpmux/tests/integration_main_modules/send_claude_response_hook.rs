//! Integration test for #185 fix on `send-claude-response.sh`.
//!
//! The hook reads a bpmux messages JSON array and emits a
//! systemMessage JSON containing the FULL text of new messages
//! (not just a count, which was the pre-fix behavior). This test
//! exercises the real shell script by piping a fake messages JSON
//! via the `BPMUX_MESSAGES_STDIN=1` test hook, then parses the
//! output and asserts on the systemMessage body.
//!
//! Same pattern as `token_adapter_test.rs`: real script, real
//! subprocess, real stdin/stdout — no mocks. Naming is behavior-
//! framed because that's what the test actually verifies.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::process::{Command, Stdio};

fn hook_path() -> std::path::PathBuf {
    // Workspace root is two parents up from applications/bpmux, where
    // this crate's Cargo.toml sits; claude-plugin lives at the root.
    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    workspace_root
        .join("claude-plugin")
        .join("hooks")
        .join("send-claude-response.sh")
}

/// Run the hook with a fake messages JSON on stdin and a
/// pre-populated last-seen file. Returns (exit_code, stdout).
fn run_hook(messages_json: &str, last_seen_ts: Option<&str>) -> (i32, String) {
    let tmp = tempfile::tempdir().unwrap();
    let last_seen_path = tmp.path().join("last-seen");
    if let Some(ts) = last_seen_ts {
        std::fs::write(&last_seen_path, ts).unwrap();
    }

    let mut child = Command::new("bash")
        .arg(hook_path())
        .env("BPMUX_MESSAGES_STDIN", "1")
        .env("BPMUX_LAST_SEEN_FILE", &last_seen_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn hook");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(messages_json.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
    )
}

fn parse_system_message(stdout: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    v.get("systemMessage")?.as_str().map(String::from)
}

/// GIVEN a messages log with one new message after the last-seen
///   timestamp
/// WHEN the hook runs
/// THEN the systemMessage output contains the FULL text of that
///   message, with the sender role as a bracketed tag
#[test]
fn hook_injects_full_message_text_not_just_count() {
    let messages = r#"[
        {"from":"pm:claude-code","text":"shipped #170 accepted","ts":"2026-04-11T23:00:00Z","message_type":"chat"},
        {"from":"dev:claude-code","text":"shipped #181 focus mode display filter","ts":"2026-04-11T23:05:00Z","message_type":"chat"}
    ]"#;
    let (code, stdout) = run_hook(messages, Some("2026-04-11T23:00:00Z"));
    assert_eq!(code, 0, "hook should exit 0");
    let sys_msg = parse_system_message(&stdout).expect("systemMessage");
    assert!(
        sys_msg.contains("[dev]"),
        "should tag sender role: {sys_msg}"
    );
    assert!(
        sys_msg.contains("shipped #181 focus mode display filter"),
        "should include full text: {sys_msg}"
    );
    // Pre-fix behavior: only count appeared. Now count AND content.
    assert!(
        sys_msg.contains("1 new bpmux message"),
        "should retain count header: {sys_msg}"
    );
}

/// GIVEN multiple new messages after the last-seen timestamp
/// WHEN the hook runs
/// THEN all new messages appear in the systemMessage body,
///   one per block, in log order
#[test]
fn hook_injects_multiple_messages_in_log_order() {
    let messages = r#"[
        {"from":"ignored:before","text":"old message","ts":"2026-04-11T22:00:00Z","message_type":"chat"},
        {"from":"pm:claude-code","text":"first new message","ts":"2026-04-11T23:00:00Z","message_type":"chat"},
        {"from":"designer:claude-code","text":"second new message","ts":"2026-04-11T23:01:00Z","message_type":"chat"},
        {"from":"dev:claude-code","text":"third new message","ts":"2026-04-11T23:02:00Z","message_type":"chat"}
    ]"#;
    let (code, stdout) = run_hook(messages, Some("2026-04-11T22:30:00Z"));
    assert_eq!(code, 0);
    let sys_msg = parse_system_message(&stdout).expect("systemMessage");
    assert!(sys_msg.contains("3 new bpmux messages"));
    let pm_idx = sys_msg.find("first new message").expect("pm msg");
    let designer_idx = sys_msg.find("second new message").expect("designer msg");
    let dev_idx = sys_msg.find("third new message").expect("dev msg");
    assert!(pm_idx < designer_idx, "log order: pm before designer");
    assert!(designer_idx < dev_idx, "log order: designer before dev");
    // Sender tags present for each.
    assert!(sys_msg.contains("[pm]"));
    assert!(sys_msg.contains("[designer]"));
    assert!(sys_msg.contains("[dev]"));
}

/// GIVEN a messages log where the only new entries are ryan's own
///   messages (which the agent shouldn't echo-read)
/// WHEN the hook runs
/// THEN no systemMessage is emitted
#[test]
fn hook_is_silent_when_only_ryan_messages_are_new() {
    let messages = r#"[
        {"from":"pm:claude-code","text":"old message","ts":"2026-04-11T22:00:00Z","message_type":"chat"},
        {"from":"ryan:claude-code","text":"ryan just typed this","ts":"2026-04-11T23:00:00Z","message_type":"chat"}
    ]"#;
    let (code, stdout) = run_hook(messages, Some("2026-04-11T22:30:00Z"));
    assert_eq!(code, 0);
    // No output means no systemMessage injection — correct silent behavior.
    assert!(
        stdout.trim().is_empty(),
        "expected silent when only ryan messages are new, got: {stdout}"
    );
}

/// GIVEN a messages log where new messages are activity-type (not
///   chat), e.g. status updates from set-status-available.sh
/// WHEN the hook runs
/// THEN no systemMessage is emitted — activity is operational
///   noise, not content the agent needs in its context
#[test]
fn hook_is_silent_when_only_activity_messages_are_new() {
    let messages = r#"[
        {"from":"dev:claude-code","text":"old chat","ts":"2026-04-11T22:00:00Z","message_type":"chat"},
        {"from":"pm:claude-code","text":"available","ts":"2026-04-11T23:00:00Z","message_type":"activity"}
    ]"#;
    let (code, stdout) = run_hook(messages, Some("2026-04-11T22:30:00Z"));
    assert_eq!(code, 0);
    assert!(
        stdout.trim().is_empty(),
        "expected silent on activity-only new messages, got: {stdout}"
    );
}

/// GIVEN the hook is running for the first time (no last-seen file)
/// WHEN it processes a messages log
/// THEN it marks the position silently and doesn't inject anything
#[test]
fn hook_is_silent_on_first_run_without_last_seen_file() {
    let messages = r#"[
        {"from":"pm:claude-code","text":"a message","ts":"2026-04-11T23:00:00Z","message_type":"chat"}
    ]"#;
    let (code, stdout) = run_hook(messages, None);
    assert_eq!(code, 0);
    assert!(
        stdout.trim().is_empty(),
        "first run should mark position silently, got: {stdout}"
    );
}

/// GIVEN a message whose text contains characters that must be
///   JSON-escaped (newlines, quotes)
/// WHEN the hook runs
/// THEN the output is still a valid JSON document with the
///   content preserved in the systemMessage field
#[test]
fn hook_handles_newlines_and_quotes_in_message_text() {
    let messages = r#"[
        {"from":"pm:claude-code","text":"line one\nline two with \"quoted\" word","ts":"2026-04-11T23:00:00Z","message_type":"chat"}
    ]"#;
    let (code, stdout) = run_hook(messages, Some("2026-04-11T22:30:00Z"));
    assert_eq!(code, 0);
    let sys_msg = parse_system_message(&stdout).expect("systemMessage parses as JSON");
    assert!(sys_msg.contains("line one\nline two"));
    assert!(sys_msg.contains("\"quoted\""));
}
