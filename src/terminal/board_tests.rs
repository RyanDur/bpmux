#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Tests for the DORA-style metric computation (#218).
//!
//! Pure-function tests over fixture issue JSON. Verifies the
//! accuracy fixes:
//! - Cycle time excludes deferred/rejected stories
//! - Rejection rate uses (accepted + rejected) as denominator
//! - Sparse-data thresholds return None instead of misleading numbers

use super::board::compute_dora_metrics;
use chrono::{DateTime, Utc};

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-04-16T03:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn issue(
    state: &str,
    labels: &[&str],
    created_offset_days: i64,
    closed_offset_days: Option<i64>,
) -> serde_json::Value {
    let now = now();
    let created = now - chrono::Duration::days(created_offset_days);
    let mut json = serde_json::json!({
        "state": state,
        "labels": labels.iter().map(|n| serde_json::json!({"name": n})).collect::<Vec<_>>(),
        "createdAt": created.to_rfc3339(),
    });
    if let Some(offset) = closed_offset_days {
        let closed = now - chrono::Duration::days(offset);
        json["closedAt"] = serde_json::Value::String(closed.to_rfc3339());
    } else {
        json["closedAt"] = serde_json::Value::Null;
    }
    json
}

#[test]
fn empty_input_returns_zero_counts_and_insufficient_metrics() {
    let m = compute_dora_metrics(&[], now());
    assert_eq!(m.shipped_this_week, 0);
    assert_eq!(m.in_progress, 0);
    assert_eq!(m.completed_count, 0);
    assert!(m.rejection_rate.is_none());
    assert!(m.avg_cycle_hours.is_none());
}

#[test]
fn deferred_stories_do_not_pollute_cycle_time() {
    // Three accepted stories closed quickly + one deferred story
    // that sat for a year. Old code averaged the deferred one in,
    // inflating cycle time. Fix: deferred excluded from cycle time.
    let issues = vec![
        issue("CLOSED", &[], 1, Some(0)),             // 1-day cycle
        issue("CLOSED", &[], 2, Some(0)),             // 2-day cycle
        issue("CLOSED", &[], 3, Some(0)),             // 3-day cycle
        issue("CLOSED", &["deferred"], 365, Some(0)), // 365-day "story"
    ];
    let m = compute_dora_metrics(&issues, now());
    let avg = m
        .avg_cycle_hours
        .expect("3 accepted = above sparse threshold");
    // Average of 1+2+3 = 2 days = 48 hours. With the bug, would be ~93 days.
    assert!(
        (avg - 48.0).abs() < 1.0,
        "expected ~48h, got {avg}h — deferred story should not be averaged in"
    );
}

#[test]
fn rejection_rate_excludes_deferred_from_denominator() {
    // 2 accepted, 2 rejected, 5 deferred. Old code: 2/9 = 22%.
    // Fix: 2/(2+2) = 50%, the actual change-failure rate.
    let issues = vec![
        issue("CLOSED", &[], 1, Some(0)),
        issue("CLOSED", &[], 1, Some(0)),
        issue("CLOSED", &["rejected"], 1, Some(0)),
        issue("CLOSED", &["rejected"], 1, Some(0)),
        issue("CLOSED", &["deferred"], 30, Some(0)),
        issue("CLOSED", &["deferred"], 30, Some(0)),
        issue("CLOSED", &["deferred"], 30, Some(0)),
        issue("CLOSED", &["deferred"], 30, Some(0)),
        issue("CLOSED", &["deferred"], 30, Some(0)),
    ];
    let m = compute_dora_metrics(&issues, now());
    let rate = m.rejection_rate.expect("4 completed >= sparse threshold");
    assert!(
        (rate - 50.0).abs() < 0.1,
        "expected 50% (2 rejected of 4 completed), got {rate}%"
    );
}

#[test]
fn sparse_completed_returns_none_for_rejection_rate() {
    let issues = vec![
        issue("CLOSED", &[], 1, Some(0)), // only 1 accepted
    ];
    let m = compute_dora_metrics(&issues, now());
    assert!(
        m.rejection_rate.is_none(),
        "1 completed = sparse, should return None"
    );
    assert!(
        m.avg_cycle_hours.is_none(),
        "1 accepted = sparse, should return None"
    );
    assert_eq!(m.completed_count, 1);
}

#[test]
fn shipped_this_week_counts_only_accepted_in_window() {
    // 2 accepted in last 7 days, 1 deferred in last 7 days, 1 accepted 30 days ago
    let issues = vec![
        issue("CLOSED", &[], 5, Some(2)),           // accepted, in window
        issue("CLOSED", &[], 5, Some(1)),           // accepted, in window
        issue("CLOSED", &["deferred"], 5, Some(1)), // deferred, in window
        issue("CLOSED", &[], 35, Some(30)),         // accepted, outside window
    ];
    let m = compute_dora_metrics(&issues, now());
    assert_eq!(m.shipped_this_week, 2, "only accepted in last 7 days count");
}

#[test]
fn in_progress_counts_open_with_started_or_finished_label() {
    let issues = vec![
        issue("OPEN", &["started"], 1, None),
        issue("OPEN", &["finished"], 1, None),
        issue("OPEN", &["ready"], 1, None), // not in progress
        issue("OPEN", &[], 1, None),        // icebox
    ];
    let m = compute_dora_metrics(&issues, now());
    assert_eq!(m.in_progress, 2);
}

#[test]
fn cycle_time_uses_average_of_accepted_only() {
    // 4 accepted, 2 rejected. Cycle time should be only over accepted.
    let issues = vec![
        issue("CLOSED", &[], 1, Some(0)),
        issue("CLOSED", &[], 2, Some(0)),
        issue("CLOSED", &[], 3, Some(0)),
        issue("CLOSED", &[], 4, Some(0)),
        issue("CLOSED", &["rejected"], 100, Some(0)),
        issue("CLOSED", &["rejected"], 200, Some(0)),
    ];
    let m = compute_dora_metrics(&issues, now());
    let avg = m.avg_cycle_hours.unwrap();
    // (1+2+3+4)/4 = 2.5 days = 60 hours
    assert!(
        (avg - 60.0).abs() < 1.0,
        "expected ~60h average, got {avg}h"
    );
}

#[test]
fn format_renders_insufficient_data_for_sparse_metrics() {
    let m = compute_dora_metrics(&[], now());
    let s = m.format();
    assert!(
        s.contains("insufficient data"),
        "expected insufficient data marker, got:\n{s}"
    );
}

#[test]
fn format_renders_numbers_when_data_is_present() {
    let issues = vec![
        issue("CLOSED", &[], 1, Some(0)),
        issue("CLOSED", &[], 1, Some(0)),
        issue("CLOSED", &[], 1, Some(0)),
        issue("CLOSED", &["rejected"], 1, Some(0)),
    ];
    let m = compute_dora_metrics(&issues, now());
    let s = m.format();
    assert!(s.contains("Rejection rate:"), "got:\n{s}");
    assert!(s.contains("Avg cycle time:"), "got:\n{s}");
    assert!(
        !s.contains("insufficient data"),
        "should have real numbers, got:\n{s}"
    );
}
