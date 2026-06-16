#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Stdio;
use std::time::{Duration, Instant};

use crate::harness::pty::TestBpmux;

fn plugin_hooks_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../claude-plugin/hooks")
}

fn commands_for(hooks_json: &serde_json::Value, event: &str) -> Vec<String> {
    hooks_json["hooks"][event]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .flat_map(|block| block["hooks"].as_array().cloned().unwrap_or_default())
                .filter_map(|hook| hook["command"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

async fn roster_presence_for_role(port: u16, secret: &str, role: &str) -> Option<String> {
    let url = format!("http://127.0.0.1:{port}/api/roster");
    let body: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .bearer_auth(secret)
        .header(reqwest::header::HOST, format!("127.0.0.1:{port}"))
        .send()
        .await
        .expect("GET /api/roster")
        .json()
        .await
        .expect("roster json");
    body["roster"].as_array().and_then(|arr| {
        arr.iter()
            .find(|entry| entry["role"].as_str() == Some(role))
            .and_then(|entry| entry["presence"].as_str().map(str::to_string))
    })
}

#[test]
fn the_shipped_hook_bundle_wires_the_1739_presence_model_with_zero_per_agent_setup() {
    let hooks_dir = plugin_hooks_dir();
    let raw = std::fs::read_to_string(hooks_dir.join("hooks.json")).expect("read hooks.json");
    let hooks_json: serde_json::Value =
        serde_json::from_str(&raw).expect("hooks.json is valid json");

    let wired = |event: &str, state: &str| {
        commands_for(&hooks_json, event)
            .iter()
            .any(|command| command.contains("signal-presence.sh") && command.ends_with(state))
    };
    assert!(
        wired("UserPromptSubmit", "busy"),
        "turn-start (UserPromptSubmit) auto-pushes busy via the shipped bundle — wired in the plugin's hooks.json so every agent of a supported type gets it with no per-agent setup; commands: {:?}",
        commands_for(&hooks_json, "UserPromptSubmit")
    );
    assert!(
        wired("Stop", "idle"),
        "definitive turn-end (Stop) auto-pushes idle; commands: {:?}",
        commands_for(&hooks_json, "Stop")
    );
    assert!(
        wired("Notification", "help"),
        "needs-attention (Notification) auto-pushes needs-help — the agent-reachable Help path, zero setup; commands: {:?}",
        commands_for(&hooks_json, "Notification")
    );
    assert!(
        !raw.contains("set-status"),
        "the legacy set-status-*.sh hooks are retired from the bundle (not left as a competing path)"
    );
}

#[test]
fn the_presence_hook_pushes_off_chat_via_s1s_gated_verb() {
    let hooks_dir = plugin_hooks_dir();
    let script = std::fs::read_to_string(hooks_dir.join("signal-presence.sh"))
        .expect("read signal-presence.sh");
    assert!(
        script.contains("bpmux signal"),
        "the hook pushes presence via S1's gated `bpmux signal` verb (-> /api/presence), not a raw chat POST"
    );
    assert!(
        !script.contains("/api/messages"),
        "the hook does NOT POST presence into the chat stream — the chat-presence path is retired (no spam)"
    );
    assert!(
        !script.contains("message_type"),
        "no activity-message presence in the chat stream"
    );
}

#[tokio::test]
async fn running_the_real_presence_hook_script_sets_the_agents_roster_dot() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let secret = bpmux.simulate_secret();
    let token_path = bpmux.auth_token_path();

    let registered = bpmux
        .authed_post(
            "/api/register",
            serde_json::json!({ "session_id": "sess-dev-hook", "display": "dev" }),
        )
        .await;
    assert!(
        registered.status().is_success(),
        "precondition: dev registers so the roster has a member to colour"
    );

    let shim = tempfile::TempDir::new().expect("shim dir");
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_bpmux"), shim.path().join("bpmux"))
        .expect("symlink the freshly-built bin onto PATH as `bpmux`");
    let path_env = format!(
        "{}:{}",
        shim.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let script = plugin_hooks_dir().join("signal-presence.sh");
    let output = tokio::process::Command::new("bash")
        .arg(&script)
        .arg("busy")
        .env("PATH", &path_env)
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .env("BPMUX_AUTH_TOKEN_PATH", &token_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("run signal-presence.sh");
    assert!(
        output.status.success(),
        "the hook is best-effort and always exits 0 so a failed push never blocks the turn"
    );

    let mut got = None;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        got = roster_presence_for_role(port, &secret, "dev").await;
        if got.as_deref() == Some("Busy") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(
        got.as_deref(),
        Some("Busy"),
        "running the REAL UserPromptSubmit hook (signal-presence.sh busy) set dev's roster dot to Busy via S1's /api/presence — the shipped bundle auto-pushes presence through the actual script, off the chat stream"
    );
}
