#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{BoardAction, BoardPlacements, placement_of, reduce};
use crate::product_management::story::StoryId;
use crate::workflow::StageId;

fn place(story: u64, stage: &str) -> BoardAction {
    BoardAction::StoryPlaced {
        story: StoryId::new(story),
        stage: StageId::from(stage),
    }
}

fn placed_stage(state: &BoardPlacements, story: u64) -> Option<StageId> {
    placement_of(state, StoryId::new(story))
}

#[test]
fn placing_a_story_records_its_middle_stage() {
    let state = reduce(&BoardPlacements::default(), &place(1, "desk-check"))
        .expect("placing into an empty board changes state");
    assert_eq!(placed_stage(&state, 1), Some(StageId::from("desk-check")));
}

#[test]
fn placing_the_same_stage_again_does_not_churn_the_store() {
    let state = reduce(&BoardPlacements::default(), &place(1, "desk-check")).unwrap();
    assert!(
        reduce(&state, &place(1, "desk-check")).is_none(),
        "an idempotent placement must reduce to no change"
    );
}

#[test]
fn re_placing_a_story_moves_it_to_the_new_stage() {
    let state = reduce(&BoardPlacements::default(), &place(1, "in-progress")).unwrap();
    let moved =
        reduce(&state, &place(1, "desk-check")).expect("moving to a new stage changes state");
    assert_eq!(placed_stage(&moved, 1), Some(StageId::from("desk-check")));
}

#[test]
fn unplacing_a_story_removes_its_placement() {
    let state = reduce(&BoardPlacements::default(), &place(1, "desk-check")).unwrap();
    let cleared = reduce(
        &state,
        &BoardAction::StoryUnplaced {
            story: StoryId::new(1),
        },
    )
    .expect("unplacing a placed story changes state");
    assert_eq!(placed_stage(&cleared, 1), None);
}

#[test]
fn unplacing_an_absent_story_does_not_churn_the_store() {
    assert!(
        reduce(
            &BoardPlacements::default(),
            &BoardAction::StoryUnplaced {
                story: StoryId::new(1),
            },
        )
        .is_none()
    );
}

#[test]
fn a_story_with_no_placement_is_not_in_the_middle() {
    assert_eq!(placed_stage(&BoardPlacements::default(), 1), None);
}
