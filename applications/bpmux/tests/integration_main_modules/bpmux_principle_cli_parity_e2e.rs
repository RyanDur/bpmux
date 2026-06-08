#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

async fn run_verb(
    role: &str,
    identity_dir: &std::path::Path,
    args: &[&str],
) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(args)
        .env("BPMUX_ROLE", role)
        .env("BPMUX_IDENTITY_DIR", identity_dir)
        .output()
        .await
        .expect("run bpmux principle verb subprocess")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_value_principle_identity_round_trip_off_mcp() {
    let id_dir = tempfile::tempdir().expect("temp identity dir");
    let dir = id_dir.path();

    let v = run_verb(
        "dev",
        dir,
        &[
            "value",
            "add",
            "--name",
            "Quality",
            "--description",
            "do the right thing",
        ],
    )
    .await;
    assert!(
        v.status.success(),
        "bpmux value add persists to this role's crate::session::identity store off MCP; stderr: {}",
        stderr(&v)
    );

    let p = run_verb(
        "dev",
        dir,
        &[
            "principle",
            "add",
            "--value",
            "Quality",
            "--principle",
            "ship the right thing simply",
        ],
    )
    .await;
    assert!(
        p.status.success(),
        "bpmux principle add persists under the value off MCP; stderr: {}",
        stderr(&p)
    );

    let i = run_verb("dev", dir, &["identity"]).await;
    assert!(
        i.status.success(),
        "bpmux identity reads this role's identity off MCP; stderr: {}",
        stderr(&i)
    );
    let o = stdout(&i);
    assert!(
        o.contains("Quality") && o.contains("ship the right thing simply"),
        "the round-trip proves value-add + principle-add + identity hit the SAME crate::session::identity store off MCP (non-vacuous: the added value + principle read back); stdout: {o}"
    );
}

#[tokio::test]
async fn bpmux_principle_add_under_a_missing_value_is_refused_off_mcp() {
    let id_dir = tempfile::tempdir().expect("temp identity dir");
    let dir = id_dir.path();
    let out = run_verb(
        "dev",
        dir,
        &[
            "principle",
            "add",
            "--value",
            "Nonexistent",
            "--principle",
            "x",
        ],
    )
    .await;
    assert!(
        !out.status.success(),
        "add_principle under an absent value must fail (the same ValueNotFound the MCP add_principle enforces) — non-vacuous: the store-validation reaches the CLI off MCP; stderr: {}",
        stderr(&out)
    );
}
