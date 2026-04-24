#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::list_ready::build_ready_queue;
use crate::session::priority::PriorityIndex;

fn story(number: u64, state: &str, title: &str) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": state,
        "title": title,
        "labels": [{"name": state}],
    })
}

#[test]
fn filters_out_non_ready_stories() {
    let stories = vec![
        story(100, "ready", "a"),
        story(200, "delivered", "b"),
        story(300, "in progress", "c"),
    ];
    let queue = build_ready_queue(stories, &PriorityIndex::default());
    let numbers: Vec<u64> = queue
        .iter()
        .map(|s| s["number"].as_u64().unwrap())
        .collect();
    assert_eq!(numbers, vec![100]);
}

#[test]
fn applies_priority_order_to_ready_column() {
    let mut priority = PriorityIndex::default();
    priority.columns.insert("ready".to_string(), vec![300, 100]);
    let stories = vec![
        story(100, "ready", "a"),
        story(200, "ready", "b"),
        story(300, "ready", "c"),
    ];
    let queue = build_ready_queue(stories, &priority);
    let numbers: Vec<u64> = queue
        .iter()
        .map(|s| s["number"].as_u64().unwrap())
        .collect();
    assert_eq!(numbers, vec![300, 100, 200]);
}

#[test]
fn assigns_position_starting_at_one() {
    let stories = vec![
        story(100, "ready", "a"),
        story(200, "ready", "b"),
        story(300, "ready", "c"),
    ];
    let queue = build_ready_queue(stories, &PriorityIndex::default());
    let positions: Vec<u64> = queue
        .iter()
        .map(|s| s["position"].as_u64().unwrap())
        .collect();
    assert_eq!(positions, vec![1, 2, 3]);
}

#[test]
fn position_reflects_priority_order_not_fetch_order() {
    let mut priority = PriorityIndex::default();
    priority.columns.insert("ready".to_string(), vec![300, 100]);
    let stories = vec![
        story(100, "ready", "a"),
        story(200, "ready", "b"),
        story(300, "ready", "c"),
    ];
    let queue = build_ready_queue(stories, &priority);
    assert_eq!(queue[0]["number"].as_u64().unwrap(), 300);
    assert_eq!(queue[0]["position"].as_u64().unwrap(), 1);
    assert_eq!(queue[1]["number"].as_u64().unwrap(), 100);
    assert_eq!(queue[1]["position"].as_u64().unwrap(), 2);
    assert_eq!(queue[2]["number"].as_u64().unwrap(), 200);
    assert_eq!(queue[2]["position"].as_u64().unwrap(), 3);
}

#[test]
fn preserves_title_and_labels() {
    let stories = vec![story(100, "ready", "my title")];
    let queue = build_ready_queue(stories, &PriorityIndex::default());
    assert_eq!(queue[0]["title"].as_str().unwrap(), "my title");
    assert_eq!(queue[0]["labels"][0]["name"].as_str().unwrap(), "ready");
}

#[test]
fn stories_not_in_priority_index_append_after_prioritized_ones() {
    let mut priority = PriorityIndex::default();
    priority.columns.insert("ready".to_string(), vec![300]);
    let stories = vec![
        story(100, "ready", "a"),
        story(200, "ready", "b"),
        story(300, "ready", "c"),
    ];
    let queue = build_ready_queue(stories, &priority);
    let numbers: Vec<u64> = queue
        .iter()
        .map(|s| s["number"].as_u64().unwrap())
        .collect();
    assert_eq!(numbers[0], 300);
    assert!(numbers[1..].contains(&100));
    assert!(numbers[1..].contains(&200));
}

#[test]
fn closed_or_delivered_stories_do_not_leak_even_if_in_priority_index() {
    let mut priority = PriorityIndex::default();
    priority
        .columns
        .insert("ready".to_string(), vec![300, 100, 200]);
    let stories = vec![
        story(100, "delivered", "a"),
        story(200, "ready", "b"),
        story(300, "closed", "c"),
    ];
    let queue = build_ready_queue(stories, &priority);
    let numbers: Vec<u64> = queue
        .iter()
        .map(|s| s["number"].as_u64().unwrap())
        .collect();
    assert_eq!(numbers, vec![200]);
}

#[test]
fn empty_queue_returns_empty_array() {
    let queue = build_ready_queue(vec![], &PriorityIndex::default());
    assert!(queue.is_empty());
}

#[test]
fn no_priority_entry_for_ready_passes_through_fetch_order() {
    let priority = PriorityIndex::default();
    let stories = vec![
        story(100, "ready", "a"),
        story(200, "ready", "b"),
        story(300, "ready", "c"),
    ];
    let queue = build_ready_queue(stories, &priority);
    let numbers: Vec<u64> = queue
        .iter()
        .map(|s| s["number"].as_u64().unwrap())
        .collect();
    assert_eq!(numbers, vec![100, 200, 300]);
}
