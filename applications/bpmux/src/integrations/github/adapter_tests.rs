#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone, Utc};

use crate::integrations::github::mapper::is_iteration_position_label_for_test as is_iteration_position_label;
use crate::integrations::github::mapper::is_position_label_for_test as is_position_label;
use crate::integrations::github::mapper::{
    classify_verdict, decode_story_metadata, decode_verdicts, is_epic_issue, is_epic_story,
    iteration_order_from_labels, outbound_state_label, parent_epic_from_labels, state_from_labels,
    story_type_from_labels, story_type_from_labels_or_title, tags_from_labels,
    warn_on_inconsistent_state_labels,
};
use crate::product_management::epic::EpicId;
use crate::product_management::story::{StoryId, StoryKind, Tag, Verdict, Work};
use crate::sync::{SyncError, classify_close_error};

#[test]
fn is_iteration_position_label_matches_iteration_name_dot_number() {
    assert!(is_iteration_position_label("iteration-Cassiopeia.100"));
    assert!(is_iteration_position_label("iteration-Aquila-B.50"));
    assert!(is_iteration_position_label(
        "iteration-My-Long-Name-Name.999"
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
    assert_eq!(state_from_labels(&labels, true), Work::Accepted);
    assert_eq!(state_from_labels(&labels, false), Work::Accepted);
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
    assert_eq!(state_from_labels(&labels, true), Work::Accepted);
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
            Work::Accepted,
            "closed issue with stale '{stale}' label should be Accepted"
        );
    }
}

#[test]
fn state_from_labels_retired_label_overrides_closed_fallback() {
    let labels = vec!["retired".to_string()];
    assert_eq!(state_from_labels(&labels, true), Work::Retired);
}

#[test]
fn state_from_labels_retired_label_loses_to_accepted() {
    let labels = vec!["accepted".to_string(), "retired".to_string()];
    assert_eq!(state_from_labels(&labels, true), Work::Accepted);
}

#[test]
fn state_from_labels_retired_label_open_issue_returns_retired() {
    let labels = vec!["retired".to_string()];
    assert_eq!(state_from_labels(&labels, false), Work::Retired);
}

#[test]
fn state_from_labels_rejected_label_decodes_to_not_started_rework() {
    let labels = vec!["delivered".to_string(), "rejected".to_string()];
    assert_eq!(state_from_labels(&labels, false), Work::NotStarted);
}

#[test]
fn state_from_labels_needs_fixes_label_projects_to_in_progress() {
    let labels = vec!["delivered".to_string(), "needs fixes".to_string()];
    assert_eq!(state_from_labels(&labels, false), Work::InProgress);
}

#[test]
fn state_from_labels_ready_for_accept_label_projects_to_delivered() {
    let labels = vec!["delivered".to_string(), "ready for accept".to_string()];
    assert_eq!(state_from_labels(&labels, false), Work::Acceptance);
}

#[test]
fn state_from_labels_delivered_without_accept_labels() {
    let labels = vec!["delivered".to_string()];
    assert_eq!(state_from_labels(&labels, false), Work::Acceptance);
}

#[test]
fn state_from_labels_in_progress_resolves_alongside_a_stale_ready_label() {
    let labels = vec!["ready".to_string(), "in progress".to_string()];
    assert_eq!(state_from_labels(&labels, false), Work::InProgress);
}

#[test]
fn state_from_labels_ready_label_is_not_a_scheduling_input_open_story_decodes_not_started() {
    let labels = vec!["ready".to_string()];
    assert_eq!(
        state_from_labels(&labels, false),
        Work::NotStarted,
        "the ready label stops resolving scheduling — a ready-labeled open story decodes to the stored fallback; current-iteration membership is the scheduling truth"
    );
}

#[test]
fn state_from_labels_not_started_is_the_open_default() {
    let labels = vec!["Feature".to_string()];
    assert_eq!(state_from_labels(&labels, false), Work::NotStarted);
}

#[test]
fn state_from_labels_is_case_insensitive() {
    let labels = vec!["DELIVERED".to_string()];
    assert_eq!(state_from_labels(&labels, false), Work::Acceptance);
}

#[test]
fn the_derived_ready_label_is_outbound_only_the_decoder_ignores_it() {
    let ready_label = outbound_state_label(&Work::NotStarted, true)
        .expect("a not-started current-iteration member derives the ready mirror label");
    assert_eq!(
        state_from_labels(&[ready_label.to_string()], false),
        Work::NotStarted,
        "the outbound ready label is a human-readable mirror, never a decode input — scheduling decodes from iteration membership"
    );
}

#[test]
fn the_in_progress_github_label_round_trips_with_its_space_through_decode() {
    let labels = vec![Work::InProgress.github_label().to_string()];
    assert_eq!(state_from_labels(&labels, false), Work::InProgress);
}

