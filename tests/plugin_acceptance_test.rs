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

    // Write JSONL messages
    let jsonl = r#"{"sender":"alice","text":"hello world"}
{"sender":"bob","text":"how are you"}
{"sender":"alice","text":"doing great"}
"#;
    fs::write(session_dir.join("messages-current.jsonl"), jsonl).unwrap();

    let output = Command::new("bash")
        .arg(check_bpmux_script())
        .env("HOME", tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "script exited non-zero");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--- Recent bpmux messages ---"),
        "missing header: {stdout}"
    );
    assert!(
        stdout.contains("[alice] hello world"),
        "missing alice's first message: {stdout}"
    );
    assert!(
        stdout.contains("[bob] how are you"),
        "missing bob's message: {stdout}"
    );
    assert!(
        stdout.contains("[alice] doing great"),
        "missing alice's second message: {stdout}"
    );
    assert!(
        stdout.contains("--- End bpmux messages ---"),
        "missing footer: {stdout}"
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
fn send_user_message_appends_to_jsonl() {
    let tmp = tempfile::tempdir().unwrap();
    let session_dir = create_session_dir(&tmp);

    // Create an empty messages file
    let messages_file = session_dir.join("messages-current.jsonl");
    fs::write(&messages_file, "").unwrap();

    let input = r#"{"prompt":"what does this function do?"}"#;

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

    assert!(output.status.success(), "script exited non-zero");

    let content = fs::read_to_string(&messages_file).unwrap();
    let line = content.lines().next().expect("should have one line");
    let msg: serde_json::Value = serde_json::from_str(line).expect("valid JSON");

    assert_eq!(msg["from"], "ryan:claude-code-user");
    assert_eq!(msg["text"], "what does this function do?");
    assert!(msg["ts"].is_string(), "should have a timestamp");
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
fn send_claude_response_appends_to_jsonl() {
    let tmp = tempfile::tempdir().unwrap();
    let session_dir = create_session_dir(&tmp);

    let messages_file = session_dir.join("messages-current.jsonl");
    fs::write(&messages_file, "").unwrap();

    let input = r#"{"response":"The function calculates the sum of two integers."}"#;

    let output = Command::new("bash")
        .arg(send_claude_response_script())
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

    assert!(output.status.success(), "script exited non-zero");

    let content = fs::read_to_string(&messages_file).unwrap();
    let line = content.lines().next().expect("should have one line");
    let msg: serde_json::Value = serde_json::from_str(line).expect("valid JSON");

    assert_eq!(msg["from"], "ryan:claude-code");
    assert_eq!(
        msg["text"],
        "The function calculates the sum of two integers."
    );
    assert!(msg["ts"].is_string(), "should have a timestamp");
}

#[test]
fn send_claude_response_silent_when_no_session() {
    let tmp = tempfile::tempdir().unwrap();

    let input = r#"{"response":"hello"}"#;

    let output = Command::new("bash")
        .arg(send_claude_response_script())
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
