#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone, Utc};

use crate::board::adapters::github::is_iteration_position_label_for_test as is_iteration_position_label;
use crate::board::adapters::github::is_position_label_for_test as is_position_label;
use crate::board::adapters::github::{
    classify_gate, decode_gate_comments, decode_story_metadata, iteration_membership_from_labels,
    iteration_order_from_labels, parent_epic_from_labels, state_from_labels, state_to_label,
    tags_from_labels, warn_on_inconsistent_state_labels,
};
use crate::board::domain::story_type_from_labels_or_title;
use crate::board::domain::{Gate, StoryState, StoryType, Tag};
use crate::board::sync::{SyncError, classify_close_error};

#[test]
fn is_iteration_position_label_matches_iteration_theme_dot_number() {
    assert!(is_iteration_position_label("iteration-Cassiopeia.100"));
    assert!(is_iteration_position_label("iteration-Aquila-B.50"));
    assert!(is_iteration_position_label(
        "iteration-My-Long-Theme-Name.999"
    ));
}

#[test]
fn is_iteration_position_label_rejects_iteration_order() {
    assert!(!is_iteration_position_label("iteration-order.100"));
    assert!(!is_iteration_position_label("iteration-order.0"));
}

#[test]
fn is_iteration_position_label_rejects_non_iteration() {
    assert!(!is_iteration_position_label("blocked"));
    assert!(!is_iteration_position_label("ready"));
    assert!(!is_iteration_position_label("parent-epic-#1247"));
    assert!(!is_iteration_position_label("iteration"));
}

#[test]
fn is_iteration_position_label_rejects_iteration_without_position_suffix() {
    assert!(!is_iteration_position_label("iteration-Cassiopeia"));
    assert!(!is_iteration_position_label("iteration-Cassiopeia."));
    assert!(!is_iteration_position_label("iteration-Cassiopeia.abc"));
}

#[test]
fn is_position_label_covers_ready_order_and_iteration_position() {
    assert!(is_position_label("ready-order.100"));
    assert!(is_position_label("ready-order.42"));
    assert!(is_position_label("iteration-Cassiopeia.100"));
    assert!(!is_position_label("ready-order.foo"));
    assert!(!is_position_label("ready"));
    assert!(!is_position_label("blocked"));
}

#[test]
fn state_from_labels_accepted_wins_regardless_of_other_labels() {
    let labels = vec!["ready".to_string(), "accepted".to_string()];
    assert_eq!(state_from_labels(&labels, true), StoryState::Accepted);
    assert_eq!(state_from_labels(&labels, false), StoryState::Accepted);
}

#[test]
fn warn_on_inconsistent_state_labels_does_not_panic() {
    warn_on_inconsistent_state_labels(&[]);
    warn_on_inconsistent_state_labels(&["ready".to_string()]);
    warn_on_inconsistent_state_labels(&["accepted".to_string(), "rejected".to_string()]);
    warn_on_inconsistent_state_labels(&[
        "delivered".to_string(),
        "in progress".to_string(),
        "feature".to_string(),
    ]);
}

#[test]
fn state_from_labels_closed_issue_is_accepted_even_without_label() {
    let labels = vec![];
    assert_eq!(state_from_labels(&labels, true), StoryState::Accepted);
}

#[test]
fn state_from_labels_closed_issue_with_stale_workflow_label_is_accepted() {
    for stale in [
        "in progress",
        "ready",
        "desk-check",
        "delivered",
        "ready for accept",
        "needs fixes",
        "rejected",
        "icebox",
    ] {
        let labels = vec![stale.to_string()];
        assert_eq!(
            state_from_labels(&labels, true),
            StoryState::Accepted,
            "closed issue with stale '{stale}' label should be Accepted"
        );
    }
}

#[test]
fn state_from_labels_retired_label_overrides_closed_fallback() {
    let labels = vec!["retired".to_string()];
    assert_eq!(state_from_labels(&labels, true), StoryState::Retired);
}

#[test]
fn state_from_labels_retired_label_loses_to_accepted() {
    let labels = vec!["accepted".to_string(), "retired".to_string()];
    assert_eq!(state_from_labels(&labels, true), StoryState::Accepted);
}

#[test]
fn state_from_labels_retired_label_open_issue_returns_retired() {
    let labels = vec!["retired".to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::Retired);
}

