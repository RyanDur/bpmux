#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::Value;
use tempfile::TempDir;

fn write_marker_agent(dir: &std::path::Path, name: &str, marker: &str) -> std::path::PathBuf {
    write_agent(dir, name, &format!("echo {marker}"))
}

fn write_ansi_agent(dir: &std::path::Path, name: &str, marker: &str) -> std::path::PathBuf {
    write_agent(dir, name, &format!("printf '\\033[31m{marker}\\033[0m\\n'"))
}

fn write_agent(dir: &std::path::Path, name: &str, command: &str) -> std::path::PathBuf {
    let script = dir.join(name);
    std::fs::write(&script, format!("#!/bin/sh\n{command}\n")).expect("write agent script");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("chmod agent script");
    script
}

fn wait_for_capture(log: &std::path::Path, marker: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(body) = std::fs::read_to_string(log)
            && body.contains(marker)
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "marker {marker:?} never captured to {}",
            log.display()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

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
    assert!(pid > 0, "AC(b): a child must be spawned; got pid={pid}");
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

#[test]
fn room_read_returns_the_spawned_agents_captured_stdout() {
    let rooms = TempDir::new().expect("tempdir");
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let agent = write_ansi_agent(rooms.path(), "emit.sh", "ROOM_READ_MARKER_pm");

    let status = Command::new(bin)
        .args([
            "room",
            "spawn",
            "pm",
            "--agent",
            agent.to_str().expect("agent path utf8"),
            "--headless",
        ])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .status()
        .expect("spawn marker agent");
    assert!(status.success(), "spawn must exit 0; got {status}");

    let log = rooms.path().join("pm").join("output.log");
    wait_for_capture(&log, "ROOM_READ_MARKER_pm");
    let raw = std::fs::read_to_string(&log).expect("read raw output.log");
    assert!(
        raw.contains('\u{1b}'),
        "precondition: the ANSI agent must have emitted a real escape into the raw log, \
         else the strip assertion below is vacuous; got {raw:?}"
    );

    let output = Command::new(bin)
        .args(["room", "read", "pm", "--lines", "20"])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .output()
        .expect("run room read");
    assert!(
        output.status.success(),
        "room read must exit 0; got {}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("ROOM_READ_MARKER_pm"),
        "AC(a): room read must return the agent's captured stdout; got {stdout:?}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "AC(c): the raw log carried an escape, room read must strip it (load-bearing on strip_ansi wiring); got {stdout:?}"
    );
}

#[test]
fn room_read_returns_only_the_named_rooms_output_no_cross_room_bleed() {
    let rooms = TempDir::new().expect("tempdir");
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let pm_agent = write_marker_agent(rooms.path(), "pm.sh", "MARK_PM_ONLY");
    let dev_agent = write_marker_agent(rooms.path(), "dev.sh", "MARK_DEV_ONLY");

    for (role, agent) in [("pm", &pm_agent), ("dev", &dev_agent)] {
        let status = Command::new(bin)
            .args([
                "room",
                "spawn",
                role,
                "--agent",
                agent.to_str().expect("agent path utf8"),
                "--headless",
            ])
            .env("BPMUX_ROOMS_DIR", rooms.path())
            .env("HOME", rooms.path())
            .status()
            .expect("spawn marker agent");
        assert!(status.success(), "spawn {role} must exit 0; got {status}");
    }
    wait_for_capture(&rooms.path().join("pm").join("output.log"), "MARK_PM_ONLY");
    wait_for_capture(
        &rooms.path().join("dev").join("output.log"),
        "MARK_DEV_ONLY",
    );

    let output = Command::new(bin)
        .args(["room", "read", "pm", "--lines", "20"])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .output()
        .expect("run room read");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("MARK_PM_ONLY"),
        "AC(b): pm read must contain pm's output; got {stdout:?}"
    );
    assert!(
        !stdout.contains("MARK_DEV_ONLY"),
        "AC(b): pm read must NOT bleed dev's output; got {stdout:?}"
    );
}

#[test]
fn room_read_on_a_non_spawned_room_errors_clearly() {
    let rooms = TempDir::new().expect("tempdir");
    let bin = env!("CARGO_BIN_EXE_bpmux");

    let output = Command::new(bin)
        .args(["room", "read", "ghost", "--lines", "20"])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .output()
        .expect("run room read");
    assert!(
        !output.status.success(),
        "AC(d): reading a non-spawned room must error (non-zero exit); got {}",
        output.status
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(
        stderr.contains("ghost"),
        "AC(d): error must clearly name the missing room; got {stderr:?}"
    );
}

#[test]
fn room_read_on_a_spawned_but_silent_room_is_a_clean_empty_not_an_error() {
    let rooms = TempDir::new().expect("tempdir");
    let bin = env!("CARGO_BIN_EXE_bpmux");

    let status = Command::new(bin)
        .args(["room", "spawn", "pm", "--agent", "/bin/sh", "--headless"])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .status()
        .expect("spawn silent agent");
    assert!(status.success(), "spawn must exit 0; got {status}");

    let output = Command::new(bin)
        .args(["room", "read", "pm"])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .output()
        .expect("run room read");
    assert!(
        output.status.success(),
        "AC(d): a spawned-but-silent room reads clean (exit 0), not an error; got {}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.trim().is_empty(),
        "spawned-but-silent room has no captured output yet; got {stdout:?}"
    );
}

#[test]
fn room_read_after_respawn_does_not_return_the_prior_childs_output() {
    let rooms = TempDir::new().expect("tempdir");
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let log = rooms.path().join("pm").join("output.log");

    let old_agent = write_marker_agent(rooms.path(), "old.sh", "STALE_OLD_MARKER");
    let status = Command::new(bin)
        .args([
            "room",
            "spawn",
            "pm",
            "--agent",
            old_agent.to_str().expect("agent path utf8"),
            "--headless",
        ])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .status()
        .expect("first spawn");
    assert!(status.success(), "first spawn must exit 0; got {status}");
    wait_for_capture(&log, "STALE_OLD_MARKER");

    let new_agent = write_marker_agent(rooms.path(), "new.sh", "FRESH_NEW_MARKER");
    let status = Command::new(bin)
        .args([
            "room",
            "spawn",
            "pm",
            "--agent",
            new_agent.to_str().expect("agent path utf8"),
            "--headless",
        ])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .status()
        .expect("respawn");
    assert!(status.success(), "respawn must exit 0; got {status}");
    wait_for_capture(&log, "FRESH_NEW_MARKER");

    let output = Command::new(bin)
        .args(["room", "read", "pm"])
        .env("BPMUX_ROOMS_DIR", rooms.path())
        .env("HOME", rooms.path())
        .output()
        .expect("run room read");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("FRESH_NEW_MARKER"),
        "respawn's output must be readable; got {stdout:?}"
    );
    assert!(
        !stdout.contains("STALE_OLD_MARKER"),
        "AC(b): respawn must truncate the prior child's captured output, not append; got {stdout:?}"
    );
}
