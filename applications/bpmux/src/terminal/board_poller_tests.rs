#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::board_poller::{BoardFetchError, format_last_updated};
use chrono::{DateTime, Utc};

fn at(iso: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(iso)
        .unwrap()
        .with_timezone(&Utc)
}

#[test]
fn format_last_updated_seconds_for_under_minute() {
    let last = at("2026-04-16T03:00:00Z");
    let now = at("2026-04-16T03:00:30Z");
    assert_eq!(format_last_updated(last, now), "last updated 30s ago");
}

#[test]
fn format_last_updated_minutes_for_under_hour() {
    let last = at("2026-04-16T03:00:00Z");
    let now = at("2026-04-16T03:05:00Z");
    assert_eq!(format_last_updated(last, now), "last updated 5m ago");
}

#[test]
fn format_last_updated_hours_for_over_hour() {
    let last = at("2026-04-16T03:00:00Z");
    let now = at("2026-04-16T05:30:00Z");
    assert_eq!(format_last_updated(last, now), "last updated 2h ago");
}

#[test]
fn format_last_updated_never_shows_zero_minutes() {
    // 59 seconds = "59s ago", not "0m ago"
    let last = at("2026-04-16T03:00:00Z");
    let now = at("2026-04-16T03:00:59Z");
    assert_eq!(format_last_updated(last, now), "last updated 59s ago");
}

#[test]
fn error_message_auth_invalid() {
    let e = BoardFetchError::AuthInvalid;
    let msg = e.user_message();
    assert!(msg.contains("GITHUB_TOKEN"), "got: {msg}");
    assert!(msg.contains("missing or invalid"), "got: {msg}");
}

#[test]
fn error_message_rate_limited_with_retry_after() {
    let e = BoardFetchError::RateLimited {
        retry_after_secs: Some(30),
    };
    let msg = e.user_message();
    assert!(msg.contains("rate limited"), "got: {msg}");
    assert!(msg.contains("30s"), "got: {msg}");
}

#[test]
fn error_message_rate_limited_without_retry_after() {
    let e = BoardFetchError::RateLimited {
        retry_after_secs: None,
    };
    let msg = e.user_message();
    assert!(msg.contains("rate limited"), "got: {msg}");
    assert!(
        !msg.contains("s "),
        "no retry-after should mean no 's' suffix. got: {msg}"
    );
}

#[test]
fn error_message_network_unreachable() {
    let e = BoardFetchError::NetworkUnreachable;
    assert_eq!(e.user_message(), "offline");
}

#[test]
fn error_message_other_includes_detail() {
    let e = BoardFetchError::Other("500 internal".to_string());
    let msg = e.user_message();
    assert!(msg.contains("500 internal"), "got: {msg}");
}
