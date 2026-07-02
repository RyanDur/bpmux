use super::{apply, created, meta, seeded, story_at};
use crate::product_management::board::events::StoryEvent;
use crate::product_management::story::{Acceptor, BoardState, Stage, StoryId, StoryKind, Tag};

#[test]
fn story_created_event_populates_the_central_store() {
    let store = seeded();
    apply(
        &store,
        StoryEvent::StoryCreated {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(500),
            title: "new story".to_string(),
            body: "body text".to_string(),
            initial_state: BoardState::Ready,
            story_type: StoryKind::Feature,
        },
    );
    let story = story_at(&store, 500).expect("story exists");
    assert_eq!(story.title, "new story");
    assert_eq!(story.state, BoardState::Ready);
    assert_eq!(story.body.as_deref(), Some("body text"));
    assert_eq!(story.filed_by, "pm");
}

#[test]
fn state_transitioned_event_updates_existing_story() {
    let store = seeded();
    created(&store, 1, "t", BoardState::Ready);
    apply(
        &store,
        StoryEvent::StateTransitioned {
            meta: meta("dev", 2026, "local"),
            id: StoryId::new(1),
            new_state: BoardState::InProgress(Stage::Dev),
        },
    );
    assert_eq!(
        story_at(&store, 1).unwrap().state,
        BoardState::InProgress(Stage::Dev)
    );
}

#[test]
fn state_transitioned_event_silently_drops_when_story_missing() {
    let store = seeded();
    apply(
        &store,
        StoryEvent::StateTransitioned {
            meta: meta("dev", 2026, "local"),
            id: StoryId::new(999),
            new_state: BoardState::InProgress(Stage::Dev),
        },
    );
    assert!(story_at(&store, 999).is_none());
}

#[test]
fn acceptor_assigned_event_sets_the_stored_acceptor_field() {
    let store = seeded();
    created(&store, 8, "needs an acceptor", BoardState::Acceptance);
    assert_eq!(
        story_at(&store, 8).unwrap().acceptor(),
        None,
        "a freshly created story carries no acceptor yet"
    );
    apply(
        &store,
        StoryEvent::AcceptorAssigned {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(8),
            acceptor: Acceptor::Owner,
        },
    );
    assert_eq!(
        story_at(&store, 8).unwrap().acceptor(),
        Some(Acceptor::Owner),
        "the AcceptorAssigned event sets the typed acceptor field directly"
    );
}

#[test]
fn tag_set_event_sets_the_typed_tags_field_directly() {
    let store = seeded();
    created(&store, 15, "blockable", BoardState::Acceptance);
    assert!(
        story_at(&store, 15).unwrap().tags().is_empty(),
        "a freshly created story carries no tags yet"
    );
    apply(
        &store,
        StoryEvent::TagSet {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(15),
            tag: Tag::Blocked,
        },
    );
    assert_eq!(
        story_at(&store, 15).unwrap().tags(),
        [Tag::Blocked],
        "the TagSet event assigns the typed tag directly, not via a label round-trip"
    );
    apply(
        &store,
        StoryEvent::TagSet {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(15),
            tag: Tag::Blocked,
        },
    );
    assert_eq!(
        story_at(&store, 15).unwrap().tags(),
        [Tag::Blocked],
        "TagSet is idempotent — re-setting the same tag does not duplicate it"
    );
    apply(
        &store,
        StoryEvent::TagUnset {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(15),
            tag: Tag::Blocked,
        },
    );
    assert!(
        story_at(&store, 15).unwrap().tags().is_empty(),
        "the TagUnset event clears the typed tag directly"
    );
}

#[test]
fn blocked_by_events_set_and_clear_the_typed_blocker_reference_directly() {
    let store = seeded();
    created(&store, 16, "blockable", BoardState::Acceptance);
    assert_eq!(
        story_at(&store, 16).unwrap().blocked_by(),
        None,
        "a freshly created story references no blocker"
    );
    apply(
        &store,
        StoryEvent::BlockedBySet {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(16),
            blocker: StoryId::new(9),
        },
    );
    assert_eq!(
        story_at(&store, 16).unwrap().blocked_by(),
        Some(StoryId::new(9)),
        "BlockedBySet assigns the typed blocker reference directly, not via a label round-trip"
    );
    apply(
        &store,
        StoryEvent::BlockedBySet {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(16),
            blocker: StoryId::new(11),
        },
    );
    assert_eq!(
        story_at(&store, 16).unwrap().blocked_by(),
        Some(StoryId::new(11)),
        "a later BlockedBySet replaces the blocker reference"
    );
    apply(
        &store,
        StoryEvent::BlockedByCleared {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(16),
        },
    );
    assert_eq!(
        story_at(&store, 16).unwrap().blocked_by(),
        None,
        "BlockedByCleared drops the typed blocker reference"
    );
}
