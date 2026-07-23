use crate::story::slice::{Stories, StoryAction, all, feature_ids, get, is_empty, reduce};
use crate::story::{Story, StoryId, StoryKind, Work};
use chrono::{TimeZone, Utc};

fn story_of(id: u64, kind: StoryKind) -> Story {
    let mut story = Story::new_minimal(
        StoryId::new(id),
        format!("story-{id}"),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 7, 7, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    story.story_type = kind;
    story
}

fn stories_of(stories: Vec<Story>) -> Stories {
    reduce(&Stories::default(), &StoryAction::Seeded(stories))
}

#[test]
fn all_returns_every_stored_story() {
    let stories = stories_of(vec![
        story_of(1, StoryKind::Feature),
        story_of(2, StoryKind::Bug),
        story_of(3, StoryKind::Chore),
    ]);

    let mut ids: Vec<u64> = all(&stories).iter().map(|story| story.id.value()).collect();
    ids.sort_unstable();

    assert_eq!(ids, vec![1, 2, 3]);
}

#[test]
fn get_hits_a_saved_story_and_misses_an_absent_id() {
    let stories = stories_of(vec![story_of(1, StoryKind::Feature)]);

    assert_eq!(
        get(&stories, StoryId::new(1)).map(|story| story.id.value()),
        Some(1)
    );
    assert_eq!(get(&stories, StoryId::new(99)), None);
}

#[test]
fn feature_ids_keeps_features_and_drops_bugs_and_chores() {
    let stories = stories_of(vec![
        story_of(1, StoryKind::Feature),
        story_of(2, StoryKind::Bug),
        story_of(3, StoryKind::Chore),
        story_of(4, StoryKind::Feature),
    ]);

    let mut ids: Vec<u64> = feature_ids(&stories).iter().map(|id| id.value()).collect();
    ids.sort_unstable();

    assert_eq!(ids, vec![1, 4]);
}

#[test]
fn is_empty_is_true_for_no_stories_and_false_once_one_is_stored() {
    assert!(is_empty(&Stories::default()));
    assert!(!is_empty(&stories_of(vec![story_of(
        1,
        StoryKind::Feature
    )])));
}
