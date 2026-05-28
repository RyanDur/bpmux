#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn room_spawn_creates_config_json_with_role_agent_pid_at_isolated_rooms_dir() {
    let rooms = TempDir::new().expect("tempdir");
    let bin = env!("CARGO_BIN_EXE_bpmux");

    let status = Command::new(bin)
        .args(["room", "spawn", "pm", "--agent", "/bin/sh", "--headless"])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .status()
        .expect("spawn bpmux room spawn");
    assert!(
        status.success(),
        "bpmux room spawn must exit 0; got {status}"
    );

    let config = rooms.path().join("pm").join("config.json");
    assert!(
        config.exists(),
        "AC(a): ~/.bpmux/rooms/<role>/config.json must exist; expected at {}",
        config.display()
    );

    let body = std::fs::read_to_string(&config).expect("read config.json");
    let parsed: Value = serde_json::from_str(&body).expect("config.json must be valid JSON");
    assert_eq!(parsed["role"].as_str(), Some("pm"));
    assert_eq!(parsed["agent"].as_str(), Some("/bin/sh"));
    let pid = parsed["pid"].as_u64().expect("pid field must be a number");
    assert!(pid > 0, "AC(b): a PTY child must be spawned; got pid={pid}");
    assert!(
        parsed["created_at"].is_string(),
        "created_at must be an RFC3339 timestamp string"
    );
}

#[test]
fn room_spawn_overwrites_a_previous_room_config_for_same_role() {
    let rooms = TempDir::new().expect("tempdir");
    let bin = env!("CARGO_BIN_EXE_bpmux");

    let first = Command::new(bin)
        .args(["room", "spawn", "dev", "--agent", "/bin/sh", "--headless"])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .status()
        .expect("first spawn");
    assert!(first.success());
    let body1 = std::fs::read_to_string(rooms.path().join("dev").join("config.json")).unwrap();
    let parsed1: Value = serde_json::from_str(&body1).unwrap();
    let pid1 = parsed1["pid"].as_u64().unwrap();

    let second = Command::new(bin)
        .args(["room", "spawn", "dev", "--agent", "/bin/sh", "--headless"])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .status()
        .expect("second spawn");
    assert!(second.success());
    let body2 = std::fs::read_to_string(rooms.path().join("dev").join("config.json")).unwrap();
    let parsed2: Value = serde_json::from_str(&body2).unwrap();
    let pid2 = parsed2["pid"].as_u64().unwrap();

    assert_ne!(
        pid1, pid2,
        "respawn must refresh config with the new child's PID; got identical pid {pid1}"
    );
}
