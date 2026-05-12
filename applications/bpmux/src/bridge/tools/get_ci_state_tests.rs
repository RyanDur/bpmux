#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

// Tests for #1287: `get_ci_state` aggregate-state across all
// workflows triggered by a commit. Targets `classify_runs` (the
// pure aggregation logic) + `build_ci_state_report` (the report
// shape including the `workflows: Vec<WorkflowSummary>` breakdown).
//
// Sister to today's #1283 + #1271 substrate-resilience chain and
// to `feedback_verify_tool_output_schema` (observation-tool
// field-overloading class): aggregate vs last-workflow-on-commit
// vs per-workflow are distinct contracts; the field semantic is
// now explicit.

use crate::bridge::tools::build_ci_state_report;
use crate::ci::{CiState, RunEntry, classify_runs};

fn run(
    status: &str,
    conclusion: Option<&str>,
    head_sha: &str,
    name: &str,
    run_id: u64,
) -> RunEntry {
    RunEntry {
        status: status.to_string(),
        conclusion: conclusion.map(String::from),
        head_sha: Some(head_sha.to_string()),
        display_title: Some(format!("commit on {head_sha}")),
        database_id: Some(run_id),
        name: Some(name.to_string()),
        annotation_message: None,
    }
}

// ── AC 1: aggregate behavior — 4 rules ──

#[test]
fn aggregate_returns_in_progress_when_any_workflow_running() {
    // Reproduces the live bug-class: smoke workflow concluded green,
    // main CI still running. Old behavior returned `success` based on
    // first-to-conclude. New behavior: `in_progress` because any
    // run still active wins.
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 100),
        run("in_progress", None, "abc1234", "CI", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Running,
        "if any workflow is in_progress on the latest commit, aggregate is Running"
    );
}

#[test]
fn aggregate_returns_failure_when_any_workflow_failed() {
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 100),
        run("completed", Some("failure"), "abc1234", "CI", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Red,
        "if any workflow failed on the latest commit, aggregate is Red"
    );
}

#[test]
fn aggregate_returns_success_only_when_all_workflows_succeeded() {
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 100),
        run("completed", Some("success"), "abc1234", "CI", 101),
        run("completed", Some("success"), "abc1234", "deploy", 102),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Green,
        "all workflows green = aggregate Green"
    );
}

#[test]
fn aggregate_returns_unknown_for_no_runs() {
    let event = classify_runs(&[]);
    assert_eq!(event.state, CiState::Unknown);
    assert!(event.workflows.is_empty());
}

// ── AC 2: failure wins over running when both present ──
// (failure is the more-actionable signal: red CI blocks gates
// regardless of whether other workflows are still progressing)

#[test]
fn aggregate_running_wins_over_failure() {
    // Spec aggregation: Running > Red. If any is running, we don't
    // know yet whether the failure is final (a re-run might be in-
    // flight). Gate callers should treat Running as "wait, don't
    // decide yet".
    let runs = vec![
        run("completed", Some("failure"), "abc1234", "CI", 100),
        run("in_progress", None, "abc1234", "smoke", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(event.state, CiState::Running);
}

// ── AC 3: head_sha filtering — only the latest commit's runs count ──

#[test]
fn aggregate_filters_to_latest_head_sha_ignores_older_commit_runs() {
    // Latest commit (abc1234) has 2 green runs; an older commit
    // (def5678) had a failure. Aggregate should be Green because
    // only the latest commit's runs are considered.
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 200),
        run("completed", Some("success"), "abc1234", "CI", 201),
        run("completed", Some("failure"), "def5678", "CI", 100),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Green,
        "older-commit failures don't affect aggregate of latest commit"
    );
    assert_eq!(
        event.workflows.len(),
        2,
        "only latest-commit runs in breakdown"
    );
}

// ── AC 4: workflows[] field carries per-run breakdown ──

#[test]
fn workflows_field_carries_per_run_breakdown() {
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 100),
        run("in_progress", None, "abc1234", "CI", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(event.workflows.len(), 2);
    let smoke = event
        .workflows
        .iter()
        .find(|w| w.name.as_deref() == Some("smoke"))
        .expect("smoke workflow in breakdown");
    assert_eq!(smoke.conclusion, "success");
    assert_eq!(smoke.run_id, Some(100));
    let ci = event
        .workflows
        .iter()
        .find(|w| w.name.as_deref() == Some("CI"))
        .expect("CI workflow in breakdown");
    assert_eq!(ci.conclusion, "in_progress");
    assert_eq!(ci.run_id, Some(101));
}

#[test]
fn build_ci_state_report_serializes_workflows_array() {
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 100),
        run("in_progress", None, "abc1234", "CI", 101),
    ];
    let report = build_ci_state_report(&runs);
    assert_eq!(report.conclusion, "in_progress");
    assert_eq!(report.workflows.len(), 2);
    let json = serde_json::to_value(&report).expect("serialize");
    assert!(
        json["workflows"].is_array(),
        "workflows must serialize as JSON array"
    );
    assert_eq!(json["workflows"].as_array().unwrap().len(), 2);
}

