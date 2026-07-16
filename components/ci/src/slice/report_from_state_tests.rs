#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{CiAction, CiSnapshot, CiState, reduce, report_from_state};
use crate::{MAX_BACKOFF, POLL_INTERVAL, WorkflowSummary};

fn green_snapshot() -> CiSnapshot {
    CiSnapshot {
        state: crate::CiState::Green,
        run_id: Some(42),
        head_sha: Some("deadbeef".to_string()),
        display_title: Some("test commit".to_string()),
        conclusion: "success",
        workflows: vec![WorkflowSummary {
            name: Some("build".to_string()),
            run_id: Some(42),
            conclusion: "success",
        }],
    }
}

#[test]
fn populated_state_reports_the_snapshot_verdict_and_the_rehomed_poll_timing() {
    let updated = reduce(&CiState::default(), &CiAction::Updated(green_snapshot()))
        .expect("an Updated fact lands the snapshot");
    let polled = reduce(
        &updated,
        &CiAction::PollSucceeded {
            at: "2026-07-16T00:00:00Z".to_string(),
            updated: true,
        },
    )
    .expect("a successful poll records the liveness timestamp");

    let report = report_from_state(&polled);

    assert_eq!(
        report.conclusion, "success",
        "the snapshot's verdict must reach the report"
    );
    assert_eq!(report.last_run_id, Some(42));
    assert_eq!(report.head_sha, Some("deadbeef".to_string()));
    assert_eq!(report.display_title, Some("test commit".to_string()));
    assert_eq!(report.workflows, green_snapshot().workflows);
    assert_eq!(
        report.last_successful_poll_at,
        Some("2026-07-16T00:00:00Z".to_string()),
        "the rehomed poll-timing (not the shadow) must feed the report"
    );
    assert_eq!(report.branch, "main");
    assert_eq!(report.poll_interval_secs, POLL_INTERVAL.as_secs());
    assert_eq!(report.backoff_cap_secs, MAX_BACKOFF.as_secs());
}

#[test]
fn empty_state_reports_unknown_matching_the_shadows_prior_fallback() {
    let report = report_from_state(&CiState::default());
    assert_eq!(
        report,
        crate::unknown_ci_state_report(),
        "a never-polled state.ci must project the same unknown report the shadow returned"
    );
}
