use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;

use crate::harness::pty::TestBpmux;

fn receive_hook_script() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root two parents up")
        .join("claude-plugin/hooks/receive-broadcast.sh")
}

fn path_with_bpmux_bin() -> String {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let dir = std::path::Path::new(bin)
        .parent()
        .expect("bpmux bin has a parent dir")
        .to_string_lossy()
        .into_owned();
    format!("{dir}:{}", std::env::var("PATH").unwrap_or_default())
}

#[tokio::test]
async fn receive_broadcast_stop_hook_delivers_a_live_broadcast_as_block_reason() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let tmp = tempfile::tempdir().expect("tmpdir");

    let mut hook = tokio::process::Command::new("bash")
        .arg(receive_hook_script())
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .env("PATH", path_with_bpmux_bin())
        .env("TMPDIR", tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn receive-broadcast.sh");
    hook.stdin
        .take()
        .expect("hook stdin")
        .write_all(b"{}")
        .await
        .expect("write empty hook stdin");

    tokio::time::sleep(Duration::from_millis(200)).await;
    let posted = bpmux
        .authed_post(
            "/api/messages",
            json!({ "from": "pm", "text": "standup in 5" }),
        )
        .await;
    assert!(
        posted.status().is_success(),
        "broadcast POST should succeed, got {}",
        posted.status()
    );

    let output = hook
        .wait_with_output()
        .await
        .expect("collect receive-broadcast.sh output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let decision: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|_| panic!("hook should emit a JSON decision; stdout: {stdout}"));

    assert_eq!(
        decision["decision"], "block",
        "the Stop hook must BLOCK the stop to deliver the broadcast received-and-processed (not parked); stdout: {stdout}"
    );
    let reason = decision["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("⟦bpmux⟧"),
        "the reason must carry the peer-untrusted marker; reason: {reason}"
    );
    assert!(
        reason.contains("pm"),
        "the reason must attribute the sender (peer-not-owner); reason: {reason}"
    );
    assert!(
        reason.contains("standup in 5"),
        "the reason must carry the broadcast text; reason: {reason}"
    );
}

#[tokio::test]
async fn receive_broadcast_stop_hook_allows_stop_when_already_blocking() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let tmp = tempfile::tempdir().expect("tmpdir");

    let mut hook = tokio::process::Command::new("bash")
        .arg(receive_hook_script())
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .env("PATH", path_with_bpmux_bin())
        .env("TMPDIR", tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn receive-broadcast.sh");
    hook.stdin
        .take()
        .expect("hook stdin")
        .write_all(b"{\"stop_hook_active\":true}")
        .await
        .expect("write hook stdin");

    let output = hook
        .wait_with_output()
        .await
        .expect("collect receive-broadcast.sh output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "when stop_hook_active is true the hook must NOT re-block (no decision output) — loop guard; stdout: {stdout}"
    );
}

#[tokio::test]
async fn receive_broadcast_stop_hook_does_not_deliver_a_message_scoped_to_another_role() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let tmp = tempfile::tempdir().expect("tmpdir");

    let mut hook = tokio::process::Command::new("bash")
        .arg(receive_hook_script())
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .env("PATH", path_with_bpmux_bin())
        .env("TMPDIR", tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn receive-broadcast.sh");
    hook.stdin
        .take()
        .expect("hook stdin")
        .write_all(b"{}")
        .await
        .expect("write empty hook stdin");

    tokio::time::sleep(Duration::from_millis(200)).await;
    let posted = bpmux
        .authed_post(
            "/api/messages",
            json!({ "from": "pm", "text": "for designer only", "to": ["designer"] }),
        )
        .await;
    assert!(
        posted.status().is_success(),
        "scoped POST should succeed, got {}",
        posted.status()
    );

    let output = hook
        .wait_with_output()
        .await
        .expect("collect receive-broadcast.sh output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "a to:[designer] broadcast must NOT reach role=dev (no-leak, falsifier #4): the hook filters it via should_deliver, times out, and emits NO decision:block; stdout: {stdout}"
    );
}
