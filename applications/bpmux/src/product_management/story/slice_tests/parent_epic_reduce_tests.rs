use super::{apply, created, meta, seeded, story_at};
use crate::product_management::epic::EpicId;
use crate::product_management::story::events::{StoryEvent, StoryEventKind};
use crate::product_management::story::{StoryId, Work};

#[test]
fn parent_epic_set_event_decodes_into_the_stored_parent_epic_field() {
    let store = seeded();
    created(&store, 12, "child story", Work::NotStarted);
    assert_eq!(
        story_at(&store, 12).unwrap().parent_epic(),
        None,
        "a freshly created story has no parent epic yet"
    );
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026, "local"),
            StoryId::new(12),
            StoryEventKind::ParentEpicSet {
                epic_id: EpicId::new(300),
            },
        ),
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
    created(&store, 100, "child", Work::NotStarted);
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026, "local"),
            StoryId::new(100),
            StoryEventKind::ParentEpicSet {
                epic_id: EpicId::new(50),
            },
        ),
    );
    assert_eq!(
        story_at(&store, 100).unwrap().parent_epic(),
        Some(EpicId::new(50))
    );
}

#[test]
fn parent_epic_unset_event_makes_story_read_no_parent_epic() {
    let store = seeded();
    created(&store, 101, "child", Work::NotStarted);
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026, "local"),
            StoryId::new(101),
            StoryEventKind::ParentEpicSet {
                epic_id: EpicId::new(51),
            },
        ),
    );
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026, "local"),
            StoryId::new(101),
            StoryEventKind::ParentEpicUnset,
        ),
    );
    assert_eq!(story_at(&store, 101).unwrap().parent_epic(), None);
}

#[test]
fn parent_epic_events_no_op_on_missing_story() {
    let store = seeded();
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026, "local"),
            StoryId::new(999),
            StoryEventKind::ParentEpicSet {
                epic_id: EpicId::new(888),
            },
        ),
    );
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026, "local"),
            StoryId::new(999),
            StoryEventKind::ParentEpicUnset,
        ),
    );
    assert!(story_at(&store, 999).is_none());
}
