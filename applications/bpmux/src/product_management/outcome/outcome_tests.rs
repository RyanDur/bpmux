#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Outcome, OutcomeLifecycle};
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::story::{Horizon, Story, StoryId, StoryState};

fn outcome_story(state: StoryState, labels: Vec<String>) -> Story {
    let mut story = Story::new_minimal(
        StoryId::new(700),
        "[Outcome] reach the milestone".to_string(),
        state,
        chrono::Utc::now(),
        "pm".to_string(),
    );
    story.raw_labels = labels;
    story
}

#[test]
fn from_story_decodes_horizon_and_order_for_an_active_outcome() {
    let labels = vec![
        GhLabelCodec.encode_horizon(Horizon::Next),
        GhLabelCodec.encode_horizon_order(3),
    ];
    let outcome = Outcome::from_story(&outcome_story(StoryState::Ready, labels));
    assert_eq!(outcome.horizon(), Some(Horizon::Next));
    assert_eq!(outcome.horizon_order(), Some(3));
    assert!(!outcome.is_retired());
}

#[test]
fn from_story_maps_the_retired_state_to_the_retired_lifecycle() {
    let outcome = Outcome::from_story(&outcome_story(StoryState::Retired, vec![]));
    assert_eq!(outcome.lifecycle, OutcomeLifecycle::Retired);
    assert!(outcome.is_retired());
    assert_eq!(outcome.horizon(), None);
}
