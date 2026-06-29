#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::adapters::github::decode_iteration;
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, Relationships,
};
use crate::product_management::story::{Story, StoryId, StoryKey, StoryState};

fn iteration_story(labels: Vec<String>, body: Option<String>, closed: bool) -> Story {
    let mut story = Story::new_minimal(
        StoryId::new(710),
        "Iteration Crux".to_string(),
        StoryState::Ready,
        chrono::Utc::now(),
        "pm".to_string(),
    );
    story.raw_labels = labels;
    story.body = body;
    story.is_closed = closed;
    story
}

#[test]
fn decode_reads_attributes_from_labels_and_the_tree_from_body_json() {
    let codec = GhLabelCodec;
    let body = IterationRelationships {
        outcome: Some(900),
        stories: vec![StoryId::new(101), StoryId::new(102)],
    }
    .write_into_body("## Iteration Crux\n");
    let iteration = decode_iteration(&iteration_story(
        vec![
            codec.encode_iteration_review_closed("designer"),
            codec.encode_iteration_dev_ok("dev"),
            codec.encode_current_iteration(),
        ],
        Some(body),
        false,
    ));
    assert_eq!(iteration.id(), StoryKey::Saved(StoryId::new(710)));
    assert_eq!(iteration.content.title, "Iteration Crux");
    assert_eq!(iteration.outcome, Some(900));
    assert_eq!(iteration.builds_outcome(), Some(900));
    assert_eq!(
        iteration.stories,
        vec![StoryId::new(101), StoryId::new(102)]
    );
    assert_eq!(iteration.review_closers, vec!["designer".to_string()]);
    assert_eq!(iteration.dev_ok_by, vec!["dev".to_string()]);
    assert!(iteration.is_current);
    assert!(!iteration.complete);
}

#[test]
fn decode_leaves_an_unannotated_iteration_empty_and_reads_closed_as_complete() {
    let iteration = decode_iteration(&iteration_story(vec![], None, true));
    assert_eq!(iteration.outcome, None);
    assert_eq!(iteration.builds_outcome(), None);
    assert!(iteration.stories.is_empty());
    assert!(iteration.review_closers.is_empty());
    assert!(iteration.dev_ok_by.is_empty());
    assert!(!iteration.is_current);
    assert!(
        iteration.complete,
        "a closed iteration's issue reads as complete"
    );
}
