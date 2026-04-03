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

fn session_start_script() -> PathBuf {
    project_dir().join("claude-plugin/hooks/session-start.sh")
}

#[test]
fn hook_reads_jsonl_and_outputs_formatted_messages() {
    let tmp = tempfile::tempdir().unwrap();
    let sessions_dir = tmp.path().join(".bpmux/sessions");
    let session_dir = sessions_dir.join("test-session");
    fs::create_dir_all(&session_dir).unwrap();

    // Create the "current" symlink
    let current_link = sessions_dir.join("current");
    unix_fs::symlink(&session_dir, &current_link).unwrap();

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
