#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{
    BoardAction, BoardPlacements, fine_stage, placement_of, placements_for_ingest, reduce,
    state_label,
};
use crate::product_management::story::{BoardState, Stage, Story, StoryId};
use crate::workflow::StageId;
use chrono::{TimeZone, Utc};

fn in_progress_story(id: u64) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("story {id}"),
        BoardState::InProgress,
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

fn board_with_desk_check(story: u64) -> BoardPlacements {
    reduce(&BoardPlacements::default(), &place(story, "desk-check")).unwrap()
}

#[test]
fn fine_stage_reads_the_placement() {
    let story = in_progress_story(1);
    assert_eq!(
        fine_stage(&story, &board_with_desk_check(1)),
        Some(Stage::DeskCheck),
        "the fine sub-section is read from the placement map"
    );
}

#[test]
fn fine_stage_defaults_to_dev_when_unplaced() {
    let story = in_progress_story(1);
    assert_eq!(
        fine_stage(&story, &BoardPlacements::default()),
        Some(Stage::Dev),
        "a middle story with no placement is the entry sub-section (dev), not desk-check"
    );
}

#[test]
fn fine_stage_is_none_outside_the_middle() {
    let mut story = in_progress_story(1);
    story.state = BoardState::Ready;
    assert_eq!(fine_stage(&story, &board_with_desk_check(1)), None);
}

#[test]
fn fine_stage_defaults_an_unknown_placement_id_to_dev() {
    let story = in_progress_story(1);
    let board = reduce(&BoardPlacements::default(), &place(1, "qa")).unwrap();
    assert_eq!(
        fine_stage(&story, &board),
        Some(Stage::Dev),
        "an unrecognized stage id resolves to the entry sub-section, mirroring as_str"
    );
}

#[test]
fn state_label_composes_the_middle_sub_section_from_the_fine_stage() {
    assert_eq!(
        state_label(&BoardState::InProgress, Some(Stage::DeskCheck)),
        "desk-check"
    );
    assert_eq!(
        state_label(&BoardState::InProgress, Some(Stage::Dev)),
        "in-progress"
    );
}

#[test]
fn state_label_passes_a_non_middle_position_through_to_its_coarse_token() {
    assert_eq!(state_label(&BoardState::Ready, None), "ready");
    assert_eq!(state_label(&BoardState::Acceptance, None), "acceptance");
    assert_eq!(state_label(&BoardState::Accepted, None), "accepted");
}

#[test]
fn ingest_places_the_story_the_desk_check_signal_names_even_when_state_says_dev() {
    let desk_check = [(StoryId::new(1), Stage::DeskCheck.id())];
    assert_eq!(
        placements_for_ingest(&[in_progress_story(1)], &desk_check),
        vec![BoardAction::StoryPlaced {
            story: StoryId::new(1),
            stage: Stage::DeskCheck.id(),
        }],
        "the label-derived signal drives the placement — the story's own coarse stage does not"
    );
}

#[test]
fn ingest_unplaces_a_middle_story_absent_from_the_desk_check_signal() {
    assert_eq!(
        placements_for_ingest(&[in_progress_story(1)], &[]),
        vec![BoardAction::StoryUnplaced {
            story: StoryId::new(1),
        }],
        "a middle story the signal does not name is the entry sub-section; a stale desk-check entry clears"
    );
}

#[test]
fn ingest_ignores_stories_outside_the_middle() {
    let mut ready = in_progress_story(1);
    ready.state = BoardState::Ready;
    let mut accepted = in_progress_story(2);
    accepted.state = BoardState::Accepted;
    let desk_check = [
        (StoryId::new(1), Stage::DeskCheck.id()),
        (StoryId::new(2), Stage::DeskCheck.id()),
    ];
    assert!(
        placements_for_ingest(&[ready, accepted], &desk_check).is_empty(),
        "coarse stories carry no middle placement even if the signal names them"
    );
}
