#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::OutcomeLifecycle;
use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::integrations::github::mapper::decode_outcome_from_story;
use crate::product_management::story::{BoardState, Horizon, Story, StoryId};

fn outcome_story(state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(700),
        "[Outcome] reach the milestone".to_string(),
        state,
        chrono::Utc::now(),
        "pm".to_string(),
    )
}

#[test]
fn from_story_decodes_horizon_and_order_for_an_active_outcome() {
    let labels = vec![
        GhLabelCodec.encode_horizon(Horizon::Next),
        GhLabelCodec.encode_horizon_order(3),
    ];
    let outcome = decode_outcome_from_story(&outcome_story(BoardState::NotStarted), &labels);
    assert_eq!(outcome.horizon(), Some(Horizon::Next));
    assert_eq!(outcome.horizon_order(), Some(3));
    assert!(!outcome.is_retired());
}

#[test]
fn from_story_maps_the_retired_state_to_the_retired_lifecycle() {
    let outcome = decode_outcome_from_story(&outcome_story(BoardState::Retired), &[]);
    assert_eq!(outcome.lifecycle, OutcomeLifecycle::Retired);
    assert!(outcome.is_retired());
    assert_eq!(outcome.horizon(), None);
}
