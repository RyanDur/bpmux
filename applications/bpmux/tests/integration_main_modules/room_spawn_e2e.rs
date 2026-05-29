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

#[test]
fn room_list_enumerates_every_spawned_room() {
    let rooms = TempDir::new().expect("tempdir");
    let bin = env!("CARGO_BIN_EXE_bpmux");

    for role in ["pm", "dev"] {
        let status = Command::new(bin)
            .args(["room", "spawn", role, "--agent", "/bin/sh", "--headless"])
            .env("BPMUX_ROOMS_DIR", rooms.path())
            .env("HOME", rooms.path())
            .status()
            .expect("spawn bpmux room spawn");
        assert!(status.success(), "spawn {role} must exit 0; got {status}");
    }

    let output = Command::new(bin)
        .args(["room", "list"])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .output()
        .expect("run bpmux room list");
    assert!(
        output.status.success(),
        "bpmux room list must exit 0; got {}",
        output.status
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "AC(a): list must enumerate exactly the 2 spawned rooms; got {lines:?}"
    );
    assert_eq!(
        lines[0].split('\t').next(),
        Some("dev"),
        "rooms must enumerate sorted by role for deterministic scripted asserts; got {lines:?}"
    );
    assert_eq!(lines[1].split('\t').next(), Some("pm"), "got {lines:?}");
    for line in &lines {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            fields.len(),
            3,
            "each room line is role\\tagent\\tpid; got {line:?}"
        );
        assert_eq!(fields[1], "/bin/sh", "agent field; got {line:?}");
        assert!(
            fields[2].parse::<u32>().is_ok(),
            "pid field must be a number; got {line:?}"
        );
    }
}

#[test]
fn room_list_on_empty_rooms_dir_exits_zero_with_no_output() {
    let rooms = TempDir::new().expect("tempdir");
    let bin = env!("CARGO_BIN_EXE_bpmux");

    let output = Command::new(bin)
        .args(["room", "list"])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .output()
        .expect("run bpmux room list");

    assert!(
        output.status.success(),
        "AC(c): list on empty rooms-dir must exit 0 (clean, not error); got {}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.trim().is_empty(),
        "AC(c): no rooms means no output; got {stdout:?}"
    );
}
