use super::*;
use crate::board::domain::{Story, StoryState};
use chrono::{TimeZone, Utc};

fn seed(number: u64, state: StoryState, raw_labels: Vec<String>) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("story-{number}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 16, 0, 0, 0).unwrap(),
        "dev".to_string(),
    );
    s.raw_labels = raw_labels;
    s
}

#[test]
fn parse_simple_position() {
    assert_eq!(parse_ready_order_label("ready-order.100"), Some(100));
    assert_eq!(parse_ready_order_label("ready-order.250"), Some(250));
}

#[test]
fn parse_rejects_nondigit() {
    assert_eq!(parse_ready_order_label("ready-order.foo"), None);
}

#[test]
fn parse_rejects_missing_position() {
    assert_eq!(parse_ready_order_label("ready-order"), None);
    assert_eq!(parse_ready_order_label("ready-order."), None);
}

#[test]
fn parse_rejects_other_label_shapes() {
    assert_eq!(parse_ready_order_label("ready"), None);
    assert_eq!(parse_ready_order_label("iteration-Pegasus.100"), None);
    assert_eq!(parse_ready_order_label("ready-order-100"), None);
}

#[test]
fn is_ready_order_label_predicate_matches_parse() {
    assert!(is_ready_order_label("ready-order.42"));
    assert!(!is_ready_order_label("ready-order.foo"));
    assert!(!is_ready_order_label("iteration-Pegasus.100"));
}

#[test]
fn story_position_returns_label_value() {
    let s = seed(
        500,
        StoryState::Ready,
        vec![
            "ready".to_string(),
            "ready-order.250".to_string(),
            "feature".to_string(),
        ],
    );
    assert_eq!(story_ready_order_position(&s), Some(250));
}

#[test]
fn story_position_returns_none_when_absent() {
    let s = seed(
        500,
        StoryState::Ready,
        vec!["ready".to_string(), "feature".to_string()],
    );
    assert_eq!(story_ready_order_position(&s), None);
}

#[test]
fn story_position_returns_min_when_multiple() {
    let s = seed(
        500,
        StoryState::Ready,
        vec![
            "ready-order.300".to_string(),
            "ready-order.100".to_string(),
            "ready-order.200".to_string(),
        ],
    );
    assert_eq!(story_ready_order_position(&s), Some(100));
}

#[test]
fn stories_with_ready_order_sorts_ascending_by_position() {
    let stories = vec![
        seed(500, StoryState::Ready, vec!["ready-order.300".to_string()]),
        seed(501, StoryState::Ready, vec!["ready-order.100".to_string()]),
        seed(502, StoryState::Ready, vec!["ready-order.200".to_string()]),
    ];
    assert_eq!(stories_with_ready_order(&stories), vec![501, 502, 500]);
}

#[test]
fn stories_with_ready_order_skips_non_ready_state() {
    let stories = vec![
        seed(500, StoryState::Ready, vec!["ready-order.100".to_string()]),
        seed(501, StoryState::Icebox, vec!["ready-order.50".to_string()]),
        seed(
            502,
            StoryState::InProgress,
            vec!["ready-order.25".to_string()],
        ),
    ];
    assert_eq!(stories_with_ready_order(&stories), vec![500]);
}

#[test]
fn stories_with_ready_order_skips_closed_stories() {
    let mut s = seed(500, StoryState::Ready, vec!["ready-order.100".to_string()]);
    s.is_closed = true;
    assert!(stories_with_ready_order(&[s]).is_empty());
}

#[test]
fn stories_with_ready_order_skips_unlabeled_stories() {
    let stories = vec![
        seed(500, StoryState::Ready, vec!["ready-order.100".to_string()]),
        seed(501, StoryState::Ready, vec!["ready".to_string()]),
    ];
    assert_eq!(stories_with_ready_order(&stories), vec![500]);
}
