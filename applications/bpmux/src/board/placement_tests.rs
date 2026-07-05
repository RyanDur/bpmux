#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{BoardAction, BoardPlacements, placement_of, reduce, story_position};
use crate::product_management::story::{BoardState, Stage, Story, StoryId};
use crate::workflow::StageId;
use chrono::{TimeZone, Utc};

fn in_progress_story(id: u64, stage: Stage) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("story {id}"),
        BoardState::InProgress(stage.id()),
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

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

#[test]
fn story_position_composes_the_fine_placement_over_the_coarse_position() {
    let story = in_progress_story(1, Stage::Dev);
    let board = board_with_desk_check(1);
    assert_eq!(
        story_position(&story, &board),
        BoardState::InProgress(Stage::DeskCheck.id()),
        "the fine placement wins over the story's own middle stage"
    );
}

#[test]
fn story_position_falls_back_to_the_story_stage_when_unplaced() {
    let story = in_progress_story(1, Stage::DeskCheck);
    assert_eq!(
        story_position(&story, &BoardPlacements::default()),
        BoardState::InProgress(Stage::DeskCheck.id()),
        "with no placement the story's own stage stands (the transitional fallback)"
    );
}

#[test]
fn story_position_leaves_a_non_middle_position_untouched() {
    let mut story = in_progress_story(1, Stage::Dev);
    story.state = BoardState::Acceptance;
    assert_eq!(
        story_position(&story, &board_with_desk_check(1)),
        BoardState::Acceptance
    );
}

fn board_with_desk_check(story: u64) -> BoardPlacements {
    reduce(&BoardPlacements::default(), &place(story, "desk-check")).unwrap()
}
