#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::Iteration;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::story::{Story, StoryKey, StoryState};

fn iteration_story(labels: Vec<String>, closed: bool) -> Story {
    let mut story = Story::new_minimal(
        710,
        "Iteration Crux".to_string(),
        StoryState::Ready,
        chrono::Utc::now(),
        "pm".to_string(),
    );
    story.raw_labels = labels;
    story.is_closed = closed;
    story
}

#[test]
fn from_story_decodes_every_iteration_label_into_a_field() {
    let codec = GhLabelCodec;
    let iteration = Iteration::from_story(&iteration_story(
        vec![
            codec.encode_parent_outcome(900, 2),
            codec.encode_iteration_order(5),
            codec.encode_iteration_review_closed("designer"),
            codec.encode_iteration_dev_ok("dev"),
            codec.encode_current_iteration(),
        ],
        false,
    ));
    assert_eq!(iteration.id(), StoryKey::Saved(710));
    assert_eq!(iteration.content.title, "Iteration Crux");
    assert_eq!(iteration.outcome_membership, Some((900, 2)));
    assert_eq!(iteration.builds_outcome(), Some(900));
    assert_eq!(iteration.iteration_order, Some(5));
    assert_eq!(iteration.review_closers, vec!["designer".to_string()]);
    assert_eq!(iteration.dev_ok_by, vec!["dev".to_string()]);
    assert!(iteration.is_current);
    assert!(!iteration.complete);
}

#[test]
fn from_story_leaves_unlabelled_facts_empty_and_reads_closed_as_complete() {
    let iteration = Iteration::from_story(&iteration_story(vec![], true));
    assert_eq!(iteration.outcome_membership, None);
    assert_eq!(iteration.builds_outcome(), None);
    assert_eq!(iteration.iteration_order, None);
    assert!(iteration.review_closers.is_empty());
    assert!(iteration.dev_ok_by.is_empty());
    assert!(!iteration.is_current);
    assert!(
        iteration.complete,
        "a closed iteration's issue reads as complete"
    );
}
