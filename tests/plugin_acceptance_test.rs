//! Plugin Acceptance Test: Claude Code plugin hooks and manifest
//!
//! Proves: the UserPromptSubmit hook reads the JSONL log and formats
//! messages for Claude, the hook exits cleanly when no session exists,
//! the SessionStart hook outputs context, and the plugin manifest is
//! valid JSON.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::PathBuf;
use std::process::Command;

fn project_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn check_bpmux_script() -> PathBuf {
    project_dir().join("claude-plugin/hooks/check-bpmux.sh")
}

fn send_user_message_script() -> PathBuf {
    project_dir().join("claude-plugin/hooks/send-user-message.sh")
}

fn send_claude_response_script() -> PathBuf {
    project_dir().join("claude-plugin/hooks/send-claude-response.sh")
}

fn session_start_script() -> PathBuf {
    project_dir().join("claude-plugin/hooks/session-start.sh")
}

fn create_session_dir(tmp: &tempfile::TempDir) -> PathBuf {
    let sessions_dir = tmp.path().join(".bpmux/sessions");
    let session_dir = sessions_dir.join("test-session");
    fs::create_dir_all(&session_dir).unwrap();

    let current_link = sessions_dir.join("current");
    unix_fs::symlink(&session_dir, &current_link).unwrap();

    session_dir
}

#[test]
fn hook_reads_jsonl_and_outputs_formatted_messages() {
    let tmp = tempfile::tempdir().unwrap();
    let session_dir = create_session_dir(&tmp);

    // Write JSONL messages (using "from" field matching session::Message)
    let jsonl = r#"{"from":"alice","text":"hello world","ts":"2026-04-03T12:00:00Z"}
{"from":"bob","text":"how are you","ts":"2026-04-03T12:01:00Z"}
{"from":"alice","text":"doing great","ts":"2026-04-03T12:02:00Z"}
"#;
    fs::write(session_dir.join("messages-current.jsonl"), jsonl).unwrap();

    let output = Command::new("bash")
        .arg(check_bpmux_script())
        .env("HOME", tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "script exited non-zero");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bpmux:"), "missing bpmux header: {stdout}");
    assert!(
        stdout.contains("recent messages"),
        "missing message count: {stdout}"
    );
    assert!(stdout.contains("alice"), "missing latest sender: {stdout}");
    assert!(
        stdout.contains("read_messages"),
        "should mention read_messages tool: {stdout}"
    );
}

#[test]
fn hook_outputs_nothing_when_no_session_exists() {
    let tmp = tempfile::tempdir().unwrap();

    let output = Command::new("bash")
        .arg(check_bpmux_script())
        .env("HOME", tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "script should exit 0");
    assert!(
        output.stdout.is_empty(),
        "expected no output, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn session_start_outputs_context() {
    let output = Command::new("bash")
        .arg(session_start_script())
        .output()
        .unwrap();

    assert!(output.status.success(), "script exited non-zero");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "session-start should produce output");
    assert!(
        stdout.contains("bpmux"),
        "output should mention bpmux: {stdout}"
    );
}

#[test]
fn send_user_message_exits_cleanly_when_server_unavailable() {
    // The script POSTs to bpmux REST endpoint — when bpmux isn't
    // running, curl fails silently and the script still exits 0.
    let input = r#"{"prompt":"what does this function do?"}"#;

    let output = Command::new("bash")
        .arg(send_user_message_script())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            child.wait_with_output()
        })
        .unwrap();

    assert!(
        output.status.success(),
        "script should exit 0 even when bpmux is unavailable"
    );
}

#[test]
fn send_user_message_silent_when_no_session() {
    let tmp = tempfile::tempdir().unwrap();

    let input = r#"{"prompt":"hello"}"#;

    let output = Command::new("bash")
        .arg(send_user_message_script())
        .env("HOME", tmp.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            child.wait_with_output()
        })
        .unwrap();

    assert!(output.status.success(), "script should exit 0");
}

#[test]
fn send_claude_response_exits_cleanly_when_server_unavailable() {
    // The Stop hook POSTs a turn-complete notification to bpmux REST
    // endpoint. When bpmux isn't running, curl fails silently.
    // NOTE: Stop hooks do NOT receive Claude's response text — only
    // common fields. The script sends a "[turn complete]" notification.
    let output = Command::new("bash")
        .arg(send_claude_response_script())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "script should exit 0 even when bpmux is unavailable"
    );
}

#[test]
fn send_claude_response_produces_no_stdout() {
    // The Stop hook should not produce stdout — it only curls the endpoint.
    let output = Command::new("bash")
        .arg(send_claude_response_script())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success(), "script should exit 0");
    assert!(
        output.stdout.is_empty(),
        "stop hook should produce no stdout, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn hooks_json_includes_stop_hook() {
    let hooks_path = project_dir().join("claude-plugin/hooks/hooks.json");
    let content = fs::read_to_string(&hooks_path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert!(
        value["hooks"]["Stop"].is_array(),
        "hooks.json should have a Stop event"
    );

    let stop_hooks = &value["hooks"]["Stop"][0]["hooks"];
    assert!(
        stop_hooks.is_array() && !stop_hooks.as_array().unwrap().is_empty(),
        "Stop should have at least one hook"
    );
}

#[test]
fn plugin_manifest_is_valid_json() {
    let manifest_path = project_dir().join("claude-plugin/plugin.json");
    let content = fs::read_to_string(&manifest_path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(value["name"], "bpmux");
    assert!(
        value["description"].is_string(),
        "manifest should have a description"
    );
    assert!(
        value["version"].is_string(),
        "manifest should have a version"
    );
}