#[test]
fn decode_story_metadata_reads_object_label_shape() {
    let json = serde_json::json!({
        "number": 451,
        "title": "Internal board abstraction",
        "state": "open",
        "labels": [{"name": "in progress"}],
        "created_at": "2026-04-24T13:22:36+00:00",
        "user": {"login": "owner"},
    });
    let story = decode_story_metadata(&json).expect("story");
    assert_eq!(story.id, StoryId::new(451));
    assert_eq!(story.title, "Internal board abstraction");
    assert_eq!(story.state, Work::InProgress);
    assert_eq!(story.filed_by, "owner");
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
        "body": "## User job\n\nWhen PM/owner wants to commit a group of stories to an iteration...\n\n## Acceptance test path\n\napplications/bpmux/src/bridge/tools/create_iteration_tests.rs",
    });
    let story = decode_story_metadata(&json).expect("story");
    assert_eq!(
        story.body.as_deref(),
        Some(
            "## User job\n\nWhen PM/owner wants to commit a group of stories to an iteration...\n\n## Acceptance test path\n\napplications/bpmux/src/bridge/tools/create_iteration_tests.rs"
        ),
        "body present in JSON must propagate to decoded Story (closes edit_story body-cascade-drop bug discovered 2026-05-14 23:33 UTC)"
    );
}

#[test]
fn decode_story_metadata_reads_string_label_shape() {
    let json = serde_json::json!({
        "number": 1,
        "title": "t",
        "state": "open",
        "labels": ["retired"],
        "created_at": "2026-04-20T00:00:00Z",
        "user": {"login": "pm"},
    });
    let story = decode_story_metadata(&json).unwrap();
    assert_eq!(story.state, Work::Retired);
}

#[test]
fn decode_story_metadata_returns_none_when_number_missing() {
    let json = serde_json::json!({"title": "no number"});
    assert!(decode_story_metadata(&json).is_none());
}

#[test]
fn decode_story_metadata_forces_terminal_state_from_gh_state() {
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
    assert_eq!(
        decode_story_metadata(&open).unwrap().state,
        Work::NotStarted,
        "an open issue with no state labels decodes to a non-terminal state"
    );
    assert_eq!(
        decode_story_metadata(&closed).unwrap().state,
        Work::Accepted,
        "a closed issue forces the terminal Accepted state at decode"
    );
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
        "user": {"login": "owner"},
    });
    let story = decode_story_metadata(&json).unwrap();
    assert_eq!(
        story.state,
        Work::Accepted,
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
    assert_eq!(
        story.state,
        Work::Accepted,
        "decode must fall back to the state field for envelopes without gh_state (raw octocrab JSON shape): closed forces Accepted"
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
    assert_eq!(
        story.state,
        Work::Acceptance,
        "state_from_labels with is_closed=false should decode delivered label as Delivered"
    );
}

#[test]
fn classify_verdict_pickup() {
    let body = "**pickup_paraphrase** (dev pickup-gate):\n\nscope statement";
    let result = classify_verdict(body).unwrap();
    assert_eq!(
        result.0,
        Verdict::Succeed {
            step: "ready".into()
        }
    );
    assert_eq!(result.1.as_deref(), Some("scope statement"));
}

#[test]
fn classify_verdict_delivered_payload_preserved() {
    let body = "**verified_observation** (dev deliver-gate):\n\nran end-to-end";
    let result = classify_verdict(body).unwrap();
    assert_eq!(
        result.0,
        Verdict::Succeed {
            step: "in-progress".into()
        }
    );
    assert_eq!(result.1.as_deref(), Some("ran end-to-end"));
}

#[test]
fn classify_verdict_rejection_default_is_rejected() {
    let body = "**rejection_reason** (reject-gate by owner):\n\nno good";
    let result = classify_verdict(body).unwrap();
    assert_eq!(
        result.0,
        Verdict::Fail {
            step: "acceptance".into()
        }
    );
    assert_eq!(result.1.as_deref(), Some("no good"));
}

#[test]
fn classify_verdict_rejection_with_desk_check_fail_is_fail_desk_check() {
    let body = "**rejection_reason** (desk-check-fail by designer):\n\nspec failed";
    let result = classify_verdict(body).unwrap();
    assert_eq!(
        result.0,
        Verdict::Fail {
            step: "desk-check".into()
        }
    );
    assert_eq!(result.1.as_deref(), Some("spec failed"));
}

#[test]
fn classify_verdict_pass_desk_check_no_payload() {
    let body = "**pass_desk_check** (desk-check-pass by designer)";
    let result = classify_verdict(body).unwrap();
    assert_eq!(
        result.0,
        Verdict::Succeed {
            step: "desk-check".into()
        }
    );
    assert!(result.1.is_none());
}

#[test]
fn classify_verdict_accepted_no_payload() {
    let body = "**accepted** (accept-gate by owner)";
    let result = classify_verdict(body).unwrap();
    assert_eq!(
        result.0,
        Verdict::Succeed {
            step: "acceptance".into()
        }
    );
    assert!(result.1.is_none());
}

#[test]
fn classify_verdict_none_for_unstructured_comment() {
    assert!(classify_verdict("just a chat comment").is_none());
}

#[test]
fn decode_verdicts_filters_unstructured_and_preserves_structured() {
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
            "body": "**accepted** (accept-gate by owner)",
            "author": "owner",
            "created_at": "2026-04-23T00:00:00Z",
        }),
    ];
    let gates = decode_verdicts(&comments);
    assert_eq!(gates.len(), 2);
    assert_eq!(
        gates[0].verdict,
        Verdict::Succeed {
            step: "ready".into()
        }
    );
    assert_eq!(
        gates[1].verdict,
        Verdict::Succeed {
            step: "acceptance".into()
        }
    );
}

