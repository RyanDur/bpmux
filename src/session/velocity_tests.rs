#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::velocity::{SPARSE_THRESHOLD, VelocityMetrics, WINDOW_DAYS, compute};
use crate::session::message::{Message, MessageType};
use chrono::{DateTime, Duration as ChronoDuration, Utc};

fn msg(text: &str, ts: DateTime<Utc>) -> Message {
    Message {
        from: "dev:claude-code".to_string(),
        text: text.to_string(),
        ts,
        ..Message::default()
    }
}

fn now() -> DateTime<Utc> {
    // Fixed reference instant for deterministic window math.
    DateTime::parse_from_rfc3339("2026-04-15T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

#[test]
fn empty_messages_returns_zero_metrics_and_sparse_flag() {
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
    let delivered = vec![msg(
        "#199 (test) delivered — desk-check needed",
        now() - ChronoDuration::hours(2),
    )];
    let m = compute(&delivered, now(), |_| true);
    assert_eq!(m.delivered_in_window, 1);
    assert!(m.sparse);
    const _: () = assert!(SPARSE_THRESHOLD > 1);
}

#[test]
fn three_delivered_stories_crosses_sparse_threshold() {
    let messages = vec![
        msg("#1 delivered", now() - ChronoDuration::hours(1)),
        msg("#2 delivered", now() - ChronoDuration::hours(2)),
        msg("#3 delivered", now() - ChronoDuration::hours(3)),
    ];
    let m = compute(&messages, now(), |_| true);
    assert_eq!(m.delivered_in_window, 3);
    assert!(!m.sparse, "3 deliveries should clear the sparse threshold");
    assert!(m.note.is_none());
}

#[test]
fn deliveries_outside_14_day_window_are_excluded() {
    let messages = vec![
        msg("#1 delivered", now() - ChronoDuration::days(1)),
        msg("#2 delivered", now() - ChronoDuration::days(7)),
        msg("#3 delivered", now() - ChronoDuration::days(20)), // outside window
        msg("#4 delivered", now() + ChronoDuration::hours(1)), // future
    ];
    let m = compute(&messages, now(), |_| true);
    assert_eq!(m.delivered_in_window, 2);
}

#[test]
fn pickup_to_deliver_median_pairs_start_and_deliver_by_issue() {
    let base = now() - ChronoDuration::days(2);
    let messages = vec![
        msg("#10 started. ready queue shrunk", base),
        msg("#10 (foo) delivered", base + ChronoDuration::hours(3)),
        msg(
            "#11 started. ready queue shrunk",
            base + ChronoDuration::hours(1),
        ),
        msg("#11 (bar) delivered", base + ChronoDuration::hours(2)),
        msg(
            "#12 started. ready queue shrunk",
            base + ChronoDuration::hours(4),
        ),
        msg("#12 (baz) delivered", base + ChronoDuration::hours(10)),
    ];
    let m = compute(&messages, now(), |_| true);
    // durations: 3, 1, 6 → sorted [1, 3, 6] → median 3
    assert_eq!(m.median_pickup_to_deliver_hours, Some(3.0));
}

#[test]
fn deliver_to_accept_median_pairs_by_issue() {
    let base = now() - ChronoDuration::days(1);
    let messages = vec![
        msg("#20 delivered", base),
        msg("#20 accepted. pick next", base + ChronoDuration::hours(2)),
        msg("#21 delivered", base + ChronoDuration::minutes(30)),
        msg("#21 accepted. pick next", base + ChronoDuration::hours(5)),
    ];
    let m = compute(&messages, now(), |_| true);
    // durations: 2h, 4.5h → median 3.25
    assert_eq!(m.median_deliver_to_accept_hours, Some(3.25));
}

#[test]
fn rework_rate_counts_failed_desk_checks_per_delivery() {
    let base = now() - ChronoDuration::days(1);
    let messages = vec![
        msg("#30 delivered", base),
        msg(
            "#30 failed desk-check: needs negative test",
            base + ChronoDuration::hours(1),
        ),
        msg("#30 delivered", base + ChronoDuration::hours(4)),
        msg("#31 delivered", base + ChronoDuration::hours(2)),
        msg("#32 delivered", base + ChronoDuration::hours(3)),
        msg("#33 delivered", base + ChronoDuration::hours(6)),
    ];
    let m = compute(&messages, now(), |_| true);
    // 5 delivered events in window (#30 twice, #31/#32/#33 once each),
    // 1 fail-desk-check → 1/5 = 0.2
    assert_eq!(m.delivered_in_window, 5);
    assert_eq!(m.rework_rate, Some(0.2));
}

#[test]
fn out_of_order_events_still_pair_by_issue_number() {
    let base = now() - ChronoDuration::hours(6);
    let messages = vec![
        msg("#40 (foo) delivered", base + ChronoDuration::hours(3)),
        msg("#40 started. ready queue shrunk", base),
        msg("#41 (bar) delivered", base + ChronoDuration::hours(1)),
        msg(
            "#41 started. ready queue shrunk",
            base - ChronoDuration::hours(2),
        ),
        msg("#42 delivered", base + ChronoDuration::hours(4)),
    ];
    let m = compute(&messages, now(), |_| true);
    assert_eq!(m.delivered_in_window, 3);
    assert!(m.median_pickup_to_deliver_hours.is_some());
}

#[test]
fn non_chat_messages_are_ignored() {
    let activity = Message {
        from: "dev:claude-code".to_string(),
        text: "#99 delivered".to_string(),
        ts: now() - ChronoDuration::hours(1),
        message_type: MessageType::Activity, // not Chat
        ..Message::default()
    };
    let m = compute(&[activity], now(), |_| true);
    assert_eq!(m.delivered_in_window, 0);
}

#[test]
fn delivered_per_day_divides_by_window() {
    let mut messages = Vec::new();
    for i in 0..14 {
        messages.push(msg(
            &format!("#{} delivered", 100 + i),
            now() - ChronoDuration::hours((i as i64) * 4),
        ));
    }
    let m: VelocityMetrics = compute(&messages, now(), |_| true);
    assert_eq!(m.delivered_in_window, 14);
    assert_eq!(m.delivered_per_day, 14.0 / WINDOW_DAYS as f64);
}

#[test]
fn text_without_number_is_not_parsed_as_lifecycle() {
    let messages = vec![
        msg(
            "hello team, just some broadcast",
            now() - ChronoDuration::hours(1),
        ),
        msg("delivered yesterday", now() - ChronoDuration::hours(2)), // no #N
    ];
    let m = compute(&messages, now(), |_| true);
    assert_eq!(m.delivered_in_window, 0);
}

#[test]
fn plumbing_stories_filtered_out_of_velocity() {
    // Three deliveries in-window, but only #2 is a feature.
    // Velocity should count 1, not 3 — matching the
    // "business-value only" rule.
    let messages = vec![
        msg("#1 delivered", now() - ChronoDuration::hours(1)),
        msg("#2 delivered", now() - ChronoDuration::hours(2)),
        msg("#3 delivered", now() - ChronoDuration::hours(3)),
    ];
    let is_feature = |n: u64| n == 2;
    let m = compute(&messages, now(), is_feature);
    assert_eq!(m.delivered_in_window, 1);
    assert!(m.sparse, "1 feature delivery is below the sparse threshold");
}

#[test]
fn predicate_drives_what_gets_counted() {
    // `|_| true` counts every event; a predicate that rejects one
    // specific issue counts fewer. Demonstrates the filter is the
    // causal input, not just a pass-through — distinguishes "filter
    // works" from "filter is silently ignored."
    let messages = vec![
        msg("#1 delivered", now() - ChronoDuration::hours(1)),
        msg("#2 delivered", now() - ChronoDuration::hours(2)),
        msg("#3 delivered", now() - ChronoDuration::hours(3)),
    ];
    let all = compute(&messages, now(), |_| true);
    let without_one = compute(&messages, now(), |n| n != 1);
    assert_eq!(all.delivered_in_window, 3);
    assert_eq!(without_one.delivered_in_window, 2);
}

#[test]
fn no_features_produces_zero_velocity() {
    // All plumbing → 0 deliveries counted. Not just sparse —
    // actually zero. The metric reads honestly low.
    let messages = vec![
        msg("#1 delivered", now() - ChronoDuration::hours(1)),
        msg("#2 delivered", now() - ChronoDuration::hours(2)),
    ];
    let m = compute(&messages, now(), |_| false);
    assert_eq!(m.delivered_in_window, 0);
    assert_eq!(m.delivered_per_day, 0.0);
}

#[test]
fn is_feature_verdict_matches_feature_label() {
    use crate::session::velocity::is_feature_verdict;
    assert!(is_feature_verdict(&["Feature".to_string()], ""));
    assert!(is_feature_verdict(
        &["ready".to_string(), "Feature".to_string()],
        ""
    ));
}

#[test]
fn is_feature_verdict_rejects_chore_and_bug_labels() {
    use crate::session::velocity::is_feature_verdict;
    assert!(!is_feature_verdict(&["Chore".to_string()], ""));
    assert!(!is_feature_verdict(&["Bug".to_string()], ""));
}

#[test]
fn is_feature_verdict_falls_back_to_title_prefix() {
    use crate::session::velocity::is_feature_verdict;
    // New canonical prefixes
    assert!(is_feature_verdict(&[], "[Feature] ship the thing"));
    assert!(!is_feature_verdict(&[], "[Chore] refactor the thing"));
    assert!(!is_feature_verdict(&[], "[Bug] fix the thing"));
    assert!(!is_feature_verdict(&[], "Bug: fix the thing"));
}

#[test]
fn is_feature_verdict_supports_legacy_prefixes_during_migration() {
    use crate::session::velocity::is_feature_verdict;
    // Pre-#264 prefixes still parse correctly so backfill can run
    // without racing the filter update.
    assert!(is_feature_verdict(&[], "[vertical] a feature"));
    assert!(is_feature_verdict(
        &[],
        "[stranded-vertical] close path broken"
    ));
    assert!(!is_feature_verdict(&[], "[honest-plumbing] debt fix"));
    assert!(!is_feature_verdict(&[], "[half-slice] partial shape"));
}

#[test]
fn is_feature_verdict_defaults_to_feature_for_untagged() {
    use crate::session::velocity::is_feature_verdict;
    // Default-to-Feature: un-tagged stories don't silently drop from
    // velocity. Better to overcount slightly than undercount silently.
    assert!(is_feature_verdict(&[], ""));
    assert!(is_feature_verdict(
        &["ready".to_string()],
        "Unlabeled story"
    ));
}

#[test]
fn is_feature_verdict_label_beats_title() {
    use crate::session::velocity::is_feature_verdict;
    // Label is authoritative when both label and title carry a type.
    // Label says Chore, title says [Feature] → Chore wins.
    assert!(!is_feature_verdict(
        &["Chore".to_string()],
        "[Feature] misleading prefix"
    ));
}
