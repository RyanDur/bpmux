use super::*;
use crate::board::adapters::gh_label_codec::GhLabelCodec;
use crate::board::domain::{Story, StoryState};
use crate::board::label_codec::LabelCodec;
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
    assert_eq!(
        parse_ready_order_label(&GhLabelCodec, "ready-order.100"),
        Some(100)
    );
    assert_eq!(
        parse_ready_order_label(&GhLabelCodec, "ready-order.250"),
        Some(250)
    );
}

#[test]
fn parse_rejects_nondigit() {
    assert_eq!(
        parse_ready_order_label(&GhLabelCodec, "ready-order.foo"),
        None
    );
}

#[test]
fn parse_rejects_missing_position() {
    assert_eq!(parse_ready_order_label(&GhLabelCodec, "ready-order"), None);
    assert_eq!(parse_ready_order_label(&GhLabelCodec, "ready-order."), None);
}

#[test]
fn parse_rejects_other_label_shapes() {
    assert_eq!(parse_ready_order_label(&GhLabelCodec, "ready"), None);
    assert_eq!(
        parse_ready_order_label(&GhLabelCodec, "iteration-Pegasus.100"),
        None
    );
    assert_eq!(
        parse_ready_order_label(&GhLabelCodec, "ready-order-100"),
        None
    );
}

#[test]
fn is_ready_order_label_predicate_matches_parse() {
    assert!(is_ready_order_label(&GhLabelCodec, "ready-order.42"));
    assert!(!is_ready_order_label(&GhLabelCodec, "ready-order.foo"));
    assert!(!is_ready_order_label(
        &GhLabelCodec,
        "iteration-Pegasus.100"
    ));
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
    assert_eq!(story_ready_order_position(&GhLabelCodec, &s), Some(250));
}

#[test]
fn story_position_returns_none_when_absent() {
    let s = seed(
        500,
        StoryState::Ready,
        vec!["ready".to_string(), "feature".to_string()],
    );
    assert_eq!(story_ready_order_position(&GhLabelCodec, &s), None);
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
    assert_eq!(story_ready_order_position(&GhLabelCodec, &s), Some(100));
}

#[test]
fn stories_with_ready_order_sorts_ascending_by_position() {
    let stories = vec![
        seed(500, StoryState::Ready, vec!["ready-order.300".to_string()]),
        seed(501, StoryState::Ready, vec!["ready-order.100".to_string()]),
        seed(502, StoryState::Ready, vec!["ready-order.200".to_string()]),
    ];
    assert_eq!(
        stories_with_ready_order(&GhLabelCodec, &stories),
        vec![501, 502, 500]
    );
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
    assert_eq!(stories_with_ready_order(&GhLabelCodec, &stories), vec![500]);
}

#[test]
fn stories_with_ready_order_skips_closed_stories() {
    let mut s = seed(500, StoryState::Ready, vec!["ready-order.100".to_string()]);
    s.is_closed = true;
    assert!(stories_with_ready_order(&GhLabelCodec, &[s]).is_empty());
}

#[test]
fn stories_with_ready_order_skips_unlabeled_stories() {
    let stories = vec![
        seed(500, StoryState::Ready, vec!["ready-order.100".to_string()]),
        seed(501, StoryState::Ready, vec!["ready".to_string()]),
    ];
    assert_eq!(stories_with_ready_order(&GhLabelCodec, &stories), vec![500]);
}

fn ready_value_with_order(number: u64, position: u32) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "ready",
        "title": "s",
        "labels": [{"name": "ready"}, {"name": format!("ready-order.{position}")}],
    })
}

fn value(number: u64, state: &str) -> serde_json::Value {
    serde_json::json!({ "number": number, "state": state, "title": "s" })
}

fn numbers_of(stories: &[serde_json::Value]) -> Vec<u64> {
    stories
        .iter()
        .filter_map(|s| s.get("number").and_then(|v| v.as_u64()))
        .collect()
}

#[test]
fn sort_board_ready_sorts_by_ready_order_label_when_present() {
    let stories = vec![
        ready_value_with_order(300, 300),
        ready_value_with_order(100, 100),
        ready_value_with_order(200, 200),
    ];
    let sorted = sort_board_by_ready_order_labels(&GhLabelCodec, stories);
    assert_eq!(numbers_of(&sorted), vec![100, 200, 300]);
}

#[test]
fn sort_board_ready_unlabeled_stories_fall_to_end_by_issue_number() {
    let stories = vec![
        value(300, "ready"),
        ready_value_with_order(100, 50),
        value(200, "ready"),
    ];
    let sorted = sort_board_by_ready_order_labels(&GhLabelCodec, stories);
    assert_eq!(numbers_of(&sorted), vec![100, 200, 300]);
}

#[test]
fn sort_board_leaves_non_ready_states_in_encounter_order() {
    let stories = vec![
        value(500, "delivered"),
        ready_value_with_order(100, 100),
        value(400, "delivered"),
    ];
    let sorted = sort_board_by_ready_order_labels(&GhLabelCodec, stories);
    assert_eq!(numbers_of(&sorted), vec![500, 400, 100]);
}

#[test]
fn sort_board_preserves_inter_state_encounter_order() {
    let stories = vec![
        value(500, "delivered"),
        ready_value_with_order(100, 100),
        value(600, "delivered"),
    ];
    let sorted = sort_board_by_ready_order_labels(&GhLabelCodec, stories);
    assert_eq!(numbers_of(&sorted), vec![500, 600, 100]);
}

#[test]
fn sort_board_empty_input_is_identity() {
    let sorted = sort_board_by_ready_order_labels(&GhLabelCodec, vec![]);
    assert!(sorted.is_empty());
}

struct StubLabelCodec;

impl LabelCodec for StubLabelCodec {
    fn decode_ready_order(&self, label: &str) -> Option<u32> {
        label.strip_prefix("rdy:")?.parse().ok()
    }
    fn decode_iteration(
        &self,
        _label: &str,
    ) -> Option<crate::board::iteration_label::IterationLabelParts> {
        None
    }
    fn is_iteration_label_loose(&self, _label: &str) -> bool {
        false
    }
    fn encode_parent_epic(&self, issue_number: u64) -> String {
        format!("parent|{issue_number}")
    }
}

#[test]
fn substrate_test_stories_with_ready_order_works_with_arbitrary_codec_syntax() {
    let stories = vec![
        seed(500, StoryState::Ready, vec!["rdy:300".to_string()]),
        seed(501, StoryState::Ready, vec!["rdy:100".to_string()]),
        seed(502, StoryState::Ready, vec!["rdy:200".to_string()]),
    ];
    assert_eq!(
        stories_with_ready_order(&StubLabelCodec, &stories),
        vec![501, 502, 500],
        "stub-syntax rdy:N sorts correctly without GH-syntax dependency"
    );
}
