//! Story #77 — An agent can pin and unpin decisions via MCP tools
//!
//! AC1: Agent pins via REST → appears in GET /api/pins
//! AC2: Duplicate pin (case-insensitive) rejected with 409
//! AC3: Unpin by index removes pin
//! AC4: Max 5 pins enforced
//! AC5: Pin/unpin broadcast system messages

mod harness;
use harness::pty::TestBpmux;
use std::time::Duration;

/// POST a pin via REST and verify it appears.
#[test]
fn ac1_agent_pins_via_rest() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();

    // POST pin
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            &format!("http://127.0.0.1:{port}/api/pins"),
            "-H",
            "Content-Type: application/json",
            "-d",
            r#"{"text":"channels-as-views","from":"dev:claude-code"}"#,
        ])
        .output()
        .expect("curl");
    let body = String::from_utf8_lossy(&output.stdout);
    assert!(body.contains("pinned #1"), "should confirm pin: {body}");

    // GET pins
    let output = std::process::Command::new("curl")
        .args(["-s", &format!("http://127.0.0.1:{port}/api/pins")])
        .output()
        .expect("curl");
    let body = String::from_utf8_lossy(&output.stdout);
    assert!(
        body.contains("channels-as-views"),
        "GET should return pin: {body}"
    );
}

/// Duplicate pin (case-insensitive) returns 409.
#[test]
fn ac2_duplicate_pin_rejected() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();

    let pin = |text: &str| -> String {
        let output = std::process::Command::new("curl")
            .args([
                "-s",
                "-X",
                "POST",
                &format!("http://127.0.0.1:{port}/api/pins"),
                "-H",
                "Content-Type: application/json",
                "-d",
                &format!(r#"{{"text":"{text}","from":"dev:claude-code"}}"#),
                "-w",
                "\n%{http_code}",
            ])
            .output()
            .expect("curl");
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    let first = pin("test-decision");
    assert!(first.contains("201"), "first pin should succeed: {first}");

    let dup = pin("Test-Decision");
    assert!(dup.contains("409"), "duplicate should return 409: {dup}");
    assert!(
        dup.contains("already pinned"),
        "should say already pinned: {dup}"
    );
}

/// Unpin by index removes pin, returns 200.
#[test]
fn ac3_unpin_removes_by_index() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();

    // Add pin
    std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            &format!("http://127.0.0.1:{port}/api/pins"),
            "-H",
            "Content-Type: application/json",
            "-d",
            r#"{"text":"to-remove","from":"dev:claude-code"}"#,
        ])
        .output()
        .expect("curl");

    // Delete pin #1
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "DELETE",
            &format!("http://127.0.0.1:{port}/api/pins/1?from=dev:claude-code"),
            "-w",
            "\n%{http_code}",
        ])
        .output()
        .expect("curl");
    let body = String::from_utf8_lossy(&output.stdout);
    assert!(body.contains("200"), "unpin should return 200: {body}");

    // Verify empty
    let output = std::process::Command::new("curl")
        .args(["-s", &format!("http://127.0.0.1:{port}/api/pins")])
        .output()
        .expect("curl");
    let body = String::from_utf8_lossy(&output.stdout);
    assert!(body.contains("[]"), "pins should be empty: {body}");
}

/// Pin action broadcasts system message visible in TUI.
#[test]
fn ac5_pin_broadcasts_to_tui() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();

    // Pin via REST
    std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            &format!("http://127.0.0.1:{port}/api/pins"),
            "-H",
            "Content-Type: application/json",
            "-d",
            r#"{"text":"shared-decision","from":"dev:claude-code"}"#,
        ])
        .output()
        .expect("curl");

    // The system message should appear in the TUI (debug mode shows system messages)
    // Send /debug to enable system messages, then check
    bpmux.type_line("/debug");
    std::thread::sleep(Duration::from_millis(200));
    let found = bpmux.wait_for_tui("pinned #1", Duration::from_secs(3));
    assert!(
        found,
        "TUI should show pin system message. TUI:\n{}",
        bpmux.tui()
    );
}