#[test]
fn state_from_labels_rejected_label_projects_to_ready() {
    let labels = vec!["delivered".to_string(), "rejected".to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::Ready);
}

#[test]
fn state_from_labels_needs_fixes_label_projects_to_in_progress() {
    let labels = vec!["delivered".to_string(), "needs fixes".to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::InProgress);
}

#[test]
fn state_from_labels_ready_for_accept_label_projects_to_delivered() {
    let labels = vec!["delivered".to_string(), "ready for accept".to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::Delivered);
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
fn state_to_label_roundtrips_in_progress_space() {
    let labels = vec![state_to_label(StoryState::InProgress).to_string()];
    assert_eq!(state_from_labels(&labels, false), StoryState::InProgress);
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
    assert!(
        story.body.is_none(),
        "body absent from JSON → body remains None on decoded Story"
    );
}

#[test]
fn decode_story_metadata_decodes_body_when_present() {
    let json = serde_json::json!({
        "number": 1297,
        "title": "Slice 3 — create_iteration inserts positional entry",
        "state": "open",
        "labels": [{"name": "ready"}],
        "created_at": "2026-05-14T23:30:00+00:00",
        "user": {"login": "pm"},
        "body": "## User job\n\nWhen PM/Ryan wants to commit a group of stories to an iteration...\n\n## Acceptance test path\n\napplications/bpmux/src/bridge/tools/create_iteration_tests.rs",
    });
    let story = decode_story_metadata(&json).expect("story");
    assert_eq!(
        story.body.as_deref(),
        Some(
            "## User job\n\nWhen PM/Ryan wants to commit a group of stories to an iteration...\n\n## Acceptance test path\n\napplications/bpmux/src/bridge/tools/create_iteration_tests.rs"
        ),
        "body present in JSON must propagate to decoded Story (closes edit_story body-cascade-drop bug discovered 2026-05-14 23:33 UTC)"
    );
    assert_eq!(
        story.acceptance_test_path.as_deref(),
        Some("applications/bpmux/src/bridge/tools/create_iteration_tests.rs"),
        "acceptance_test_path must be parsed from body when body decoded"
    );
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
fn decode_story_metadata_sets_is_closed_from_gh_state() {
    let open = serde_json::json!({
        "number": 10,
        "title": "open",
        "state": "open",
        "labels": [],
        "created_at": "2026-04-20T00:00:00Z",
        "user": {"login": "pm"},
    });
    let closed = serde_json::json!({
        "number": 11,
        "title": "closed-non-accepted",
        "state": "closed",
        "labels": [],
        "created_at": "2026-04-20T00:00:00Z",
        "user": {"login": "pm"},
    });
    assert!(!decode_story_metadata(&open).unwrap().is_closed);
    assert!(decode_story_metadata(&closed).unwrap().is_closed);
}

#[test]
fn decode_story_metadata_closed_without_retired_collapses_to_accepted() {
    let json = serde_json::json!({
        "number": 99,
        "title": "close_story-fired on a Delivered story; no accepted label present",
        "state": "accepted",
        "gh_state": "closed",
        "labels": ["ready for accept", "delivered"],
        "created_at": "2026-04-20T00:00:00Z",
        "user": {"login": "ryan"},
    });
    let story = decode_story_metadata(&json).unwrap();
    assert!(
        story.is_closed,
        "gh_state=closed still drives is_closed=true regardless of label priority"
    );
    assert_eq!(
        story.state,
        StoryState::Accepted,
        "closed without retired tag is Accepted regardless of stale workflow labels (#1296)"
    );
}

#[test]
fn decode_story_metadata_falls_back_to_state_field_when_gh_state_absent() {
    let json = serde_json::json!({
        "number": 100,
        "title": "raw GH shape, no gh_state field present (legacy / test-fixture envelope)",
        "state": "closed",
        "labels": [],
        "created_at": "2026-04-20T00:00:00Z",
        "user": {"login": "pm"},
    });
    let story = decode_story_metadata(&json).unwrap();
    assert!(
        story.is_closed,
        "decode must fall back to state field for envelopes without gh_state (raw octocrab JSON shape)"
    );
}

#[test]
fn decode_story_metadata_gh_state_open_does_not_force_closed() {
    let json = serde_json::json!({
        "number": 101,
        "title": "open issue with renderer-state delivered",
        "state": "delivered",
        "gh_state": "open",
        "labels": ["delivered"],
        "created_at": "2026-04-20T00:00:00Z",
        "user": {"login": "dev"},
    });
    let story = decode_story_metadata(&json).unwrap();
    assert!(
        !story.is_closed,
        "gh_state=open must drive is_closed=false; renderer-state delivered must NOT trigger is_closed"
    );
    assert_eq!(
        story.state,
        StoryState::Delivered,
        "state_from_labels with is_closed=false should decode delivered label as Delivered"
    );
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
fn story_type_decodes_epic_from_title_prefix() {
    assert_eq!(
        story_type_from_labels_or_title(&[], "[Epic] foo"),
        StoryType::Epic
    );
    assert_eq!(
        story_type_from_labels_or_title(&[], "[epic] lowercase"),
        StoryType::Epic
    );
}

#[test]
fn story_type_decodes_epic_from_label() {
    let labels = vec!["epic".to_string()];
    assert_eq!(
        story_type_from_labels_or_title(&labels, "[Feature] mismatched"),
        StoryType::Epic
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
fn classify_close_error_recognizes_octocrab_phrasings() {
    assert!(matches!(
        classify_close_error("Issue #42 is already closed".to_string()),
        SyncError::AlreadyClosed
    ));
    assert!(matches!(
        classify_close_error("the issue is closed and cannot be edited".to_string()),
        SyncError::AlreadyClosed
    ));
}

#[test]
fn classify_close_error_wraps_unrelated_errors_as_other() {
    assert!(matches!(
        classify_close_error("404 Not Found".to_string()),
        SyncError::Other(_)
    ));
    assert!(matches!(
        classify_close_error("rate limit exceeded".to_string()),
        SyncError::Other(_)
    ));
    assert!(matches!(
        classify_close_error(String::new()),
        SyncError::Other(_)
    ));
}

#[test]
fn parent_epic_from_labels_extracts_number_from_well_formed_label() {
    let labels = vec!["ready".to_string(), "parent-epic-#457".to_string()];
    assert_eq!(parent_epic_from_labels(&labels), Some(457));
}

#[test]
fn parent_epic_from_labels_returns_none_when_label_absent() {
    let labels = vec!["ready".to_string(), "delivered".to_string()];
    assert_eq!(parent_epic_from_labels(&labels), None);
}

#[test]
fn parent_epic_from_labels_returns_none_when_number_missing() {
    let labels = vec!["parent-epic-#".to_string()];
    assert_eq!(parent_epic_from_labels(&labels), None);
}

#[test]
fn parent_epic_from_labels_returns_none_when_number_unparseable() {
    let labels = vec!["parent-epic-#abc".to_string()];
    assert_eq!(parent_epic_from_labels(&labels), None);
}

#[test]
fn parent_epic_from_labels_picks_first_well_formed_match() {
    let labels = vec![
        "parent-epic-#abc".to_string(),
        "parent-epic-#100".to_string(),
        "parent-epic-#200".to_string(),
    ];
    assert_eq!(parent_epic_from_labels(&labels), Some(100));
}

#[test]
fn iteration_membership_from_labels_extracts_number_and_position_1399() {
    let labels = vec![
        "ready".to_string(),
        "parent-iteration-#1397.100".to_string(),
    ];
    assert_eq!(iteration_membership_from_labels(&labels), Some((1397, 100)));
}

#[test]
fn iteration_membership_from_labels_returns_none_when_label_absent_1399() {
    let labels = vec!["ready".to_string(), "icebox".to_string()];
    assert_eq!(iteration_membership_from_labels(&labels), None);
}

#[test]
fn iteration_membership_from_labels_returns_none_when_position_missing_1399() {
    let labels = vec!["parent-iteration-#1397".to_string()];
    assert_eq!(iteration_membership_from_labels(&labels), None);
}

#[test]
fn iteration_membership_from_labels_returns_none_when_position_unparseable_1399() {
    let labels = vec!["parent-iteration-#1397.abc".to_string()];
    assert_eq!(iteration_membership_from_labels(&labels), None);
}

#[test]
fn iteration_membership_from_labels_returns_none_when_iteration_number_unparseable_1399() {
    let labels = vec!["parent-iteration-#abc.100".to_string()];
    assert_eq!(iteration_membership_from_labels(&labels), None);
}

#[test]
fn iteration_membership_from_labels_handles_large_position_and_iteration_number_1399() {
    let labels = vec!["parent-iteration-#999999999.4294967295".to_string()];
    assert_eq!(
        iteration_membership_from_labels(&labels),
        Some((999_999_999, u32::MAX))
    );
}

#[test]
fn iteration_membership_from_labels_picks_first_well_formed_match_1399() {
    let labels = vec![
        "parent-iteration-#1397.bad".to_string(),
        "parent-iteration-#1397.100".to_string(),
        "parent-iteration-#1398.200".to_string(),
    ];
    assert_eq!(iteration_membership_from_labels(&labels), Some((1397, 100)));
}

#[test]
fn iteration_order_from_labels_extracts_position_1735() {
    let labels = vec!["iteration".to_string(), "iteration-order.300".to_string()];
    assert_eq!(iteration_order_from_labels(&labels), Some(300));
}

#[test]
fn iteration_order_from_labels_returns_none_when_absent_1735() {
    let labels = vec!["iteration-Octans.3".to_string(), "ready".to_string()];
    assert_eq!(iteration_order_from_labels(&labels), None);
}

#[test]
fn iteration_order_from_labels_picks_lowest_when_multiple_1735() {
    let labels = vec![
        "iteration-order.300".to_string(),
        "iteration-order.100".to_string(),
        "iteration-order.500".to_string(),
    ];
    assert_eq!(iteration_order_from_labels(&labels), Some(100));
}

#[test]
fn decode_story_metadata_sets_iteration_order_from_label_1735() {
    let v = serde_json::json!({
        "number": 1397,
        "title": "Iteration Fornax",
        "labels": [{"name": "iteration-Fornax.100"}, {"name": "iteration-order.200"}],
    });
    let story = decode_story_metadata(&v).expect("decodes");
    assert_eq!(story.iteration_order, Some(200));
}

#[test]
fn decode_story_metadata_populates_parent_epic_from_label() {
    let json = serde_json::json!({
        "number": 1000,
        "title": "child story",
        "state": "open",
        "labels": [{"name": "ready"}, {"name": "parent-epic-#457"}],
        "created_at": "2026-04-25T00:00:00Z",
        "user": {"login": "pm"},
    });
    let story = decode_story_metadata(&json).unwrap();
    assert_eq!(story.parent_epic, Some(457));
}

#[test]
fn decode_story_metadata_parent_epic_none_when_label_absent() {
    let json = serde_json::json!({
        "number": 1001,
        "title": "regular story",
        "state": "open",
        "labels": [{"name": "ready"}],
        "created_at": "2026-04-25T00:00:00Z",
        "user": {"login": "pm"},
    });
    let story = decode_story_metadata(&json).unwrap();
    assert_eq!(story.parent_epic, None);
}

#[test]
fn decode_story_metadata_populates_raw_labels_for_reap_pre_filter() {
    let json = serde_json::json!({
        "number": 1156,
        "title": "[Bug] reap-pre-filter substrate",
        "state": "open",
        "labels": [
            {"name": "ready"},
            {"name": "Feature"},
            {"name": "blocked"},
        ],
        "created_at": "2026-05-03T00:00:00Z",
        "user": {"login": "pm"},
    });
    let story = decode_story_metadata(&json).unwrap();
    assert_eq!(
        story.raw_labels,
        vec![
            "ready".to_string(),
            "Feature".to_string(),
            "blocked".to_string()
        ],
        "raw_labels must mirror GH labels list verbatim — this is what reap_workflow_labels pre-filters against; if decode drops the field, every gate-fire silently no-ops"
    );
}

#[test]
fn decode_story_metadata_raw_labels_empty_when_gh_returns_no_labels() {
    let json = serde_json::json!({
        "number": 2002,
        "title": "story with no labels",
        "state": "open",
        "labels": [],
        "created_at": "2026-05-03T00:00:00Z",
        "user": {"login": "pm"},
    });
    let story = decode_story_metadata(&json).unwrap();
    assert!(
        story.raw_labels.is_empty(),
        "raw_labels empty when GH returns []; pre-filter sees []; reap is no-op (correct since no labels to strip)"
    );
}
