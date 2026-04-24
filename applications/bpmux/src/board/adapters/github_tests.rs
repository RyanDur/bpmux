#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone, Utc};

use crate::board::adapters::github::{
    classify_gate, decode_gate_comments, decode_story_metadata, is_already_closed_error,
    state_from_labels, state_to_label, story_type_from_labels_or_title, tags_from_labels,
};
use crate::board::domain::{Gate, StoryState, StoryType, Tag};

#[test]
fn state_from_labels_accepted_wins_regardless_of_other_labels() {
    let labels = vec!["ready".to_string(), "accepted".to_string()];
    assert_eq!(state_from_labels(&labels, true), StoryState::Accepted);
    assert_eq!(state_from_labels(&labels, false), StoryState::Accepted);
}

#[test]
fn state_from_labels_closed_issue_is_accepted_even_without_label() {
    let labels = vec![];
    assert_eq!(state_from_labels(&labels, true), StoryState::Accepted);
}

#[test]
fn state_from_labels_rejected_beats_delivered() {
    let labels = vec!["delivered".to_string(), "rejected".to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::Rejected);
}

#[test]
fn state_from_labels_needs_fixes_beats_delivered() {
    let labels = vec!["delivered".to_string(), "needs fixes".to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::NeedsFixes);
}

#[test]
fn state_from_labels_ready_for_accept_beats_delivered() {
    let labels = vec!["delivered".to_string(), "ready for accept".to_string()];
    assert_eq!(
        state_from_labels(&labels, false),
        StoryState::ReadyForAccept
    );
}

#[test]
fn state_from_labels_delivered_without_accept_labels() {
    let labels = vec!["delivered".to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::Delivered);
}

#[test]
fn state_from_labels_in_progress_has_priority_over_ready() {
    let labels = vec!["ready".to_string(), "in progress".to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::InProgress);
}

#[test]
fn state_from_labels_plain_ready() {
    let labels = vec!["ready".to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::Ready);
}

#[test]
fn state_from_labels_icebox_is_the_default() {
    let labels = vec!["Feature".to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::Icebox);
}

#[test]
fn state_from_labels_is_case_insensitive() {
    let labels = vec!["READY".to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::Ready);
}

#[test]
fn state_to_label_roundtrips_ready() {
    let labels = vec![state_to_label(StoryState::Ready).to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::Ready);
}

#[test]
fn state_to_label_roundtrips_needs_fixes_space() {
    let labels = vec![state_to_label(StoryState::NeedsFixes).to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::NeedsFixes);
}

#[test]
fn decode_story_metadata_reads_object_label_shape() {
    let json = serde_json::json!({
        "number": 451,
        "title": "Internal board abstraction",
        "state": "open",
        "labels": [{"name": "in progress"}],
        "created_at": "2026-04-24T13:22:36+00:00",
        "user": {"login": "ryan"},
    });
    let story = decode_story_metadata(&json).expect("story");
    assert_eq!(story.number, 451);
    assert_eq!(story.title, "Internal board abstraction");
    assert_eq!(story.state, StoryState::InProgress);
    assert_eq!(story.filed_by, "ryan");
    assert!(story.body.is_none(), "body should not be eager-populated");
}

#[test]
fn decode_story_metadata_reads_string_label_shape() {
    let json = serde_json::json!({
        "number": 1,
        "title": "t",
        "state": "open",
        "labels": ["ready"],
        "created_at": "2026-04-20T00:00:00Z",
        "user": {"login": "pm"},
    });
    let story = decode_story_metadata(&json).unwrap();
    assert_eq!(story.state, StoryState::Ready);
}

#[test]
fn decode_story_metadata_returns_none_when_number_missing() {
    let json = serde_json::json!({"title": "no number"});
    assert!(decode_story_metadata(&json).is_none());
}

#[test]
fn classify_gate_pickup() {
    let body = "**pickup_paraphrase** (dev pickup-gate):\n\nscope statement";
    let result = classify_gate(body).unwrap();
    assert_eq!(result.0, Gate::Pickup);
    assert_eq!(result.1.as_deref(), Some("scope statement"));
}

#[test]
fn classify_gate_delivered_payload_preserved() {
    let body = "**verified_observation** (dev deliver-gate):\n\nran end-to-end";
    let result = classify_gate(body).unwrap();
    assert_eq!(result.0, Gate::Delivered);
    assert_eq!(result.1.as_deref(), Some("ran end-to-end"));
}

#[test]
fn classify_gate_rejection_default_is_rejected() {
    let body = "**rejection_reason** (reject-gate by ryan):\n\nno good";
    let result = classify_gate(body).unwrap();
    assert_eq!(result.0, Gate::Rejected);
    assert_eq!(result.1.as_deref(), Some("no good"));
}

