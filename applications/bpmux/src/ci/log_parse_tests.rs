#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::log_parse::{build_non_test_failure_record, parse_playwright_failures_from_log};
use crate::ci::CiLogError;

#[test]
fn empty_log_yields_zero_failures_and_success_conclusion() {
    let report = parse_playwright_failures_from_log(42, "");
    assert_eq!(report.run_id, 42);
    assert_eq!(report.conclusion, "success");
    assert!(report.failures.is_empty());
}

#[test]
fn parses_single_failure_with_test_timeout_message() {
    let log = "\
Running 47 tests using 4 workers, shard 6 of 6
··············································×T

  1) tests/playwright/offline-board.spec.ts:121:7 › #1158 — offline-first board › F8 stale — glyph surfaces stale state when ingest exceeds threshold

    Test timeout of 30000ms exceeded.
";
    let report = parse_playwright_failures_from_log(42, log);
    assert_eq!(report.conclusion, "failure");
    assert_eq!(report.failures.len(), 1);
    let f = &report.failures[0];
    assert_eq!(
        f.file.as_deref(),
        Some("tests/playwright/offline-board.spec.ts")
    );
    assert_eq!(f.line, Some(121));
    assert!(f.test_name.contains("F8 stale"));
    assert!(f.error_message.contains("Test timeout"));
}

#[test]
fn retry_marker_increments_retry_count() {
    let log = "\
  ✘  1 tests/foo.spec.ts:10:5 › suite › my flaky test
  ✘  2 tests/foo.spec.ts:10:5 › suite › my flaky test (retry #1)

    Error: assertion failed
";
    let report = parse_playwright_failures_from_log(7, log);
    assert!(report.failures.iter().any(|f| f.retry_count > 0));
}

#[test]
fn run_not_found_serializes_with_run_id_payload() {
    let err = CiLogError::RunNotFound { run_id: 9999 };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "run_not_found");
    assert_eq!(json["run_id"], 9999);
}

#[test]
fn run_in_progress_carries_current_conclusion() {
    let err = CiLogError::RunInProgress {
        run_id: 100,
        conclusion: "in_progress".to_string(),
    };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "run_in_progress");
    assert_eq!(json["conclusion"], "in_progress");
}

#[test]
fn parses_failures_from_gh_actions_timestamp_prefixed_log() {
    let log = "\
2026-05-04T16:17:35.1721141Z Running 47 tests using 4 workers, shard 6 of 6
2026-05-04T16:17:36.0000000Z
2026-05-04T16:26:55.1234567Z   1) tests/playwright/offline-board.spec.ts:121:7 \u{203A} #1158 \u{2014} offline-first board \u{203A} F8 stale
2026-05-04T16:26:55.1234568Z
2026-05-04T16:26:55.1234569Z     Test timeout of 30000ms exceeded.
";
    let report = parse_playwright_failures_from_log(42, log);
    assert_eq!(report.conclusion, "failure");
    assert_eq!(
        report.failures.len(),
        1,
        "GH-Actions timestamp-prefixed log must parse"
    );
    let f = &report.failures[0];
    assert_eq!(
        f.file.as_deref(),
        Some("tests/playwright/offline-board.spec.ts"),
        "parser strips GH timestamps before extracting file/line"
    );
    assert_eq!(f.line, Some(121));
}

#[test]
fn rate_limited_carries_wait_secs() {
    let err = CiLogError::RateLimited { wait_secs: 60 };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "rate_limited");
    assert_eq!(json["wait_secs"], 60);
}

#[test]
fn fallback_record_for_clippy_failure_carries_tail_of_log() {
    let log = "\
2026-05-11T14:00:00Z Compiling bpmux v0.1.0
2026-05-11T14:00:01Z error[E0308]: mismatched types
2026-05-11T14:00:01Z   --> applications/bpmux/src/lib.rs:42:15
2026-05-11T14:00:01Z    |
2026-05-11T14:00:02Z error: aborting due to previous error
2026-05-11T14:00:02Z Process completed with exit code 101.
";
    let record = build_non_test_failure_record("build-bpmux", log);
    assert_eq!(record.class.as_deref(), Some("non_test_failure"));
    assert_eq!(record.stage.as_deref(), Some("build-bpmux"));
    assert_eq!(record.shard_id.as_deref(), Some("build-bpmux"));
    assert!(
        record.error_message.contains("error[E0308]"),
        "tail-of-log carries the compiler error: {}",
        record.error_message
    );
    assert!(
        record.error_message.contains("exit code 101"),
        "tail captures step exit code: {}",
        record.error_message
    );
}

#[test]
fn fallback_record_for_empty_log_returns_explicit_placeholder() {
    let record = build_non_test_failure_record("test", "");
    assert_eq!(record.class.as_deref(), Some("non_test_failure"));
    assert!(
        record.error_message.contains("empty or unparseable"),
        "explicit placeholder beats empty failures[]: {}",
        record.error_message
    );
}

#[test]
fn fallback_record_bounded_by_max_tail_lines() {
    let mut log = String::new();
    for i in 0..200 {
        log.push_str(&format!("line {i}\n"));
    }
    let record = build_non_test_failure_record("audit", &log);
    let line_count = record.error_message.lines().count();
    assert!(
        line_count <= 50,
        "MAX_TAIL_LINES bounds payload: {line_count}"
    );
    assert!(
        record.error_message.contains("line 199"),
        "tail captures the LAST lines (not first): {}",
        record.error_message
    );
}

#[test]
fn playwright_failure_carries_playwright_class() {
    let log = "\
  ✘  1 tests/foo.spec.ts:10:5 › suite › my test

    Error: assertion failed
";
    let report = parse_playwright_failures_from_log(42, log);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(
        report.failures[0].class.as_deref(),
        Some("playwright"),
        "test-level failures carry class=\"playwright\" for caller-side routing"
    );
}
