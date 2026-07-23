#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn project_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
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

fn create_data_home(tmp: &tempfile::TempDir) -> PathBuf {
    let data_base = tmp.path().join(".bpmux/data");
    fs::create_dir_all(&data_base).unwrap();
    data_base
}

#[test]
fn hook_reads_jsonl_and_outputs_formatted_messages() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = create_data_home(&tmp);

    let jsonl = r#"{"from":"alice","text":"hello world","ts":"2026-04-03T12:00:00Z"}
{"from":"bob","text":"how are you","ts":"2026-04-03T12:01:00Z"}
{"from":"alice","text":"doing great","ts":"2026-04-03T12:02:00Z"}
"#;
    fs::write(data_dir.join("messages-current.jsonl"), jsonl).unwrap();

    let output = Command::new("bash")
        .arg(check_bpmux_script())
        .env("HOME", tmp.path())
        .env_remove("BPMUX_DATA_BASE")
        .output()
        .unwrap();

    assert!(output.status.success(), "script exited non-zero");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bpmux:"), "missing bpmux header: {stdout}");
    assert!(
        stdout.contains("new"),
        "missing new message indicator: {stdout}"
    );
    assert!(stdout.contains("alice"), "missing latest sender: {stdout}");
}

#[test]
fn hook_outputs_nothing_when_no_session_exists() {
    let tmp = tempfile::tempdir().unwrap();

    let output = Command::new("bash")
        .arg(check_bpmux_script())
        .env("HOME", tmp.path())
        .env_remove("BPMUX_DATA_BASE")
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
    let tmp = tempfile::tempdir().unwrap();
    let output = Command::new("bash")
        .arg(session_start_script())
        .env("HOME", tmp.path())
        .env_remove("BPMUX_DATA_BASE")
        .env_remove("BPMUX_ROLE")
        .env_remove("BPMUX_BIN_OVERRIDE")
        .output()
        .unwrap();

    assert!(output.status.success(), "script exited non-zero");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "session-start should produce output");
    assert!(
        stdout.contains("bpmux"),
        "output should mention bpmux: {stdout}"
    );
    assert!(
        !tmp.path().join(".bpmux").exists(),
        "with no role, session-start performs no register — nothing is written under the data home"
    );
}

#[test]
fn session_start_register_mints_the_identity_into_the_workspace_marker() {
    let home = tempfile::tempdir().unwrap();
    let data_base = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();

    let output = Command::new("bash")
        .arg(session_start_script())
        .current_dir(workspace.path())
        .env("HOME", home.path())
        .env("BPMUX_ROLE", "dev")
        .env("BPMUX_DATA_BASE", data_base.path())
        .env("BPMUX_BIN_OVERRIDE", env!("CARGO_BIN_EXE_bpmux"))
        .env("BPMUX_PORT", "19999")
        .env_remove("BPMUX_USER_ID")
        .env_remove("BPMUX_TERMINAL_ID")
        .output()
        .unwrap();

    assert!(output.status.success(), "script exited non-zero");
    assert!(
        !home.path().join(".bpmux").exists(),
        "the hook's register must NEVER fall through to the DEFAULT data home when a test runs it — identity I/O lands in the injected BPMUX_DATA_BASE (this is the leak class that minted a real ~/.bpmux/data/users/by-dir entry during a test run; the sibling network leak registered temp-minted strangers against the LIVE daemon through the ambient BPMUX_PORT and linked them over the owner's real terminal via the ambient BPMUX_TERMINAL_ID)"
    );
    assert!(
        workspace.path().join(".bpmux-user-id").exists(),
        "the hook's register mints the uuid into the workspace's own marker — the dir IS the agent"
    );
    assert!(
        !data_base.path().join("users").join("by-dir").exists(),
        "no central registry entry is created anywhere"
    );
}

#[test]
fn send_user_message_exits_cleanly_when_server_unavailable() {
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
            let _ = child.stdin.take().unwrap().write_all(input.as_bytes());
            child.wait_with_output()
        })
        .unwrap();

    assert!(output.status.success(), "script should exit 0");
}

#[test]
fn send_claude_response_exits_cleanly_when_server_unavailable() {
    let output = Command::new("bash")
        .arg(send_claude_response_script())
        .env("BPMUX_PORT", "19999")
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
    let tmp = tempfile::tempdir().unwrap();
    let output = Command::new("bash")
        .arg(send_claude_response_script())
        .env("HOME", tmp.path())
        .env("BPMUX_PORT", "19999")
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
    let manifest_path = project_dir().join("claude-plugin/.claude-plugin/plugin.json");
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
