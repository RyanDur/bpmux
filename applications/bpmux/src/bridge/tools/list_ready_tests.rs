#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::list_ready::build_ready_queue;
use crate::board::adapters::gh_label_codec::GhLabelCodec;
use crate::board::label_codec::LabelCodec;

fn story(number: u64, state: &str, title: &str) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": state,
        "title": title,
        "labels": [{"name": state}],
    })
}

fn ready_story_with_order(number: u64, position: u32) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "ready",
        "title": "s",
        "labels": [{"name": "ready"}, {"name": format!("ready-order.{position}")}],
    })
}

#[test]
fn filters_out_non_ready_stories() {
    let stories = vec![
        story(100, "ready", "a"),
        story(200, "delivered", "b"),
        story(300, "in progress", "c"),
    ];
    let queue = build_ready_queue(&GhLabelCodec, stories);
    let numbers: Vec<u64> = queue
        .iter()
        .map(|s| s["number"].as_u64().unwrap())
        .collect();
    assert_eq!(numbers, vec![100]);
}

#[test]
fn assigns_position_starting_at_one() {
    let stories = vec![
        ready_story_with_order(100, 100),
        ready_story_with_order(200, 200),
        ready_story_with_order(300, 300),
    ];
    let queue = build_ready_queue(&GhLabelCodec, stories);
    let positions: Vec<u64> = queue
        .iter()
        .map(|s| s["position"].as_u64().unwrap())
        .collect();
    assert_eq!(positions, vec![1, 2, 3]);
}

#[test]
fn position_reflects_label_position_not_fetch_order() {
    let stories = vec![
        ready_story_with_order(100, 300),
        ready_story_with_order(200, 100),
        ready_story_with_order(300, 200),
    ];
    let queue = build_ready_queue(&GhLabelCodec, stories);
    assert_eq!(queue[0]["number"].as_u64().unwrap(), 200);
    assert_eq!(queue[0]["position"].as_u64().unwrap(), 1);
    assert_eq!(queue[1]["number"].as_u64().unwrap(), 300);
    assert_eq!(queue[1]["position"].as_u64().unwrap(), 2);
    assert_eq!(queue[2]["number"].as_u64().unwrap(), 100);
    assert_eq!(queue[2]["position"].as_u64().unwrap(), 3);
}

#[test]
fn preserves_title_and_labels() {
    let stories = vec![story(100, "ready", "my title")];
    let queue = build_ready_queue(&GhLabelCodec, stories);
    assert_eq!(queue[0]["title"].as_str().unwrap(), "my title");
    assert_eq!(queue[0]["labels"][0]["name"].as_str().unwrap(), "ready");
}

#[test]
fn closed_or_delivered_stories_do_not_leak() {
    let stories = vec![
        story(100, "delivered", "a"),
        story(200, "ready", "b"),
        story(300, "closed", "c"),
    ];
    let queue = build_ready_queue(&GhLabelCodec, stories);
    let numbers: Vec<u64> = queue
        .iter()
        .map(|s| s["number"].as_u64().unwrap())
        .collect();
    assert_eq!(numbers, vec![200]);
}

#[test]
fn empty_queue_returns_empty_array() {
    let queue = build_ready_queue(&GhLabelCodec, vec![]);
    assert!(queue.is_empty());
}

#[test]
fn unlabeled_ready_stories_sort_by_issue_number() {
    let stories = vec![
        story(300, "ready", "c"),
        story(100, "ready", "a"),
        story(200, "ready", "b"),
    ];
    let queue = build_ready_queue(&GhLabelCodec, stories);
    let numbers: Vec<u64> = queue
        .iter()
        .map(|s| s["number"].as_u64().unwrap())
        .collect();
    assert_eq!(numbers, vec![100, 200, 300]);
}

#[test]
fn mixed_labeled_and_unlabeled_ready_stories_labels_first() {
    let stories = vec![
        ready_story_with_order(100, 50),
        story(200, "ready", "b"),
        story(300, "ready", "c"),
    ];
    let queue = build_ready_queue(&GhLabelCodec, stories);
    let numbers: Vec<u64> = queue
        .iter()
        .map(|s| s["number"].as_u64().unwrap())
        .collect();
    assert_eq!(
        numbers,
        vec![100, 200, 300],
        "labeled first (by position), unlabeled after (by issue number)"
    );
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
    fn encode_ready_order(&self, position: u32) -> String {
        format!("rdy:{position}")
    }
    fn encode_iteration(&self, _theme_name: &str, _position: u32) -> String {
        String::new()
    }
    fn iteration_label_class(&self, _label: &str) -> Option<String> {
        None
    }
}

#[test]
fn substrate_test_build_ready_queue_works_with_arbitrary_codec_syntax() {
    let stories = vec![
        serde_json::json!({
            "number": 100,
            "state": "ready",
            "title": "a",
            "labels": [{"name": "ready"}, {"name": "rdy:200"}],
        }),
        serde_json::json!({
            "number": 200,
            "state": "ready",
            "title": "b",
            "labels": [{"name": "ready"}, {"name": "rdy:100"}],
        }),
    ];
    let queue = build_ready_queue(&StubLabelCodec, stories);
    assert_eq!(
        queue[0]["number"].as_u64().unwrap(),
        200,
        "rdy:100 sorts first under stub syntax"
    );
    assert_eq!(queue[1]["number"].as_u64().unwrap(), 100);
}
