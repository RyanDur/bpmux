#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::story_type::{StoryKind, StoryType, story_type_from_title};

#[test]
fn story_type_from_title_reads_bpmux_title_conventions() {
    assert_eq!(story_type_from_title("[Epic] foo"), StoryType::Epic);
    assert_eq!(
        story_type_from_title("[Iteration] Aquila"),
        StoryType::Iteration
    );
    assert_eq!(story_type_from_title("[Outcome] reach"), StoryType::Outcome);
    assert_eq!(
        story_type_from_title("[Bug] broken"),
        StoryType::Story(StoryKind::Bug)
    );
    assert_eq!(
        story_type_from_title("bug: lowercase prefix"),
        StoryType::Story(StoryKind::Bug)
    );
    assert_eq!(
        story_type_from_title("[Chore] cleanup"),
        StoryType::Story(StoryKind::Chore)
    );
    assert_eq!(
        story_type_from_title("[honest-plumbing] wiring"),
        StoryType::Story(StoryKind::Chore)
    );
    assert_eq!(
        story_type_from_title("[half-slice] part"),
        StoryType::Story(StoryKind::Chore)
    );
}

#[test]
fn story_type_from_title_defaults_to_feature_without_a_known_prefix() {
    assert_eq!(
        story_type_from_title("just a title"),
        StoryType::Story(StoryKind::Feature)
    );
    assert_eq!(
        story_type_from_title("[Feature] explicit"),
        StoryType::Story(StoryKind::Feature)
    );
    assert_eq!(
        story_type_from_title(""),
        StoryType::Story(StoryKind::Feature)
    );
}

#[test]
fn story_type_serde_representation_is_flat_and_stable_for_the_outbox() {
    for (story_type, on_disk) in [
        (StoryType::Story(StoryKind::Feature), "\"Feature\""),
        (StoryType::Story(StoryKind::Bug), "\"Bug\""),
        (StoryType::Story(StoryKind::Chore), "\"Chore\""),
        (StoryType::Epic, "\"Epic\""),
        (StoryType::Iteration, "\"Iteration\""),
        (StoryType::Outcome, "\"Outcome\""),
    ] {
        assert_eq!(
            serde_json::to_string(&story_type).unwrap(),
            on_disk,
            "{story_type:?} must serialize to the flat string the outbox already persisted"
        );
        assert_eq!(
            serde_json::from_str::<StoryType>(on_disk).unwrap(),
            story_type,
            "{on_disk} must deserialize back (queued events from before the split still load)"
        );
    }
}
