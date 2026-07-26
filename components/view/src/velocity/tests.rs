#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{SPARSE_THRESHOLD, Transition, VelocityMetrics, WINDOW_DAYS, compute};
use chrono::{DateTime, Duration as ChronoDuration, Utc};

fn tr(story_id: u64, new_state: &str, at: DateTime<Utc>) -> Transition {
    Transition {
        story_id,
        new_state: new_state.to_string(),
        at,
    }
}

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-04-15T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

#[test]
fn empty_transitions_returns_zero_metrics_and_sparse_flag() {
    let m = compute(&[], now(), |_| true);
    assert_eq!(m.delivered_in_window, 0);
    assert_eq!(m.delivered_per_day, 0.0);
    assert!(m.median_pickup_to_deliver_hours.is_none());
    assert!(m.median_deliver_to_accept_hours.is_none());
    assert!(m.rework_rate.is_none());
    assert!(m.sparse);
    assert!(m.note.as_deref().unwrap_or("").contains("sparse"));
}

#[test]
fn one_delivered_story_marked_sparse_below_threshold() {
    let delivered = vec![tr(199, "acceptance", now() - ChronoDuration::hours(2))];
    let m = compute(&delivered, now(), |_| true);
    assert_eq!(m.delivered_in_window, 1);
    assert!(m.sparse);
    const _: () = assert!(SPARSE_THRESHOLD > 1);
}

#[test]
fn three_delivered_stories_crosses_sparse_threshold() {
    let transitions = vec![
        tr(1, "acceptance", now() - ChronoDuration::hours(1)),
        tr(2, "acceptance", now() - ChronoDuration::hours(2)),
        tr(3, "acceptance", now() - ChronoDuration::hours(3)),
    ];
    let m = compute(&transitions, now(), |_| true);
    assert_eq!(m.delivered_in_window, 3);
    assert!(!m.sparse, "3 deliveries should clear the sparse threshold");
    assert!(m.note.is_none());
}

#[test]
fn deliveries_outside_14_day_window_are_excluded() {
    let transitions = vec![
        tr(1, "acceptance", now() - ChronoDuration::days(1)),
        tr(2, "acceptance", now() - ChronoDuration::days(7)),
        tr(3, "acceptance", now() - ChronoDuration::days(20)),
        tr(4, "acceptance", now() + ChronoDuration::hours(1)),
    ];
    let m = compute(&transitions, now(), |_| true);
    assert_eq!(m.delivered_in_window, 2);
}

#[test]
fn pickup_to_deliver_median_pairs_start_and_deliver_by_story() {
    let base = now() - ChronoDuration::days(2);
    let transitions = vec![
        tr(10, "in-progress", base),
        tr(10, "acceptance", base + ChronoDuration::hours(3)),
        tr(11, "in-progress", base + ChronoDuration::hours(1)),
        tr(11, "acceptance", base + ChronoDuration::hours(2)),
        tr(12, "in-progress", base + ChronoDuration::hours(4)),
        tr(12, "acceptance", base + ChronoDuration::hours(10)),
    ];
    let m = compute(&transitions, now(), |_| true);
    assert_eq!(m.median_pickup_to_deliver_hours, Some(3.0));
}

#[test]
fn deliver_to_accept_median_pairs_by_story() {
    let base = now() - ChronoDuration::days(1);
    let transitions = vec![
        tr(20, "acceptance", base),
        tr(20, "accepted", base + ChronoDuration::hours(2)),
        tr(21, "acceptance", base + ChronoDuration::minutes(30)),
        tr(21, "accepted", base + ChronoDuration::hours(5)),
    ];
    let m = compute(&transitions, now(), |_| true);
    assert_eq!(m.median_deliver_to_accept_hours, Some(3.25));
}

#[test]
fn rework_rate_counts_desk_check_send_backs_per_unique_delivery() {
    let base = now() - ChronoDuration::days(1);
    let transitions = vec![
        tr(30, "acceptance", base),
        tr(30, "in-progress", base + ChronoDuration::hours(1)),
        tr(30, "acceptance", base + ChronoDuration::hours(4)),
        tr(31, "acceptance", base + ChronoDuration::hours(2)),
        tr(32, "acceptance", base + ChronoDuration::hours(3)),
        tr(33, "acceptance", base + ChronoDuration::hours(6)),
    ];
    let m = compute(&transitions, now(), |_| true);
    assert_eq!(m.delivered_in_window, 4);
    assert_eq!(
        m.rework_rate,
        Some(0.25),
        "a story sent back from acceptance to in-progress is one rework"
    );
}

#[test]
fn re_delivered_story_counts_once_in_delivered_in_window() {
    let base = now() - ChronoDuration::days(1);
    let transitions = vec![
        tr(50, "acceptance", base),
        tr(50, "in-progress", base + ChronoDuration::hours(1)),
        tr(50, "acceptance", base + ChronoDuration::hours(2)),
        tr(50, "in-progress", base + ChronoDuration::hours(3)),
        tr(50, "acceptance", base + ChronoDuration::hours(4)),
    ];
    let m = compute(&transitions, now(), |_| true);
    assert_eq!(
        m.delivered_in_window, 1,
        "re-delivered story counts as ONE unique delivery"
    );
}

