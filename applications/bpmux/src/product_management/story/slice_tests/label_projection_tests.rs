use super::{apply, created, meta, minimal, seed, seeded, story_at};
use crate::product_management::board::adapters::github::{
    acceptor_from_labels, decode_iteration_from_story, parent_epic_from_labels,
    story_type_from_labels_or_title, tags_from_labels,
};
use crate::product_management::board::events::{LabelDerivedFields, StoryEvent};
use crate::product_management::epic::EpicId;
use crate::product_management::story::{Acceptor, BoardState, Stage, StoryId, StoryKind, Tag};
use crate::store::SeededStore;

#[test]
fn tag_added_awaiting_label_rederives_acceptor_then_removal_clears_it_1551() {
    let store = seeded();
    created(&store, 1, "t", BoardState::Delivered);
    apply(
        &store,
        StoryEvent::TagAdded {
            meta: meta("designer", 2026, "local"),
            id: StoryId::new(1),
            label: "awaiting team".to_string(),
        },
    );
    assert_eq!(
        acceptor_from_labels(&story_at(&store, 1).unwrap().raw_labels),
        Some(Acceptor::Team),
        "the awaiting-label add re-derives the typed acceptor (closes the view refresh-gap)"
    );
    apply(
        &store,
        StoryEvent::TagRemoved {
            meta: meta("designer", 2026, "local"),
            id: StoryId::new(1),
            label: "awaiting team".to_string(),
        },
    );
    assert_eq!(
        acceptor_from_labels(&story_at(&store, 1).unwrap().raw_labels),
        None,
        "removing the awaiting-label clears the acceptor — honest-neutral, not stale"
    );
}

fn iteration_at(store: &SeededStore, id: u64) -> crate::product_management::iteration::Iteration {
    decode_iteration_from_story(&story_at(store, id).expect("iteration issue present in the store"))
}

fn seed_iteration_issue(store: &SeededStore, id: u64) {
    apply(
        store,
        StoryEvent::StoryCreated {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(id),
            title: "Iteration Vela".to_string(),
            body: "b".to_string(),
            initial_state: BoardState::Ready,
            story_type: StoryKind::default(),
        },
    );
    apply(
        store,
        StoryEvent::TagAdded {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(id),
            label: "iteration".to_string(),
        },
    );
}

#[test]
fn tag_added_review_close_label_rederives_closers_then_removal_clears_them_1703() {
    let store = seeded();
    seed_iteration_issue(&store, 1);
    apply(
        &store,
        StoryEvent::TagAdded {
            meta: meta("designer", 2026, "local"),
            id: StoryId::new(1),
            label: "iteration-review-closed.designer".to_string(),
        },
    );
    assert_eq!(
        iteration_at(&store, 1).review_closers,
        vec!["designer".to_string()],
        "the review-close label add re-derives the closer-set from the recorded act"
    );
    apply(
        &store,
        StoryEvent::TagRemoved {
            meta: meta("designer", 2026, "local"),
            id: StoryId::new(1),
            label: "iteration-review-closed.designer".to_string(),
        },
    );
    assert!(
        iteration_at(&store, 1).review_closers.is_empty(),
        "removing the review-close label clears the closer — derived, never stale"
    );
}

#[test]
fn tag_added_dev_ok_label_rederives_approvers_then_removal_clears_them_1736() {
    let store = seeded();
    seed_iteration_issue(&store, 1);
    apply(
        &store,
        StoryEvent::TagAdded {
            meta: meta("dev", 2026, "local"),
            id: StoryId::new(1),
            label: "iteration-dev-ok.dev".to_string(),
        },
    );
    assert_eq!(
        iteration_at(&store, 1).dev_ok_by,
        vec!["dev".to_string()],
        "the dev-OK label add re-derives the approver-set from the recorded act"
    );
    apply(
        &store,
        StoryEvent::TagRemoved {
            meta: meta("dev", 2026, "local"),
            id: StoryId::new(1),
            label: "iteration-dev-ok.dev".to_string(),
        },
    );
    assert!(
        iteration_at(&store, 1).dev_ok_by.is_empty(),
        "removing the dev-OK label clears the approver — derived, never stale"
    );
}