#[test]
fn decode_verdicts_preserves_timestamps_from_source() {
    let comments = vec![serde_json::json!({
        "body": "**verified_observation** (dev deliver-gate):\n\nX",
        "author": "dev",
        "created_at": "2026-04-22T14:30:00+00:00",
    })];
    let gates = decode_verdicts(&comments);
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
        StoryKind::Bug
    );
}

#[test]
fn story_type_falls_back_to_title_prefix_when_no_label() {
    assert_eq!(
        story_type_from_labels_or_title(&[], "[Chore] cleanup"),
        StoryKind::Chore
    );
    assert_eq!(
        story_type_from_labels_or_title(&[], "[Bug] broken"),
        StoryKind::Bug
    );
    assert_eq!(
        story_type_from_labels_or_title(&[], "just a title"),
        StoryKind::Feature
    );
}

#[test]
fn story_type_label_matching_is_case_insensitive() {
    let labels = vec!["CHORE".to_string()];
    assert_eq!(
        story_type_from_labels_or_title(&labels, "anything"),
        StoryKind::Chore
    );
}

#[test]
fn epic_recognized_from_title_prefix() {
    let upper = decode_story_metadata(&serde_json::json!({
        "number": 1,
        "title": "[Epic] foo",
        "gh_state": "open",
        "labels": [],
        "created_at": "2026-05-15T01:00:00+00:00",
        "user": { "login": "pm" },
    }))
    .expect("issue JSON must decode");
    assert!(is_epic_story(&upper, &[]));
    assert!(is_epic_issue(
        &serde_json::json!({ "title": "[epic] lowercase" })
    ));
}

#[test]
fn epic_recognized_from_label() {
    let story = decode_story_metadata(&serde_json::json!({
        "number": 2,
        "title": "[Feature] mismatched",
        "gh_state": "open",
        "labels": [{ "name": "epic" }],
        "created_at": "2026-05-15T01:00:00+00:00",
        "user": { "login": "pm" },
    }))
    .expect("issue JSON must decode");
    assert!(is_epic_story(&story, &["epic".to_string()]));
}

#[test]
fn story_type_from_labels_is_some_only_for_a_type_label() {
    assert_eq!(
        story_type_from_labels(&["CHORE".to_string()]),
        Some(StoryKind::Chore)
    );
    assert_eq!(story_type_from_labels(&["ready".to_string()]), None);
    assert_eq!(story_type_from_labels(&[]), None);
}

#[test]
fn tags_from_labels_picks_up_desk_check_fail() {
    let labels = vec!["ready".to_string(), "desk-check-fail".to_string()];
    assert_eq!(tags_from_labels(&labels), vec![Tag::DeskCheckFail]);
}

#[test]
fn tags_from_labels_ignores_unknown() {
    let labels = vec!["ready".to_string(), "high-priority".to_string()];
    assert!(tags_from_labels(&labels).is_empty());
}

#[test]
fn tags_from_labels_deduplicates_desk_check_fail() {
    let labels = vec!["desk-check-fail".to_string(), "Desk-Check-Fail".to_string()];
    assert_eq!(tags_from_labels(&labels), vec![Tag::DeskCheckFail]);
}

#[test]
fn decode_story_metadata_populates_story_type_and_tags_from_labels() {
    let v = serde_json::json!({
        "number": 1,
        "title": "some story",
        "state": "open",
        "created_at": "2026-04-20T00:00:00Z",
        "user": {"login": "pm"},
        "labels": [{"name": "bug"}, {"name": "desk-check-fail"}],
    });
    let story = decode_story_metadata(&v).unwrap();
    assert_eq!(story.story_type, StoryKind::Bug);
    assert_eq!(story.tags, vec![Tag::DeskCheckFail]);
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
fn decode_iteration_from_story_sets_iteration_order_from_label_1735() {
    let v = serde_json::json!({
        "number": 1397,
        "title": "Iteration Fornax",
        "labels": [{"name": "iteration-Fornax.100"}, {"name": "iteration-order.200"}],
    });
    let story = decode_story_metadata(&v).expect("decodes");
    let labels = vec![
        "iteration-Fornax.100".to_string(),
        "iteration-order.200".to_string(),
    ];
    let iteration =
        crate::integrations::github::mapper::decode_iteration_from_story(&story, &labels, false);
    assert_eq!(iteration.iteration_order, Some(200));
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
    assert_eq!(story.parent_epic, Some(EpicId::new(457)));
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
