#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::product_management::story::{Story, StoryId, Work};
use chrono::{TimeZone, Utc};

fn story(id: u64, title: &str, state: Work) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

#[test]
fn project_mirrors_core_identity_fields() {
    let s = story(42, "[Feature] do the thing", Work::NotStarted);
    let view = StoryView::project(&GhLabelCodec, &s, "ready");
    assert_eq!(view.id, 42);
    assert_eq!(view.title, "[Feature] do the thing");
    assert_eq!(view.state, "ready");
    assert_eq!(view.gh_state, "open");
}

#[test]
fn project_derives_gh_state_closed_from_workflow_conclusion() {
    let s = story(7, "done", Work::Accepted);
    let view = StoryView::project(&GhLabelCodec, &s, s.state.as_str());
    assert_eq!(
        view.gh_state, "closed",
        "gh_state derives from the workflow conclusion (Accepted|Retired), not from a stored GH mirror"
    );
}

#[test]
fn project_encodes_badge_labels_from_typed_fields() {
    let mut s = story(11, "badged", Work::Acceptance);
    s.tags = vec![crate::product_management::story::Tag::DeskCheckFail];
    let view = StoryView::project(&GhLabelCodec, &s, s.state.as_str());
    assert!(
        view.labels.iter().any(|l| l == "desk-check-fail"),
        "the rework badge label re-encodes from the typed DeskCheckFail tag; got {:?}",
        view.labels
    );
}

#[test]
fn project_leads_the_wire_labels_with_the_derived_type_label() {
    let s = story(20, "[Bug] broken thing", Work::NotStarted);
    let view = StoryView::project(&GhLabelCodec, &s, s.state.as_str());
    assert_eq!(
        view.labels.first().map(String::as_str),
        Some("bug"),
        "the type label leads the wire labels (derived from title when no label present)"
    );
}

#[test]
fn story_view_serializes_to_the_renderer_wire_shape() {
    let s = story(5, "wire", Work::NotStarted);
    let view = StoryView::project(&GhLabelCodec, &s, "ready");
    let json = serde_json::to_value(&view).expect("StoryView serializes");
    assert_eq!(json["id"], 5);
    assert_eq!(json["state"], "ready");
    assert_eq!(json["gh_state"], "open");
    assert!(json.get("labels").is_some(), "labels key present");
    assert!(
        json.get("iteration_membership").is_none(),
        "iteration_membership is gone from the wire — membership is the iteration container's concern (iteration.stories)"
    );
    assert_eq!(json["assignees"], serde_json::json!([]));
    assert_eq!(
        json["rework_comment"],
        serde_json::Value::Null,
        "absent rework comment serializes as null, preserving the wire contract"
    );
}
