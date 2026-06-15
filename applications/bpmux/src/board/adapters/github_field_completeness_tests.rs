#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::board::adapters::github::decode_story_metadata;
use crate::board::domain::Story;

#[test]
fn decode_story_metadata_covers_every_metadata_axis_field_via_roundtrip() {
    let json = serde_json::json!({
        "number": 42,
        "title": "FIELD_COMPLETENESS_SENTINEL_TITLE",
        "state": "open",
        "gh_state": "open",
        "labels": [
            {"name": "feature"},
            {"name": "in progress"},
            {"name": "parent-epic-#100"},
            {"name": "iteration-#200"},
            {"name": "blocked"},
            {"name": "awaiting owner"},
            {"name": "iteration-review-closed.designer"},
        ],
        "created_at": "2026-05-15T01:00:00+00:00",
        "user": {"login": "FIELD_COMPLETENESS_SENTINEL_FILER"},
        "body": "## Acceptance test path\n\ntests/field_completeness_sentinel.rs\n",
    });
    let decoded = decode_story_metadata(&json).expect("sentinel JSON must decode");
    assert_metadata_axis_complete(&decoded);
}

fn assert_metadata_axis_complete(decoded: &Story) {
    let Story {
        number,
        title,
        state,
        story_type,
        tags,
        filed_at,
        filed_by,
        body,
        free_comments,
        gate_comments,
        is_closed,
        parent_epic,
        acceptance_test_path,
        raw_labels,
        body_etag,
        iteration_membership: _,
        iteration_order: _,
        iteration_review_closed_by,
        is_current: _,
        acceptor,
    } = decoded;

    assert_eq!(*number, 42, "number must roundtrip from JSON");
    assert_eq!(
        title.as_str(),
        "FIELD_COMPLETENESS_SENTINEL_TITLE",
        "title must roundtrip from JSON"
    );
    assert_eq!(
        *state,
        crate::board::domain::StoryState::InProgress,
        "state must decode from labels"
    );
    assert_eq!(
        *story_type,
        crate::board::domain::StoryType::Feature,
        "story_type must decode from labels"
    );
    assert_eq!(tags.len(), 1, "tags must decode from labels (blocked)");
    assert_eq!(
        filed_at.to_rfc3339(),
        "2026-05-15T01:00:00+00:00",
        "filed_at must roundtrip from created_at"
    );
    assert_eq!(
        filed_by.as_str(),
        "FIELD_COMPLETENESS_SENTINEL_FILER",
        "filed_by must roundtrip from user.login"
    );
    assert_eq!(
        body.as_deref(),
        Some("## Acceptance test path\n\ntests/field_completeness_sentinel.rs\n"),
        "body must roundtrip from JSON (closes 5ae7266a cascade-fix gap)"
    );
    assert!(!*is_closed, "is_closed must decode from gh_state");
    assert_eq!(
        *parent_epic,
        Some(100),
        "parent_epic must decode from labels"
    );
    assert_eq!(
        acceptance_test_path.as_deref(),
        Some("tests/field_completeness_sentinel.rs"),
        "acceptance_test_path must decode from body"
    );
    assert!(
        !raw_labels.is_empty(),
        "raw_labels must roundtrip from JSON labels"
    );
    assert_eq!(
        *acceptor,
        Some(crate::board::domain::Acceptor::Owner),
        "acceptor must decode from the 'awaiting owner' label"
    );
    assert_eq!(
        iteration_review_closed_by.as_slice(),
        ["designer".to_string()],
        "iteration_review_closed_by must decode the closer role from the review-close label"
    );

    let _ = free_comments;
    let _ = gate_comments;
    let _ = body_etag;
}

#[test]
fn decode_story_metadata_returns_none_when_body_present_but_atp_absent() {
    let json = serde_json::json!({
        "number": 99,
        "title": "no-ATP story",
        "state": "open",
        "labels": [],
        "created_at": "2026-05-15T01:00:00+00:00",
        "user": {"login": "x"},
        "body": "plain body without acceptance test path section\n",
    });
    let decoded = decode_story_metadata(&json).expect("decode");
    assert_eq!(
        decoded.acceptance_test_path, None,
        "ATP must be None when body lacks the section"
    );
    assert_eq!(
        decoded.body.as_deref(),
        Some("plain body without acceptance test path section\n"),
        "body itself still decodes even when ATP absent"
    );
}
