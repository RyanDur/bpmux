#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::velocity::{SPARSE_THRESHOLD, VelocityMetrics, WINDOW_DAYS, compute};
use crate::message::{Message, MessageType};
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
        msg("#3 delivered", now() - ChronoDuration::days(20)),
        msg("#4 delivered", now() + ChronoDuration::hours(1)),
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
    assert_eq!(m.median_deliver_to_accept_hours, Some(3.25));
}

#[test]
fn rework_rate_counts_failed_desk_checks_per_unique_delivery() {
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
    assert_eq!(m.delivered_in_window, 4);
    assert_eq!(m.rework_rate, Some(0.25));
}

#[test]
fn re_delivered_story_counts_once_in_delivered_in_window() {
    let base = now() - ChronoDuration::days(1);
    let messages = vec![
        msg("#50 delivered", base),
        msg(
            "#50 failed desk-check: fix prefix",
            base + ChronoDuration::hours(1),
        ),
        msg("#50 delivered", base + ChronoDuration::hours(2)),
        msg(
            "#50 failed desk-check: missing test",
            base + ChronoDuration::hours(3),
        ),
        msg("#50 delivered", base + ChronoDuration::hours(4)),
    ];
    let m = compute(&messages, now(), |_| true);
    assert_eq!(
        m.delivered_in_window, 1,
        "re-delivered story counts as ONE unique delivery"
    );
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
        message_type: MessageType::Activity,
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
        msg("delivered yesterday", now() - ChronoDuration::hours(2)),
    ];
    let m = compute(&messages, now(), |_| true);
    assert_eq!(m.delivered_in_window, 0);
}

#[test]
fn plumbing_stories_filtered_out_of_velocity() {
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
    use crate::velocity::is_feature_verdict;
    assert!(is_feature_verdict(&["Feature".to_string()], ""));
    assert!(is_feature_verdict(
        &["ready".to_string(), "Feature".to_string()],
        ""
    ));
}

#[test]
fn is_feature_verdict_rejects_chore_and_bug_labels() {
    use crate::velocity::is_feature_verdict;
    assert!(!is_feature_verdict(&["Chore".to_string()], ""));
    assert!(!is_feature_verdict(&["Bug".to_string()], ""));
}

#[test]
fn is_feature_verdict_falls_back_to_title_prefix() {
    use crate::velocity::is_feature_verdict;
    assert!(is_feature_verdict(&[], "[Feature] ship the thing"));
    assert!(!is_feature_verdict(&[], "[Chore] refactor the thing"));
    assert!(!is_feature_verdict(&[], "[Bug] fix the thing"));
    assert!(!is_feature_verdict(&[], "Bug: fix the thing"));
}

#[test]
fn is_feature_verdict_supports_legacy_prefixes_during_migration() {
    use crate::velocity::is_feature_verdict;
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
    use crate::velocity::is_feature_verdict;
    assert!(is_feature_verdict(&[], ""));
    assert!(is_feature_verdict(
        &["ready".to_string()],
        "Unlabeled story"
    ));
}

#[test]
fn is_feature_verdict_label_beats_title() {
    use crate::velocity::is_feature_verdict;
    assert!(!is_feature_verdict(
        &["Chore".to_string()],
        "[Feature] misleading prefix"
    ));
}

#[test]
fn classify_story_decodes_epic_from_title_prefix() {
    use crate::board::domain::StoryType;
    use crate::velocity::classify_story;
    assert_eq!(classify_story(&[], "[Epic] big arc"), StoryType::Epic);
    assert_eq!(
        classify_story(&[], "[epic] case-insensitive"),
        StoryType::Epic
    );
}

#[test]
fn classify_story_decodes_epic_from_label() {
    use crate::board::domain::StoryType;
    use crate::velocity::classify_story;
    let labels = vec!["epic".to_string()];
    assert_eq!(
        classify_story(&labels, "[Feature] mismatched"),
        StoryType::Epic
    );
}

#[test]
fn wip_history_is_a_real_per_activity_day_in_progress_series() {
    let messages = vec![
        msg("#1 started", now() - ChronoDuration::days(4)),
        msg("#2 started", now() - ChronoDuration::days(3)),
        msg("#3 started", now() - ChronoDuration::days(2)),
        msg("#1 delivered", now() - ChronoDuration::days(1)),
    ];
    let m = compute(&messages, now(), |_| true);
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
    let messages = vec![
        msg("#1 started", now() - ChronoDuration::days(20)),
        msg("#2 started", now() - ChronoDuration::days(2)),
        msg("#2 delivered", now() - ChronoDuration::days(1)),
    ];
    let m = compute(&messages, now(), |_| true);
    let counts: Vec<usize> = m.wip_history.iter().map(|p| p.wip_count).collect();
    assert_eq!(
        counts,
        vec![1, 0],
        "the out-of-window #1 start is excluded; delivering #2 returns to zero (no underflow)"
    );
}
