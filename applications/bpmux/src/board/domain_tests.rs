#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::domain::{Story, StoryState, StoryType, Tag, story_to_board_json};

fn sample_story(state: StoryState, story_type: StoryType, is_closed: bool) -> Story {
    Story {
        number: 42,
        title: "[Feature] sample".to_string(),
        state,
        story_type,
        tags: Vec::new(),
        filed_at: chrono::DateTime::parse_from_rfc3339("2026-04-26T01:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        filed_by: "pm:test".to_string(),
        body: Some("body content".to_string()),
        free_comments: Some(Vec::new()),
        gate_comments: Some(Vec::new()),
        is_closed,
        parent_epic: None,
        acceptance_test_path: None,
    }
}

#[test]
fn story_to_board_json_emits_renderer_wire_format_field_list() {
    let s = sample_story(StoryState::Icebox, StoryType::Feature, false);
    let json = story_to_board_json(&s);

    let obj = json.as_object().expect("object");
    let keys: Vec<&String> = obj.keys().collect();
    for required in &[
        "number",
        "title",
        "state",
        "gh_state",
        "labels",
        "parent_epic",
        "assignees",
        "created_at",
        "body",
    ] {
        assert!(
            keys.iter().any(|k| k.as_str() == *required),
            "wire-format must include `{required}` (drift catches #451-class regression); keys present: {keys:?}"
        );
    }
    assert_eq!(obj.len(), 9, "no spurious keys; keys present: {keys:?}");
}

#[test]
fn story_to_board_json_emits_canonical_8_state_vocabulary() {
    for (state, expected) in &[
        (StoryState::Icebox, "icebox"),
        (StoryState::Ready, "ready"),
        (StoryState::InProgress, "in-progress"),
        (StoryState::Delivered, "delivered"),
        (StoryState::Accepted, "accepted"),
        (StoryState::ReadyForAccept, "ready-for-accept"),
        (StoryState::NeedsFixes, "needs-fixes"),
        (StoryState::Rejected, "rejected"),
    ] {
        let s = sample_story(*state, StoryType::Feature, false);
        assert_eq!(
            json_str(&story_to_board_json(&s), "state"),
            *expected,
            "{state:?} serializes to canonical wire-vocab `{expected}` per #505 boundary-principle"
        );
    }
}

#[test]
fn story_to_board_json_is_closed_emits_gh_state_closed_independent_of_state() {
    let s = sample_story(StoryState::Icebox, StoryType::Feature, true);
    assert_eq!(
        json_str(&story_to_board_json(&s), "state"),
        "icebox",
        "Story.state stays canonical regardless of gh-side is_closed; the GH-adapter is_closed→Accepted translation happens at INGEST not at serialization"
    );
    assert_eq!(json_str(&story_to_board_json(&s), "gh_state"), "closed");
}

#[test]
fn story_to_board_json_gh_state_lowercase_open_or_closed() {
    let open = sample_story(StoryState::Icebox, StoryType::Feature, false);
    assert_eq!(json_str(&story_to_board_json(&open), "gh_state"), "open");
    let closed = sample_story(StoryState::Accepted, StoryType::Feature, true);
    assert_eq!(
        json_str(&story_to_board_json(&closed), "gh_state"),
        "closed"
    );
}

#[test]
fn story_to_board_json_labels_include_story_type() {
    for (story_type, expected_label) in &[
        (StoryType::Feature, "feature"),
        (StoryType::Bug, "bug"),
        (StoryType::Chore, "chore"),
        (StoryType::Epic, "epic"),
    ] {
        let s = sample_story(StoryState::Icebox, *story_type, false);
        let json = story_to_board_json(&s);
        let labels = json["labels"].as_array().expect("labels array");
        assert!(
            labels.iter().any(|l| l.as_str() == Some(expected_label)),
            "labels include `{expected_label}` for {story_type:?}"
        );
    }
}

#[test]
fn story_to_board_json_labels_include_blocked_tag() {
    let mut s = sample_story(StoryState::Icebox, StoryType::Feature, false);
    s.tags.push(Tag::Blocked);
    let json = story_to_board_json(&s);
    let labels = json["labels"].as_array().expect("labels array");
    assert!(labels.iter().any(|l| l.as_str() == Some("blocked")));
}

#[test]
fn story_to_board_json_parent_epic_emits_field_and_label() {
    let mut s = sample_story(StoryState::Icebox, StoryType::Feature, false);
    s.parent_epic = Some(7);
    let json = story_to_board_json(&s);
    assert_eq!(json["parent_epic"].as_u64(), Some(7));
    let labels = json["labels"].as_array().expect("labels array");
    assert!(
        labels.iter().any(|l| l.as_str() == Some("parent-epic-#7")),
        "labels include `parent-epic-#7`; got {labels:?}"
    );
}

#[test]
fn story_to_board_json_no_parent_epic_emits_null() {
    let s = sample_story(StoryState::Icebox, StoryType::Feature, false);
    let json = story_to_board_json(&s);
    assert!(
        json["parent_epic"].is_null(),
        "parent_epic null when absent; got {:?}",
        json["parent_epic"]
    );
}

#[test]
fn story_to_board_json_body_unwraps_to_empty_string_when_none() {
    let mut s = sample_story(StoryState::Icebox, StoryType::Feature, false);
    s.body = None;
    let json = story_to_board_json(&s);
    assert_eq!(json["body"].as_str(), Some(""));
}

#[test]
fn story_to_board_json_assignees_empty_array() {
    let s = sample_story(StoryState::Icebox, StoryType::Feature, false);
    let json = story_to_board_json(&s);
    let assignees = json["assignees"].as_array().expect("assignees array");
    assert!(assignees.is_empty());
}

#[test]
fn story_to_board_json_created_at_is_rfc3339() {
    let s = sample_story(StoryState::Icebox, StoryType::Feature, false);
    let json = story_to_board_json(&s);
    let created = json["created_at"].as_str().expect("created_at string");
    chrono::DateTime::parse_from_rfc3339(created).expect("created_at parses as RFC3339");
}

fn json_str<'a>(json: &'a serde_json::Value, key: &str) -> &'a str {
    json[key].as_str().unwrap_or_else(|| {
        panic!("key `{key}` not a string in {json}");
    })
}
