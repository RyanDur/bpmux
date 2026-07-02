use super::{apply, created, meta, minimal, seed, seeded, story_at};
use crate::product_management::board::adapters::github::{
    acceptor_from_labels, parent_epic_from_labels,
};
use crate::product_management::board::events::{LabelDerivedFields, StoryEvent};
use crate::product_management::epic::EpicId;
use crate::product_management::story::{Acceptor, BoardState, Stage, StoryId, StoryKind, Tag};

#[test]
fn acceptor_assigned_lands_typed_without_any_label_projection_1551() {
    let store = seeded();
    created(&store, 1, "t", BoardState::Delivered);
    apply(
        &store,
        StoryEvent::AcceptorAssigned {
            meta: meta("designer", 2026, "local"),
            id: StoryId::new(1),
            acceptor: Acceptor::Team,
        },
    );
    assert_eq!(
        story_at(&store, 1).unwrap().acceptor(),
        Some(Acceptor::Team),
        "the acceptor lands via its typed event — the awaiting-label projection is gone; \
         generic awaiting-label adds reach the domain only through the adapter refresh"
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
    assert!(
        story_at(&store, 1).unwrap().tags().is_empty(),
        "a generic raw-label add never lands in the typed tags field"
    );
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
    assert_eq!(
        story_at(&store, 1).unwrap().story_type(),
        StoryKind::Bug,
        "the reducer takes story_type from the StoryCreated event itself"
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
                iteration_slot: None,
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
