#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::relationship_block::{embed, extract};
use serde_json::json;

#[test]
fn extract_returns_none_when_body_has_no_marker() {
    assert!(extract("just a human description\n\nwith prose").is_none());
}

#[test]
fn extract_returns_none_when_body_is_empty() {
    assert!(extract("").is_none());
}

#[test]
fn embed_then_extract_round_trips_the_block() {
    let block = json!({ "stories": [710, 720, 730], "outcome": 700 });
    let body = embed("Outcome prose.", &block);
    assert_eq!(extract(&body), Some(block));
}

#[test]
fn embed_preserves_the_human_prose() {
    let body = embed(
        "## Why\n\nThe human-written description.",
        &json!({ "iterations": [1, 2] }),
    );
    assert!(body.contains("## Why"));
    assert!(body.contains("The human-written description."));
}

#[test]
fn embed_replaces_an_existing_block_without_duplicating() {
    let first = embed("prose", &json!({ "stories": [1] }));
    let second = embed(&first, &json!({ "stories": [1, 2] }));
    assert_eq!(extract(&second), Some(json!({ "stories": [1, 2] })));
    assert_eq!(
        second.matches("bpmux-relationships").count(),
        1,
        "embedding twice must replace the block, never duplicate it"
    );
}

#[test]
fn extract_ignores_unrelated_html_comments() {
    let body = "text\n\n<!-- a normal comment -->\n\nmore text";
    assert!(extract(body).is_none());
}
