use super::{apply, created, meta, minimal, seed, seeded, story_at};
use product_management::epic::EpicId;
use product_management::story::events::{LabelDerivedFields, StoryEvent, StoryEventKind};
use product_management::story::{Acceptor, StoryId, StoryKind, Tag, Work};

#[test]
fn acceptor_assigned_lands_typed_without_any_label_projection() {
    let store = seeded();
    created(&store, 1, "t", Work::Acceptance);
    apply(
        &store,
        StoryEvent::new(
            meta("designer", 2026),
            StoryId::new(1),
            StoryEventKind::AcceptorAssigned {
                acceptor: Acceptor::Team,
            },
        ),
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
    created(&store, 1, "t", Work::NotStarted);
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(1),
            StoryEventKind::TagAdded {
                label: "high-priority".to_string(),
            },
        ),
    );
    assert!(
        story_at(&store, 1).unwrap().tags().is_empty(),
        "a generic raw-label add never lands in the typed tags field"
    );
}

#[test]
fn generic_label_churn_does_not_clobber_the_typed_tags_field() {
    let store = seeded();
    created(&store, 1, "t", Work::NotStarted);
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(1),
            StoryEventKind::TagSet {
                tag: Tag::DeskCheckFail,
            },
        ),
    );
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(1),
            StoryEventKind::TagAdded {
                label: "high-priority".to_string(),
            },
        ),
    );
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(1),
            StoryEventKind::TagRemoved {
                label: "high-priority".to_string(),
            },
        ),
    );
    assert_eq!(
        story_at(&store, 1).unwrap().tags(),
        [Tag::DeskCheckFail],
        "a generic raw-label add/remove must not re-project (clobber) the typed tags field"
    );
}

#[test]
fn story_created_event_carries_story_type() {
    let store = seeded();
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(1),
            StoryEventKind::StoryCreated {
                title: "t".to_string(),
                body: "b".to_string(),
                initial_state: Work::NotStarted,
                story_type: StoryKind::Bug,
            },
        ),
    );
    assert_eq!(
        story_at(&store, 1).unwrap().story_type(),
        StoryKind::Bug,
        "the reducer takes story_type from the StoryCreated event itself"
    );
}

#[test]
fn labels_refreshed_purely_assigns_typed_fields_and_leaves_state_to_state_transitioned() {
    let store = seeded();
    seed(&store, vec![minimal(77, "membered", Work::InProgress)]);
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(77),
            StoryEventKind::LabelsRefreshed {
                fields: LabelDerivedFields {
                    raw_labels: vec![
                        "parent-epic-#7".to_string(),
                        "parent-iteration-#1655.106".to_string(),
                        "awaiting dev".to_string(),
                    ],
                    acceptor: Some(Acceptor::Dev),
                    parent_epic: Some(EpicId::new(7)),
                    tags: Vec::new(),
                },
            },
        ),
    );
    let s = story_at(&store, 77).expect("story exists");
    assert_eq!(s.parent_epic(), Some(EpicId::new(7)));
    assert_eq!(s.acceptor(), Some(Acceptor::Dev));
    assert_eq!(
        s.state,
        Work::InProgress,
        "LabelsRefreshed is a pure typed assign; workflow state flows via StateTransitioned, not the label echo"
    );
}
