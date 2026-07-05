#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::board_placement::reconcile_placements;
use crate::board::{BoardAction, BoardPlacements, reduce};
use crate::product_management::story::slice::Stories;
use crate::product_management::story::{BoardState, Stage, Story, StoryId};
use crate::workflow::StageId;
use chrono::{TimeZone, Utc};

fn story(id: u64, state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("story {id}"),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn stories(items: Vec<Story>) -> Stories {
    items.into_iter().map(|s| (s.key(), s)).collect()
}

fn board(placements: &[(u64, &str)]) -> BoardPlacements {
    let mut board = BoardPlacements::default();
    for (id, stage) in placements {
        board = reduce(
            &board,
            &BoardAction::StoryPlaced {
                story: StoryId::new(*id),
                stage: StageId::from(*stage),
            },
        )
        .expect("seeding a placement changes the board");
    }
    board
}

#[test]
fn an_in_progress_story_with_no_placement_is_placed_at_its_stage() {
    let changes = reconcile_placements(
        &stories(vec![story(1, BoardState::InProgress(Stage::Dev.id()))]),
        &BoardPlacements::default(),
    );
    assert_eq!(
        changes,
        vec![BoardAction::StoryPlaced {
            story: StoryId::new(1),
            stage: Stage::Dev.id(),
        }]
    );
}

#[test]
fn a_correctly_placed_story_reconciles_to_no_change() {
    let changes = reconcile_placements(
        &stories(vec![story(
            1,
            BoardState::InProgress(Stage::DeskCheck.id()),
        )]),
        &board(&[(1, "desk-check")]),
    );
    assert!(
        changes.is_empty(),
        "a placement that already matches must not churn"
    );
}

#[test]
fn a_story_moved_to_a_new_stage_is_re_placed() {
    let changes = reconcile_placements(
        &stories(vec![story(
            1,
            BoardState::InProgress(Stage::DeskCheck.id()),
        )]),
        &board(&[(1, "in-progress")]),
    );
    assert_eq!(
        changes,
        vec![BoardAction::StoryPlaced {
            story: StoryId::new(1),
            stage: Stage::DeskCheck.id(),
        }]
    );
}

#[test]
fn a_story_that_left_the_middle_is_unplaced() {
    let changes = reconcile_placements(
        &stories(vec![story(1, BoardState::Acceptance)]),
        &board(&[(1, "desk-check")]),
    );
    assert_eq!(
        changes,
        vec![BoardAction::StoryUnplaced {
            story: StoryId::new(1),
        }]
    );
}

#[test]
fn a_placement_whose_story_is_gone_is_unplaced() {
    let changes = reconcile_placements(&stories(vec![]), &board(&[(1, "desk-check")]));
    assert_eq!(
        changes,
        vec![BoardAction::StoryUnplaced {
            story: StoryId::new(1),
        }]
    );
}

#[test]
fn a_story_outside_the_middle_with_no_placement_needs_no_change() {
    let changes = reconcile_placements(
        &stories(vec![story(1, BoardState::Ready)]),
        &BoardPlacements::default(),
    );
    assert!(changes.is_empty());
}

#[test]
fn a_mixed_board_places_the_unplaced_and_unplaces_the_departed() {
    let changes = reconcile_placements(
        &stories(vec![
            story(1, BoardState::InProgress(Stage::Dev.id())),
            story(2, BoardState::Accepted),
        ]),
        &board(&[(2, "desk-check")]),
    );
    assert_eq!(changes.len(), 2);
    assert!(changes.contains(&BoardAction::StoryPlaced {
        story: StoryId::new(1),
        stage: Stage::Dev.id(),
    }));
    assert!(changes.contains(&BoardAction::StoryUnplaced {
        story: StoryId::new(2),
    }));
}