#[test]
fn tag_added_unknown_label_is_dropped() {
    let store = seeded();
    created(&store, 1, "t", BoardState::Ready);
    apply(
        &store,
        StoryEvent::TagAdded {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(1),
            label: "high-priority".to_string(),
        },
    );
    assert!(tags_from_labels(&story_at(&store, 1).unwrap().raw_labels).is_empty());
}

#[test]
fn generic_label_churn_does_not_clobber_the_typed_tags_field() {
    let store = seeded();
    created(&store, 1, "t", BoardState::Ready);
    apply(
        &store,
        StoryEvent::TagSet {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(1),
            tag: Tag::Blocked,
        },
    );
    apply(
        &store,
        StoryEvent::TagAdded {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(1),
            label: "high-priority".to_string(),
        },
    );
    apply(
        &store,
        StoryEvent::TagRemoved {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(1),
            label: "high-priority".to_string(),
        },
    );
    assert_eq!(
        story_at(&store, 1).unwrap().tags(),
        [Tag::Blocked],
        "a generic raw-label add/remove must not re-project (clobber) the typed tags field"
    );
}

#[test]
fn story_created_event_carries_story_type() {
    let store = seeded();
    apply(
        &store,
        StoryEvent::StoryCreated {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(1),
            title: "t".to_string(),
            body: "b".to_string(),
            initial_state: BoardState::Ready,
            story_type: StoryKind::Bug,
        },
    );
    apply(
        &store,
        StoryEvent::TagAdded {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(1),
            label: "bug".to_string(),
        },
    );
    assert_eq!(
        story_type_from_labels_or_title(
            &story_at(&store, 1).unwrap().raw_labels,
            &story_at(&store, 1).unwrap().title
        ),
        StoryKind::Bug
    );
}

#[test]
fn labels_refreshed_assigns_the_ready_order_field_from_adapter_decoded_fields() {
    let store = seeded();
    created(&store, 18, "orderable", BoardState::Ready);
    assert_eq!(
        story_at(&store, 18).unwrap().ready_order(),
        None,
        "a freshly created story has no ready-order yet"
    );
    apply(
        &store,
        StoryEvent::LabelsRefreshed {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(18),
            fields: LabelDerivedFields {
                raw_labels: vec!["ready-order.3".to_string()],
                ready_order: Some(3),
                ..Default::default()
            },
        },
    );
    assert_eq!(
        story_at(&store, 18).unwrap().ready_order(),
        Some(3),
        "the adapter-decoded ready-order lands in the typed field via the LabelsRefreshed resync; apply_raw_labels no longer re-derives it from raw_labels"
    );
}

#[test]
fn labels_refreshed_purely_assigns_typed_fields_and_leaves_state_to_state_transitioned() {
    let store = seeded();
    seed(
        &store,
        vec![minimal(77, "membered", BoardState::InProgress(Stage::Dev))],
    );
    apply(
        &store,
        StoryEvent::LabelsRefreshed {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(77),
            fields: LabelDerivedFields {
                raw_labels: vec![
                    "parent-epic-#7".to_string(),
                    "parent-iteration-#1655.106".to_string(),
                    "awaiting dev".to_string(),
                ],
                acceptor: Some(Acceptor::Dev),
                parent_epic: Some(EpicId::new(7)),
                tags: Vec::new(),
                blocked_by: None,
                ready_order: None,
            },
        },
    );
    let s = story_at(&store, 77).expect("story exists");
    assert_eq!(parent_epic_from_labels(&s.raw_labels), Some(7));
    assert_eq!(acceptor_from_labels(&s.raw_labels), Some(Acceptor::Dev));
    assert_eq!(
        s.raw_labels,
        vec![
            "parent-epic-#7".to_string(),
            "parent-iteration-#1655.106".to_string(),
            "awaiting dev".to_string()
        ]
    );
    assert_eq!(
        s.state,
        BoardState::InProgress(Stage::Dev),
        "LabelsRefreshed is a pure typed assign; workflow state flows via StateTransitioned, not the label echo"
    );
}