// ── AC 7: regression — false-positive green when some still running ──

#[test]
fn regression_smoke_fast_main_slow_does_not_fire_premature_green() {
    // Today's reproducer-class: smoke workflow concludes in 1-8s,
    // main CI in 5-60 min. Bridge previously fired "🟢 CI green" on
    // smoke conclusion alone; new aggregate makes this in_progress.
    let runs = vec![
        run(
            "completed",
            Some("success"),
            "5e2f832",
            "self-hosted runner smoke-test",
            100,
        ),
        run("in_progress", None, "5e2f832", "CI", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Running,
        "smoke-green-while-CI-running must aggregate to Running (#1287 reproducer-class)"
    );
}

#[test]
fn regression_smoke_green_main_failure_aggregates_to_failure() {
    let runs = vec![
        run(
            "completed",
            Some("success"),
            "f8c42d7",
            "self-hosted runner smoke-test",
            100,
        ),
        run("completed", Some("failure"), "f8c42d7", "CI", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Red,
        "smoke-green-main-failed must aggregate to Red — gate callers see failure"
    );
}

// ── Edge cases ──

#[test]
fn single_workflow_run_aggregates_to_its_own_state() {
    // Backward-compat: single-workflow projects must behave the
    // same as before (per-run state = aggregate state).
    let runs = vec![run("completed", Some("success"), "abc1234", "CI", 100)];
    let event = classify_runs(&runs);
    assert_eq!(event.state, CiState::Green);
    assert_eq!(event.workflows.len(), 1);
}

#[test]
fn runs_without_head_sha_fall_back_to_single_run_classification() {
    // Legacy test-shape: no head_sha set. classify_runs falls back
    // to per-run on `latest` only (pre-#1287 behavior preserved).
    let mut entry = run("completed", Some("success"), "", "CI", 100);
    entry.head_sha = None;
    let runs = vec![entry];
    let event = classify_runs(&runs);
    assert_eq!(event.state, CiState::Green);
}

#[test]
fn aggregate_handles_hybrid_runner_shape_post_1253_cutover() {
    // #1254 (Epic #1247 slice 4): post-cutover verification that
    // `get_ci_state`'s classification works on the hybrid CI shape
    // shipped in #1253: `test` on [self-hosted, macos, arm64];
    // `build-bpmux` + `playwright-e2e` on `ubuntu-latest`. The
    // bridge-adapter consumes RunEntries from the GHA API — the
    // runner-label (self-hosted vs ubuntu-latest) doesn't appear in
    // the entry; only status + conclusion drive aggregation. This
    // test pins that platform-axis-independence: any mix of runners
    // for workflows-on-the-same-commit aggregates by the same rule.
    //
    // Sister to `feedback_local_pass_ci_fail_hidden_axis_coupling` —
    // the bridge-tool contract is independent of runner platform; we
    // assert that invariant here so a future refactor that introduces
    // platform-conditional classification fires immediately.
    let runs = vec![
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "test",
            75572890367,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "build-bpmux",
            75572890351,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (1)",
            75573139951,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (2)",
            75573139874,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (3)",
            75573139941,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (4)",
            75573139917,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (5)",
            75573140045,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (6)",
            75573140101,
        ),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Green,
        "hybrid runner shape (1 self-hosted + 7 ubuntu-latest jobs) aggregates Green when all succeed"
    );
    assert_eq!(
        event.workflows.len(),
        8,
        "all 8 jobs surface in workflows[] breakdown"
    );
}

#[test]
fn aggregate_red_on_hybrid_shape_when_playwright_shard_fails() {
    // Sister: #1254 falsifier — if any job in the hybrid shape fails
    // (e.g., a playwright-e2e shard on ubuntu-latest), aggregate is
    // Red regardless of which runner the failure came from.
    let runs = vec![
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "test",
            75572890367,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "build-bpmux",
            75572890351,
        ),
        run(
            "completed",
            Some("failure"),
            "57f1a08c",
            "playwright-e2e (2)",
            75573139874,
        ),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Red,
        "a single playwright shard failure on ubuntu-latest still aggregates to Red"
    );
}

#[test]
fn workflow_summary_conclusion_uses_canonical_strings() {
    // Workflow per-run conclusions must use the same vocabulary as
    // the aggregate (`success` / `failure` / `in_progress` /
    // `quota_exhausted` / `unknown`) so callers can rely on one
    // contract.
    let runs = vec![
        run("completed", Some("success"), "abc", "a", 1),
        run("completed", Some("failure"), "abc", "b", 2),
        run("in_progress", None, "abc", "c", 3),
        run("queued", None, "abc", "d", 4),
    ];
    let event = classify_runs(&runs);
    let conclusions: Vec<&str> = event.workflows.iter().map(|w| w.conclusion).collect();
    assert!(conclusions.contains(&"success"));
    assert!(conclusions.contains(&"failure"));
    // both in_progress and queued map to "in_progress" in the
    // aggregate vocabulary
    assert!(conclusions.iter().filter(|c| **c == "in_progress").count() >= 2);
}
