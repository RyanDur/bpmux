use super::{apply, created, meta, seeded, story_at};
use crate::product_management::epic::EpicId;
use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::{BoardState, StoryId};

#[test]
fn parent_epic_set_event_decodes_into_the_stored_parent_epic_field() {
    let store = seeded();
    created(&store, 12, "child story", BoardState::Icebox);
    assert_eq!(
        story_at(&store, 12).unwrap().parent_epic(),
        None,
        "a freshly created story has no parent epic yet"
    );
    apply(
        &store,
        StoryEvent::ParentEpicSet {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(12),
            epic_id: EpicId::new(300),
        },
    );
    assert_eq!(
        story_at(&store, 12).unwrap().parent_epic(),
        Some(EpicId::new(300)),
        "the ParentEpicSet event sets the typed parent_epic field directly"
    );
}

#[test]
fn parent_epic_set_event_makes_story_read_its_parent_epic() {
    let store = seeded();
    created(&store, 100, "child", BoardState::Ready);
    apply(
        &store,
        StoryEvent::ParentEpicSet {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(100),
            epic_id: EpicId::new(50),
        },
    );
    assert_eq!(
        story_at(&store, 100).unwrap().parent_epic(),
        Some(EpicId::new(50))
    );
}

#[test]
fn parent_epic_unset_event_makes_story_read_no_parent_epic() {
    let store = seeded();
    created(&store, 101, "child", BoardState::Ready);
    apply(
        &store,
        StoryEvent::ParentEpicSet {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(101),
            epic_id: EpicId::new(51),
        },
    );
    apply(
        &store,
        StoryEvent::ParentEpicUnset {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(101),
        },
    );
    assert_eq!(story_at(&store, 101).unwrap().parent_epic(), None);
}

#[test]
fn parent_epic_events_no_op_on_missing_story() {
    let store = seeded();
    apply(
        &store,
        StoryEvent::ParentEpicSet {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(999),
            epic_id: EpicId::new(888),
        },
    );
    apply(
        &store,
        StoryEvent::ParentEpicUnset {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(999),
        },
    );
    assert!(story_at(&store, 999).is_none());
}
