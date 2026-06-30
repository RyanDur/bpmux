#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::view_model::*;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::iteration::IterationId;
use crate::product_management::outcome::OutcomeId;
use crate::product_management::story::{Story, StoryId, StoryState};
use chrono::{TimeZone, Utc};

fn story(id: u64, title: &str, state: StoryState) -> Story {
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
    let s = story(42, "[Feature] do the thing", StoryState::Ready);
    let view = StoryView::project(&GhLabelCodec, &s);
    assert_eq!(view.id, 42);
    assert_eq!(view.title, "[Feature] do the thing");
    assert_eq!(view.state, "ready");
    assert_eq!(view.gh_state, "open");
}

#[test]
fn project_marks_a_closed_story_gh_state_closed() {
    let mut s = story(7, "done", StoryState::Accepted);
    s.is_closed = true;
    let view = StoryView::project(&GhLabelCodec, &s);
    assert_eq!(view.gh_state, "closed");
}

#[test]
fn project_translates_iteration_membership_into_a_typed_subview() {
    let mut s = story(10, "member", StoryState::Ready);
    s.raw_labels.push("parent-iteration-#200.150".to_string());
    s.iteration_membership = Some((IterationId::new(200), 150));
    let view = StoryView::project(&GhLabelCodec, &s);
    let m = view
        .iteration_membership
        .expect("iteration membership present");
    assert_eq!(m.iteration_id, IterationId::new(200));
    assert_eq!(m.position, 150);
}

#[test]
fn project_translates_outcome_membership_into_a_typed_subview() {
    let mut s = story(11, "mapped", StoryState::Ready);
    s.raw_labels.push("parent-outcome-#700.2".to_string());
    let view = StoryView::project(&GhLabelCodec, &s);
    let m = view.outcome_membership.expect("outcome membership present");
    assert_eq!(m.outcome_id, OutcomeId::new(700));
    assert_eq!(m.position, 2);
}

#[test]
fn project_leads_the_wire_labels_with_the_derived_type_label() {
    let s = story(20, "[Epic] umbrella", StoryState::Icebox);
    let view = StoryView::project(&GhLabelCodec, &s);
    assert_eq!(
        view.labels.first().map(String::as_str),
        Some("epic"),
        "the type label leads the wire labels (derived from title when no label present)"
    );
}

#[test]
fn story_view_serializes_to_the_renderer_wire_shape() {
    let s = story(5, "wire", StoryState::Ready);
    let view = StoryView::project(&GhLabelCodec, &s);
    let json = serde_json::to_value(&view).expect("StoryView serializes");
    assert_eq!(json["id"], 5);
    assert_eq!(json["state"], "ready");
    assert_eq!(json["gh_state"], "open");
    assert!(json.get("labels").is_some(), "labels key present");
    assert!(
        json.get("iteration_membership").is_some(),
        "iteration_membership key present (null when absent)"
    );
    assert_eq!(json["assignees"], serde_json::json!([]));
    assert_eq!(
        json["rework_comment"],
        serde_json::Value::Null,
        "absent rework comment serializes as null, preserving the wire contract"
    );
}