#[test]
fn out_of_order_input_still_pairs_by_story_id() {
    let base = now() - ChronoDuration::hours(6);
    let transitions = vec![
        tr(40, "acceptance", base + ChronoDuration::hours(3)),
        tr(40, "in-progress", base),
        tr(41, "acceptance", base + ChronoDuration::hours(1)),
        tr(41, "in-progress", base - ChronoDuration::hours(2)),
        tr(42, "acceptance", base + ChronoDuration::hours(4)),
    ];
    let m = compute(&transitions, now(), |_| true);
    assert_eq!(m.delivered_in_window, 3);
    assert!(m.median_pickup_to_deliver_hours.is_some());
}

#[test]
fn states_outside_the_lifecycle_are_ignored() {
    let transitions = vec![
        tr(99, "retired", now() - ChronoDuration::hours(1)),
        tr(98, "not-started", now() - ChronoDuration::hours(2)),
    ];
    let m = compute(&transitions, now(), |_| true);
    assert_eq!(m.delivered_in_window, 0);
    assert!(m.wip_history.is_empty());
}

#[test]
fn delivered_per_day_divides_by_window() {
    let mut transitions = Vec::new();
    for i in 0..14 {
        transitions.push(tr(
            100 + i,
            "acceptance",
            now() - ChronoDuration::hours(i as i64 * 4),
        ));
    }
    let m: VelocityMetrics = compute(&transitions, now(), |_| true);
    assert_eq!(m.delivered_in_window, 14);
    assert_eq!(m.delivered_per_day, 14.0 / WINDOW_DAYS as f64);
}

#[test]
fn plumbing_stories_filtered_out_of_velocity() {
    let transitions = vec![
        tr(1, "acceptance", now() - ChronoDuration::hours(1)),
        tr(2, "acceptance", now() - ChronoDuration::hours(2)),
        tr(3, "acceptance", now() - ChronoDuration::hours(3)),
    ];
    let is_feature = |n: u64| n == 2;
    let m = compute(&transitions, now(), is_feature);
    assert_eq!(m.delivered_in_window, 1);
    assert!(m.sparse, "1 feature delivery is below the sparse threshold");
}

#[test]
fn predicate_drives_what_gets_counted() {
    let transitions = vec![
        tr(1, "acceptance", now() - ChronoDuration::hours(1)),
        tr(2, "acceptance", now() - ChronoDuration::hours(2)),
        tr(3, "acceptance", now() - ChronoDuration::hours(3)),
    ];
    let all = compute(&transitions, now(), |_| true);
    let without_one = compute(&transitions, now(), |n| n != 1);
    assert_eq!(all.delivered_in_window, 3);
    assert_eq!(without_one.delivered_in_window, 2);
}

#[test]
fn no_features_produces_zero_velocity() {
    let transitions = vec![
        tr(1, "acceptance", now() - ChronoDuration::hours(1)),
        tr(2, "acceptance", now() - ChronoDuration::hours(2)),
    ];
    let m = compute(&transitions, now(), |_| false);
    assert_eq!(m.delivered_in_window, 0);
    assert_eq!(m.delivered_per_day, 0.0);
}

#[test]
fn wip_history_is_a_real_per_activity_day_in_progress_series() {
    let transitions = vec![
        tr(1, "in-progress", now() - ChronoDuration::days(4)),
        tr(2, "in-progress", now() - ChronoDuration::days(3)),
        tr(3, "in-progress", now() - ChronoDuration::days(2)),
        tr(1, "acceptance", now() - ChronoDuration::days(1)),
    ];
    let m = compute(&transitions, now(), |_| true);
    let counts: Vec<usize> = m.wip_history.iter().map(|p| p.wip_count).collect();
    assert_eq!(
        counts,
        vec![1, 2, 3, 2],
        "in-progress count rises with each start, falls on the deliver — one point per activity-day"
    );
    assert_eq!(m.wip_history.len(), 4);
}

#[test]
fn wip_history_excludes_out_of_window_events_and_never_underflows() {
    let transitions = vec![
        tr(1, "in-progress", now() - ChronoDuration::days(20)),
        tr(2, "in-progress", now() - ChronoDuration::days(2)),
        tr(2, "acceptance", now() - ChronoDuration::days(1)),
    ];
    let m = compute(&transitions, now(), |_| true);
    let counts: Vec<usize> = m.wip_history.iter().map(|p| p.wip_count).collect();
    assert_eq!(
        counts,
        vec![1, 0],
        "the out-of-window #1 start is excluded; delivering #2 returns to zero (no underflow)"
    );
}
