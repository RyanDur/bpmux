use super::{apply, created, meta, seeded, story_at};
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::{Acceptor, StoryId, StoryKind, Tag, Work};

#[test]
fn story_created_event_populates_the_central_store() {
    let store = seeded();
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(500),
            StoryEventKind::StoryCreated {
                title: "new story".to_string(),
                body: "body text".to_string(),
                initial_state: Work::NotStarted,
                story_type: StoryKind::Feature,
            },
        ),
    );
    let story = story_at(&store, 500).expect("story exists");
    assert_eq!(story.title, "new story");
    assert_eq!(story.state, Work::NotStarted);
    assert_eq!(story.body.as_deref(), Some("body text"));
    assert_eq!(story.filed_by, "pm");
}

#[test]
fn state_transitioned_event_updates_existing_story() {
    let store = seeded();
    created(&store, 1, "t", Work::NotStarted);
    apply(
        &store,
        StoryEvent::new(
            meta("dev", 2026),
            StoryId::new(1),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ),
    );
    assert_eq!(story_at(&store, 1).unwrap().state, Work::InProgress);
}

#[test]
fn state_transitioned_event_silently_drops_when_story_missing() {
    let store = seeded();
    apply(
        &store,
        StoryEvent::new(
            meta("dev", 2026),
            StoryId::new(999),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ),
    );
    assert!(story_at(&store, 999).is_none());
}

#[test]
fn acceptor_assigned_event_sets_the_stored_acceptor_field() {
    let store = seeded();
    created(&store, 8, "needs an acceptor", Work::Acceptance);
    assert_eq!(
        story_at(&store, 8).unwrap().acceptor(),
        None,
        "a freshly created story carries no acceptor yet"
    );
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(8),
            StoryEventKind::AcceptorAssigned {
                acceptor: Acceptor::Owner,
            },
        ),
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
    created(&store, 15, "blockable", Work::Acceptance);
    assert!(
        story_at(&store, 15).unwrap().tags().is_empty(),
        "a freshly created story carries no tags yet"
    );
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(15),
            StoryEventKind::TagSet {
                tag: Tag::DeskCheckFail,
            },
        ),
    );
    assert_eq!(
        story_at(&store, 15).unwrap().tags(),
        [Tag::DeskCheckFail],
        "the TagSet event assigns the typed tag directly, not via a label round-trip"
    );
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(15),
            StoryEventKind::TagSet {
                tag: Tag::DeskCheckFail,
            },
        ),
    );
    assert_eq!(
        story_at(&store, 15).unwrap().tags(),
        [Tag::DeskCheckFail],
        "TagSet is idempotent — re-setting the same tag does not duplicate it"
    );
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(15),
            StoryEventKind::TagUnset {
                tag: Tag::DeskCheckFail,
            },
        ),
    );
    assert!(
        story_at(&store, 15).unwrap().tags().is_empty(),
        "the TagUnset event clears the typed tag directly"
    );
}
