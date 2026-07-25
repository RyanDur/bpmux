#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

fn hook_path(name: &str) -> PathBuf {
    repo_root().join("claude-plugin").join("hooks").join(name)
}

fn run_hook(hook: &str, role: &str, stdin: &str, env_extras: &[(&str, &str)]) -> String {
    let path = hook_path(hook);
    let mut cmd = Command::new("bash");
    cmd.arg(&path);
    cmd.env("BPMUX_ROLE", role);
    for (k, v) in env_extras {
        cmd.env(k, v);
    }
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn hook");
    {
        let stdin_handle = child.stdin.as_mut().expect("stdin");
        stdin_handle.write_all(stdin.as_bytes()).expect("write");
    }
    let output = child.wait_with_output().expect("wait");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn unique_state_file(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("bpmux-{prefix}-test-{nanos}"))
}

fn now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

#[test]
fn test_a_pre_accept_hook_fires_with_four_audit_lenses_for_pm_role() {
    let state = unique_state_file("pre-accept-density-pm-test-a");
    let last_ts = (now_secs() - 60).to_string();
    std::fs::write(&state, last_ts).expect("seed prior accept");
    let env: &[(&str, &str)] = &[(
        "BPMUX_HOOK_ACCEPT_DENSITY_STATE_FILE",
        state.to_str().unwrap(),
    )];
    let out = run_hook("pre-accept-reminder.sh", "pm", "{}", env);
    assert!(
        out.starts_with('{') && out.contains("systemMessage"),
        "hook output must be systemMessage JSON; got: {out:?}"
    );
    assert!(
        out.contains("ci green"),
        "missing ci-green lens; got: {out}"
    );
    assert!(
        out.contains("live-walk"),
        "missing live-walk lens; got: {out}"
    );
    assert!(
        out.contains("bug-question") || out.contains("file a Bug"),
        "missing bug-question lens; got: {out}"
    );
    assert!(
        out.contains("user-job") || out.contains("AC-checkbox"),
        "missing user-job-not-AC lens; got: {out}"
    );
}

#[test]
fn test_b_pre_commit_hook_fires_when_two_commits_within_density_window() {
    let state = unique_state_file("pre-commit-density-dev-test-b");
    std::fs::write(&state, (now_secs() - 60).to_string()).expect("seed prior commit");
    let env: &[(&str, &str)] = &[
        ("BPMUX_HOOK_COMMIT_DENSITY_WINDOW_SECS", "300"),
        (
            "BPMUX_HOOK_COMMIT_DENSITY_STATE_FILE",
            state.to_str().unwrap(),
        ),
    ];
    let out = run_hook(
        "pre-commit-reminder.sh",
        "dev",
        r#"{"tool_name":"Bash","tool_input":{"command":"git commit -m something"}}"#,
        env,
    );
    assert!(
        out.starts_with('{') && out.contains("systemMessage"),
        "hook must produce systemMessage JSON on burst commit; got: {out:?}"
    );
    assert!(
        out.contains("debt-at-sight"),
        "missing debt-at-sight lens; got: {out}"
    );
    assert!(
        out.contains("tidy-first"),
        "missing tidy-first lens; got: {out}"
    );
    assert!(
        out.contains("pre-commit-review"),
        "missing pre-commit-review lens; got: {out}"
    );
    assert!(
        out.contains("scope"),
        "missing scope-discipline lens; got: {out}"
    );
}

#[test]
fn test_c_pre_commit_hook_silent_when_prior_commit_outside_window() {
    let state = unique_state_file("pre-commit-density-dev-test-c");
    std::fs::write(&state, (now_secs() - 3600).to_string()).expect("seed old commit");
    let env: &[(&str, &str)] = &[
        ("BPMUX_HOOK_COMMIT_DENSITY_WINDOW_SECS", "60"),
        (
            "BPMUX_HOOK_COMMIT_DENSITY_STATE_FILE",
            state.to_str().unwrap(),
        ),
    ];
    let out = run_hook(
        "pre-commit-reminder.sh",
        "dev",
        r#"{"tool_name":"Bash","tool_input":{"command":"git commit -m slow"}}"#,
        env,
    );
    assert_eq!(
        out.trim(),
        "",
        "slow isolated commit (>window) must NOT fire reminder per high-throughput class-target; got: {out:?}"
    );
}

#[test]
fn test_d_existing_filing_premise_hook_still_fires_always_for_pm_create_story() {
    let env: &[(&str, &str)] = &[];
    let out = run_hook("filing-premise-reminder.sh", "pm", "{}", env);
    assert!(
        out.contains("premise check"),
        "filing-premise hook regression: must still fire on every create_story per always-fire shape; got: {out}"
    );
}

#[test]
fn test_e_existing_preflight_hook_still_fires_always_for_designer_start_story() {
    let env: &[(&str, &str)] = &[];
    let out = run_hook("preflight-reminder.sh", "designer", "{}", env);
    assert!(
        out.contains("pre-flight") || out.contains("convergence"),
        "preflight hook regression: must still fire on every start_story per always-fire shape; got: {out}"
    );
}

#[test]
fn test_pre_commit_hook_only_matches_actual_git_commit_not_other_bash() {
    let state = unique_state_file("pre-commit-density-dev-non-commit");
    std::fs::write(&state, (now_secs() - 60).to_string()).expect("seed");
    let env: &[(&str, &str)] = &[
        ("BPMUX_HOOK_COMMIT_DENSITY_WINDOW_SECS", "300"),
        (
            "BPMUX_HOOK_COMMIT_DENSITY_STATE_FILE",
            state.to_str().unwrap(),
        ),
    ];
    let out = run_hook(
        "pre-commit-reminder.sh",
        "dev",
        r#"{"tool_name":"Bash","tool_input":{"command":"git status"}}"#,
        env,
    );
    assert_eq!(
        out.trim(),
        "",
        "non-git-commit Bash invocation must not fire commit-reminder; got: {out:?}"
    );
}

#[test]
fn test_pre_accept_hook_silent_for_non_pm_non_owner_role() {
    let env: &[(&str, &str)] = &[];
    let out = run_hook("pre-accept-reminder.sh", "dev", "{}", env);
    assert_eq!(
        out.trim(),
        "",
        "pre-accept hook must be silent for roles outside pm/owner; got: {out:?}"
    );
}

#[test]
fn test_pre_accept_hook_silent_when_no_prior_accept_in_window() {
    let state = unique_state_file("pre-accept-density-pm-no-prior");
    let _ = std::fs::remove_file(&state);
    let env: &[(&str, &str)] = &[
        ("BPMUX_HOOK_ACCEPT_DENSITY_WINDOW_SECS", "60"),
        (
            "BPMUX_HOOK_ACCEPT_DENSITY_STATE_FILE",
            state.to_str().unwrap(),
        ),
    ];
    let out = run_hook("pre-accept-reminder.sh", "pm", "{}", env);
    assert_eq!(
        out.trim(),
        "",
        "first-of-its-kind accept (no prior in window) must NOT fire reminder; got: {out:?}"
    );
}
