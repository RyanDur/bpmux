#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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

/// Truth-table mirror of the renderer's `columnFor` from
/// `applications/electron/src/renderer/src/board/types.ts`.
///
/// Encoded by hand from the TS source so this test catches drift on
/// either side. Update both when the projection legitimately changes.
fn renderer_column_for(state: &str, is_closed: bool) -> &'static str {
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
fn renderer_column_projection_agrees_with_rust_over_the_reachable_wire_space() {
    let reachable: [(BoardState, Option<Stage>, &str); 7] = [
        (BoardState::Icebox, None, "icebox"),
        (BoardState::Ready, None, "ready"),
        (BoardState::InProgress, Some(Stage::Dev), "in-progress"),
        (BoardState::InProgress, Some(Stage::DeskCheck), "desk-check"),
        (BoardState::Acceptance, None, "acceptance"),
        (BoardState::Accepted, None, "accepted"),
        (BoardState::Retired, None, "retired"),
    ];
    for (state, fine, wire_state) in reachable {
        let is_closed = derived_gh_closed(&state);
        let renderer = renderer_column_for(wire_state, is_closed);
        let rust =
            rust_column_as_wire_str(&column_of(&story_in(state.clone()), fine.map(Stage::id)));
        assert_eq!(
            rust, renderer,
            "drift: wire state={wire_state:?} gh_closed={is_closed} — Rust column_of = {rust:?}, renderer columnFor (mirrored) = {renderer:?}"
        );
    }
}

#[test]
fn renderer_legacy_wire_states_still_bucket_into_live_columns() {
    assert_eq!(renderer_column_for("ready-for-accept", false), "acceptance");
    assert_eq!(renderer_column_for("needs-fixes", false), "in-progress");
    assert_eq!(renderer_column_for("rejected", false), "ready");
}

#[test]
fn open_workflow_states_bucket_one_to_one_to_their_columns() {
    assert_eq!(
        column_of(&story_in(BoardState::Icebox), None),
        Column::Icebox
    );
    assert_eq!(column_of(&story_in(BoardState::Ready), None), Column::Ready);
    assert_eq!(
        column_of(&story_in(BoardState::InProgress), None),
        Column::Middle(Stage::Dev.id())
    );
    assert_eq!(
        column_of(&story_in(BoardState::Acceptance), None),
        Column::Acceptance
    );
}

#[test]
fn terminal_states_bucket_to_their_own_columns() {
    assert_eq!(
        column_of(&story_in(BoardState::Accepted), None),
        Column::Accepted
    );
    assert_eq!(
        column_of(&story_in(BoardState::Retired), None),
        Column::Retired
    );
}
