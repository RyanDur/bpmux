#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::board_placement::unplace_departed;
use crate::board::{BoardAction, BoardPlacements, placement_edge, reduce};
use crate::product_management::story::events::{EventMeta, StoryEvent};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::slice::{Stories, StoryAction};
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

fn transition(id: u64, placement: Option<StageId>) -> StoryAction {
    StoryAction::Event(StoryEvent::StateTransitioned {
        meta: EventMeta {
            actor: "dev".to_string(),
            ts: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            peer_id: PeerId::local(),
        },
        id: StoryId::new(id),
        new_state: BoardState::InProgress,
        placement,
    })
}

fn batched_intent(id: u64, placement: Option<StageId>) -> StoryAction {
    let StoryAction::Event(state) = transition(id, placement) else {
        unreachable!()
    };
    let gate = StoryEvent::VerdictCast {
        meta: EventMeta {
            actor: "dev".to_string(),
            ts: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            peer_id: PeerId::local(),
        },
        id: StoryId::new(id),
        verdict: crate::product_management::story::Verdict::Succeed {
            step: "in-progress".into(),
        },
        payload: None,
    };
    StoryAction::Events(vec![gate, state])
}

#[test]
fn a_transition_carrying_a_placement_places_from_it_not_from_the_state() {
    let edge = placement_edge(
        &BoardPlacements::default(),
        &transition(1, Some(Stage::DeskCheck.id())),
    );
    assert_eq!(
        edge,
        Some(BoardAction::StoryPlaced {
            story: StoryId::new(1),
            stage: Stage::DeskCheck.id(),
        }),
        "the fine placement rides the event — the coarse new_state (Dev) does not decide it"
    );
}

#[test]
fn a_transition_with_no_placement_unplaces_a_placed_story() {
    let edge = placement_edge(&board(&[(1, "desk-check")]), &transition(1, None));
    assert_eq!(
        edge,
        Some(BoardAction::StoryUnplaced {
            story: StoryId::new(1),
        }),
        "a transition to the entry sub-section (placement None) clears the middle placement"
    );
}

#[test]
fn a_transition_placing_what_is_already_placed_is_a_no_op() {
    let edge = placement_edge(
        &board(&[(1, "desk-check")]),
        &transition(1, Some(Stage::DeskCheck.id())),
    );
    assert_eq!(
        edge, None,
        "an idempotent placement must not churn the board"
    );
}

#[test]
fn a_transition_unplacing_what_is_not_placed_is_a_no_op() {
    let edge = placement_edge(&BoardPlacements::default(), &transition(1, None));
    assert_eq!(edge, None);
}

#[test]
fn a_batched_intent_places_from_the_state_transition_inside_it() {
    let edge = placement_edge(
        &BoardPlacements::default(),
        &batched_intent(1, Some(Stage::DeskCheck.id())),
    );
    assert_eq!(
        edge,
        Some(BoardAction::StoryPlaced {
            story: StoryId::new(1),
            stage: Stage::DeskCheck.id(),
        }),
        "an atomic-intent batch projects the placement from its StateTransitioned event"
    );
}

#[test]
fn a_batched_intent_with_no_placement_unplaces() {
    let edge = placement_edge(&board(&[(1, "desk-check")]), &batched_intent(1, None));
    assert_eq!(
        edge,
        Some(BoardAction::StoryUnplaced {
            story: StoryId::new(1),
        }),
        "a batched intent transitioning to the entry sub-section clears the placement"
    );
}

#[test]
fn a_non_transition_story_action_yields_no_edge() {
    let edge = placement_edge(
        &board(&[(1, "desk-check")]),
        &StoryAction::Removed(StoryId::new(1)),
    );
    assert_eq!(
        edge, None,
        "only StateTransitioned carries a placement; other story actions are handled by the safety-net"
    );
}

#[test]
fn a_placement_whose_story_left_the_middle_is_unplaced() {
    let changes = unplace_departed(
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
    let changes = unplace_departed(&stories(vec![]), &board(&[(1, "desk-check")]));
    assert_eq!(
        changes,
        vec![BoardAction::StoryUnplaced {
            story: StoryId::new(1),
        }]
    );
}

#[test]
fn a_story_still_in_the_middle_is_left_placed_by_the_safety_net() {
    let changes = unplace_departed(
        &stories(vec![story(1, BoardState::InProgress)]),
        &board(&[(1, "desk-check")]),
    );
    assert!(
        changes.is_empty(),
        "the coarse-only safety-net never touches a story still in the middle"
    );
}

#[test]
fn the_safety_net_is_coarse_only_a_placed_dev_story_is_not_its_concern() {
    let changes = unplace_departed(
        &stories(vec![story(1, BoardState::InProgress)]),
        &board(&[(1, "in-progress")]),
    );
    assert!(
        changes.is_empty(),
        "reshaping a placed Dev story off the map is ingest's job, not the coarse safety-net's"
    );
}

#[test]
fn a_cold_start_hydrates_the_desk_check_placement_from_the_file() {
    let dir = std::env::temp_dir().join(format!("bpmux-hydrate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let placed = reduce(
        &BoardPlacements::default(),
        &BoardAction::StoryPlaced {
            story: StoryId::new(9),
            stage: Stage::DeskCheck.id(),
        },
    )
    .unwrap();
    crate::board::persistence::save(&dir, &placed).unwrap();
    let state = crate::store::AppState::with_board(crate::board::persistence::load(&dir));
    assert_eq!(
        crate::board::placement_of(&state.board, StoryId::new(9)),
        Some(Stage::DeskCheck.id()),
        "a backend restart recovers the fine desk-check stage from placements.json, with no GitHub label involved"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
