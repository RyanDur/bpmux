#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bpmux::product_management::iteration::{Iteration, IterationId};
use bpmux::product_management::story::{BoardState, Stage, Story, StoryId};
use bpmux::view::board::arrangement::{Column, column_of};
use chrono::Utc;

fn story_in(state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(1),
        "sample".to_string(),
        state,
        Utc::now(),
        "pm".to_string(),
    )
}

fn current_iteration_of(members: &[u64]) -> Vec<Iteration> {
    vec![Iteration {
        id: IterationId::new(9000),
        title: String::new(),
        body: None,
        filed_at: Utc::now(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: members.iter().map(|n| StoryId::new(*n)).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    }]
}

fn renderer_column_for_mirrored_by_hand_from_types_ts(
    state: &str,
    is_closed: bool,
) -> &'static str {
    if is_closed && state != "accepted" {
        return "retired";
    }
    match state {
        "ready-for-accept" => "acceptance",
        "needs-fixes" => "in-progress",
        "rejected" => "ready",
        "icebox" => "icebox",
        "ready" => "ready",
        "in-progress" => "in-progress",
        "desk-check" => "desk-check",
        "acceptance" => "acceptance",
        "accepted" => "accepted",
        "retired" => "retired",
        other => panic!("renderer mirror got unknown wire state {other:?}"),
    }
}

fn rust_column_as_wire_str(column: &Column) -> String {
    match column {
        Column::Icebox => "icebox".to_string(),
        Column::Ready => "ready".to_string(),
        Column::Middle(stage) => stage.as_str().to_string(),
        Column::Acceptance => "acceptance".to_string(),
        Column::Accepted => "accepted".to_string(),
        Column::Retired => "retired".to_string(),
    }
}

fn derived_gh_closed(state: &BoardState) -> bool {
    story_in(state.clone()).is_concluded()
}

#[test]
fn every_reachable_wire_state_derives_from_the_projection_and_agrees_with_the_renderer_mirror() {
    let non_member: Vec<Iteration> = Vec::new();
    let member = current_iteration_of(&[1]);
    let projection_input_to_wire: [(BoardState, Option<Stage>, &[Iteration], &str); 7] = [
        (BoardState::NotStarted, None, &non_member, "icebox"),
        (BoardState::NotStarted, None, &member, "ready"),
        (
            BoardState::InProgress,
            Some(Stage::Dev),
            &non_member,
            "in-progress",
        ),
        (
            BoardState::InProgress,
            Some(Stage::DeskCheck),
            &non_member,
            "desk-check",
        ),
        (BoardState::Acceptance, None, &non_member, "acceptance"),
        (BoardState::Accepted, None, &non_member, "accepted"),
        (BoardState::Retired, None, &non_member, "retired"),
    ];
    for (state, fine, iterations, wire_state) in projection_input_to_wire {
        let is_closed = derived_gh_closed(&state);
        let renderer = renderer_column_for_mirrored_by_hand_from_types_ts(wire_state, is_closed);
        let rust = rust_column_as_wire_str(&column_of(
            &story_in(state.clone()),
            fine.map(Stage::id),
            iterations,
        ));
        assert_eq!(
            rust, renderer,
            "drift: wire state={wire_state:?} gh_closed={is_closed} — Rust column_of = {rust:?}, renderer columnFor (mirrored from types.ts; update BOTH sides when the projection legitimately changes) = {renderer:?}"
        );
    }
}

#[test]
fn renderer_legacy_wire_states_still_bucket_into_live_columns() {
    assert_eq!(
        renderer_column_for_mirrored_by_hand_from_types_ts("ready-for-accept", false),
        "acceptance"
    );
    assert_eq!(
        renderer_column_for_mirrored_by_hand_from_types_ts("needs-fixes", false),
        "in-progress"
    );
    assert_eq!(
        renderer_column_for_mirrored_by_hand_from_types_ts("rejected", false),
        "ready"
    );
}

#[test]
fn open_workflow_states_bucket_one_to_one_to_their_columns() {
    assert_eq!(
        column_of(&story_in(BoardState::NotStarted), None, &[]),
        Column::Icebox
    );
    assert_eq!(
        column_of(
            &story_in(BoardState::NotStarted),
            None,
            &current_iteration_of(&[1])
        ),
        Column::Ready
    );
    assert_eq!(
        column_of(&story_in(BoardState::InProgress), None, &[]),
        Column::Middle(Stage::Dev.id())
    );
    assert_eq!(
        column_of(&story_in(BoardState::Acceptance), None, &[]),
        Column::Acceptance
    );
}

#[test]
fn a_not_started_non_member_buckets_to_icebox() {
    assert_eq!(
        column_of(
            &story_in(BoardState::NotStarted),
            None,
            &current_iteration_of(&[])
        ),
        Column::Icebox,
        "ready-ness derives from current-iteration membership — a not-started non-member is icebox on the board"
    );
    assert_eq!(
        column_of(&story_in(BoardState::NotStarted), None, &[]),
        Column::Icebox,
        "with no iteration at all, nothing is ready"
    );
}

#[test]
fn terminal_states_bucket_to_their_own_columns() {
    assert_eq!(
        column_of(&story_in(BoardState::Accepted), None, &[]),
        Column::Accepted
    );
    assert_eq!(
        column_of(&story_in(BoardState::Retired), None, &[]),
        Column::Retired
    );
}
