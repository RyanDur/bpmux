#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

async fn run_verb(args: &[&str], env: &[(&str, &str)]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let mut cmd = tokio::process::Command::new(bin);
    cmd.args(args);
    for (key, value) in env {
        cmd.env(key, value);
    }
    cmd.output()
        .await
        .expect("run bpmux briefing verb subprocess")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_briefing_save_then_get_round_trips_off_mcp() {
    let briefings = tempfile::TempDir::new().expect("briefings dir");
    let briefings_str = briefings
        .path()
        .to_str()
        .expect("briefings path")
        .to_string();
    let body = "# dev briefing\n\nground-firsthand before any mutation.";
    let env: Vec<(&str, &str)> = vec![
        ("BPMUX_BRIEFINGS_DIR", briefings_str.as_str()),
        ("BPMUX_ROLE", "dev"),
    ];

    let saved = run_verb(&["briefing", "save", body], &env).await;
    assert!(
        saved.status.success(),
        "bpmux briefing save writes to the same crate::briefing store the bridge uses; stderr: {}",
        stderr(&saved)
    );

    let got = run_verb(&["briefing", "get"], &env).await;
    assert!(
        got.status.success(),
        "bpmux briefing get reads this role's briefing off MCP; stderr: {}",
        stderr(&got)
    );
    assert!(
        stdout(&got).contains("ground-firsthand before any mutation."),
        "briefing get surfaces exactly what briefing save wrote — a filesystem-direct round trip off MCP (non-vacuous: the saved body, not empty); stdout: {}",
        stdout(&got)
    );
}

#[tokio::test]
async fn bpmux_briefing_get_is_per_role_off_mcp() {
    let briefings = tempfile::TempDir::new().expect("briefings dir");
    let briefings_str = briefings
        .path()
        .to_str()
        .expect("briefings path")
        .to_string();

    let save_dev = run_verb(
        &["briefing", "save", "dev-only briefing marker"],
        &[
            ("BPMUX_BRIEFINGS_DIR", briefings_str.as_str()),
            ("BPMUX_ROLE", "dev"),
        ],
    )
    .await;
    assert!(
        save_dev.status.success(),
        "save dev briefing; stderr: {}",
        stderr(&save_dev)
    );

    let pm_view = run_verb(
        &["briefing", "get", "--role", "pm"],
        &[
            ("BPMUX_BRIEFINGS_DIR", briefings_str.as_str()),
            ("BPMUX_ROLE", "dev"),
        ],
    )
    .await;
    assert!(
        pm_view.status.success(),
        "get pm briefing; stderr: {}",
        stderr(&pm_view)
    );
    assert!(
        !stdout(&pm_view).contains("dev-only briefing marker"),
        "briefing is per-role: pm's briefing must not leak dev's content (BPMUX_ROLE keys the store); stdout: {}",
        stdout(&pm_view)
    );
}