#[test]
fn classify_gate_rejection_with_desk_check_fail_is_fail_desk_check() {
    let body = "**rejection_reason** (desk-check-fail by designer):\n\nspec failed";
    let result = classify_gate(body).unwrap();
    assert_eq!(result.0, Gate::FailDeskCheck);
    assert_eq!(result.1.as_deref(), Some("spec failed"));
}

#[test]
fn classify_gate_pass_desk_check_no_payload() {
    let body = "**pass_desk_check** (desk-check-pass by designer)";
    let result = classify_gate(body).unwrap();
    assert_eq!(result.0, Gate::PassDeskCheck);
    assert!(result.1.is_none());
}

#[test]
fn classify_gate_accepted_no_payload() {
    let body = "**accepted** (accept-gate by ryan)";
    let result = classify_gate(body).unwrap();
    assert_eq!(result.0, Gate::Accepted);
    assert!(result.1.is_none());
}

#[test]
fn classify_gate_none_for_unstructured_comment() {
    assert!(classify_gate("just a chat comment").is_none());
}

#[test]
fn decode_gate_comments_filters_unstructured_and_preserves_structured() {
    let comments = vec![
        serde_json::json!({
            "body": "**pickup_paraphrase** (dev pickup-gate):\n\nA",
            "author": "dev",
            "created_at": "2026-04-21T00:00:00Z",
        }),
        serde_json::json!({
            "body": "unstructured aside",
            "author": "pm",
            "created_at": "2026-04-22T00:00:00Z",
        }),
        serde_json::json!({
            "body": "**accepted** (accept-gate by ryan)",
            "author": "ryan",
            "created_at": "2026-04-23T00:00:00Z",
        }),
    ];
    let gates = decode_gate_comments(&comments);
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0].gate, Gate::Pickup);
    assert_eq!(gates[1].gate, Gate::Accepted);
}

#[test]
fn decode_gate_comments_preserves_timestamps_from_source() {
    let comments = vec![serde_json::json!({
        "body": "**verified_observation** (dev deliver-gate):\n\nX",
        "author": "dev",
        "created_at": "2026-04-22T14:30:00+00:00",
    })];
    let gates = decode_gate_comments(&comments);
    assert_eq!(
        gates[0].ts,
        Utc.with_ymd_and_hms(2026, 4, 22, 14, 30, 0).unwrap()
    );
}

#[test]
fn story_type_label_takes_precedence_over_title() {
    let labels = vec!["bug".to_string()];
    assert_eq!(
        story_type_from_labels_or_title(&labels, "[Feature] something"),
        StoryType::Bug
    );
}

#[test]
fn story_type_falls_back_to_title_prefix_when_no_label() {
    assert_eq!(
        story_type_from_labels_or_title(&[], "[Chore] cleanup"),
        StoryType::Chore
    );
    assert_eq!(
        story_type_from_labels_or_title(&[], "[Bug] broken"),
        StoryType::Bug
    );
    assert_eq!(
        story_type_from_labels_or_title(&[], "just a title"),
        StoryType::Feature
    );
}

#[test]
fn story_type_label_matching_is_case_insensitive() {
    let labels = vec!["CHORE".to_string()];
    assert_eq!(
        story_type_from_labels_or_title(&labels, "anything"),
        StoryType::Chore
    );
}

#[test]
fn tags_from_labels_picks_up_blocked() {
    let labels = vec!["ready".to_string(), "blocked".to_string()];
    assert_eq!(tags_from_labels(&labels), vec![Tag::Blocked]);
}

#[test]
fn tags_from_labels_ignores_unknown() {
    let labels = vec!["ready".to_string(), "high-priority".to_string()];
    assert!(tags_from_labels(&labels).is_empty());
}

#[test]
fn tags_from_labels_deduplicates_blocked() {
    let labels = vec!["blocked".to_string(), "Blocked".to_string()];
    assert_eq!(tags_from_labels(&labels), vec![Tag::Blocked]);
}

#[test]
fn decode_story_metadata_populates_story_type_and_tags_from_labels() {
    let v = serde_json::json!({
        "number": 1,
        "title": "some story",
        "state": "open",
        "created_at": "2026-04-20T00:00:00Z",
        "user": {"login": "pm"},
        "labels": [{"name": "bug"}, {"name": "blocked"}],
    });
    let story = decode_story_metadata(&v).unwrap();
    assert_eq!(story.story_type, StoryType::Bug);
    assert_eq!(story.tags, vec![Tag::Blocked]);
}

#[test]
fn is_already_closed_error_recognizes_octocrab_phrasings() {
    assert!(is_already_closed_error("Issue #42 is already closed"));
    assert!(is_already_closed_error(
        "the issue is closed and cannot be edited"
    ));
}

#[test]
fn is_already_closed_error_rejects_unrelated_errors() {
    assert!(!is_already_closed_error("404 Not Found"));
    assert!(!is_already_closed_error("rate limit exceeded"));
    assert!(!is_already_closed_error(""));
}
