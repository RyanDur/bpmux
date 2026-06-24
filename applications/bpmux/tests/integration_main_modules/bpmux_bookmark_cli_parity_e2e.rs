#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

async fn run_verb(
    role: &str,
    sessions_base: &std::path::Path,
    args: &[&str],
) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(args)
        .env("BPMUX_ROLE", role)
        .env("BPMUX_SESSIONS_BASE", sessions_base)
        .output()
        .await
        .expect("run bpmux bookmark verb subprocess")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_bookmark_save_then_get_round_trips_off_mcp() {
    let base = tempfile::tempdir().expect("temp sessions base");
    let dir = base.path();

    let s = run_verb(
        "dev",
        dir,
        &[
            "bookmark",
            "save",
            "--current-task",
            "re-homing the bookmark verb",
            "--next-step",
            "ship it",
        ],
    )
    .await;
    assert!(
        s.status.success(),
        "bpmux bookmark save persists to this role's session bookmark store off MCP; stderr: {}",
        stderr(&s)
    );

    let g = run_verb("dev", dir, &["bookmark", "get"]).await;
    assert!(
        g.status.success(),
        "bpmux bookmark get reads the session bookmark off MCP; stderr: {}",
        stderr(&g)
    );
    assert!(
        stdout(&g).contains("re-homing the bookmark verb") && stdout(&g).contains("ship it"),
        "the round-trip proves save + get hit the SAME crate::bookmark store in the current session off MCP (non-vacuous: the saved fields read back); stdout: {}",
        stdout(&g)
    );
}
