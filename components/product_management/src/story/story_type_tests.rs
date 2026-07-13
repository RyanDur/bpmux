#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::story_type::{StoryKind, story_type_from_title};

#[test]
fn story_type_from_title_reads_bpmux_title_conventions() {
    assert_eq!(story_type_from_title("[Epic] foo"), StoryKind::Feature);
    assert_eq!(story_type_from_title("[Bug] broken"), StoryKind::Bug);
    assert_eq!(
        story_type_from_title("bug: lowercase prefix"),
        StoryKind::Bug
    );
    assert_eq!(story_type_from_title("[Chore] cleanup"), StoryKind::Chore);
    assert_eq!(
        story_type_from_title("[honest-plumbing] wiring"),
        StoryKind::Chore
    );
    assert_eq!(story_type_from_title("[half-slice] part"), StoryKind::Chore);
}

#[test]
fn story_type_from_title_defaults_to_feature_without_a_known_prefix() {
    assert_eq!(story_type_from_title("just a title"), StoryKind::Feature);
    assert_eq!(
        story_type_from_title("[Feature] explicit"),
        StoryKind::Feature
    );
    assert_eq!(story_type_from_title(""), StoryKind::Feature);
}

#[test]
fn story_type_serde_representation_is_flat_and_stable_for_the_outbox() {
    for (story_type, on_disk) in [
        (StoryKind::Feature, "\"Feature\""),
        (StoryKind::Bug, "\"Bug\""),
        (StoryKind::Chore, "\"Chore\""),
    ] {
        assert_eq!(
            serde_json::to_string(&story_type).unwrap(),
            on_disk,
            "{story_type:?} must serialize to the flat string the outbox already persisted"
        );
        assert_eq!(
            serde_json::from_str::<StoryKind>(on_disk).unwrap(),
            story_type,
            "{on_disk} must deserialize back (queued events from before the split still load)"
        );
    }
}
