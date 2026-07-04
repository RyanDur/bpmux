#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bpmux::product_management::story::BoardState;
use bpmux::product_management::story::Stage;
use bpmux::view::board::arrangement::board_position;

/// Truth-table mirror of the renderer's `columnFor` from
/// `applications/electron/src/renderer/src/board/types.ts`.
///
/// Encoded by hand from the TS source so this test catches drift on
/// either side. Update both when the projection legitimately changes.
fn renderer_board_position(state: BoardState, is_closed: bool) -> BoardState {
    if is_closed && state != BoardState::Accepted && state != BoardState::Retired {
        return BoardState::Retired;
    }
    state
}

#[test]
fn board_position_agrees_across_full_cross_product() {
    let states = [
        BoardState::Icebox,
        BoardState::Ready,
        BoardState::InProgress(Stage::Dev.id()),
        BoardState::InProgress(Stage::DeskCheck.id()),
        BoardState::Acceptance,
        BoardState::Accepted,
    ];
    for state in states {
        for is_closed in [false, true] {
            let rust_position = board_position(&state, is_closed);
            let renderer_position = renderer_board_position(state.clone(), is_closed);
            assert_eq!(
                rust_position, renderer_position,
                "drift: state={state:?} is_closed={is_closed} \
                 — Rust board_position = {rust_position:?}, renderer columnFor (mirrored) = {renderer_position:?}"
            );
        }
    }
}

#[test]
fn closed_non_accepted_stories_project_to_retired() {
    // #1146 slice 2b: close_story for retire/dedupe/won't-fix lands in
    // the Retired column rather than polluting Accepted's success-
    // narrative. Closed + Accepted stays in Accepted (user-story
    // satisfied + closed = legitimate completion).
    for state in [
        BoardState::Icebox,
        BoardState::Ready,
        BoardState::InProgress(Stage::Dev.id()),
        BoardState::InProgress(Stage::DeskCheck.id()),
        BoardState::Acceptance,
    ] {
        assert_eq!(
            board_position(&state, true),
            BoardState::Retired,
            "closed + state={state:?} (non-Accepted) must project to RETIRED"
        );
    }
}

#[test]
fn closed_accepted_stays_accepted() {
    assert_eq!(
        board_position(&BoardState::Accepted, true),
        BoardState::Accepted,
        "closed + Accepted stays in ACCEPTED (user-story satisfied + closed = success-narrative)"
    );
}

#[test]
fn open_workflow_states_bucket_one_to_one() {
    // #1146 slice 3: post-collapse, the 6 enum states map 1:1 to
    // their display columns. Rework lives in transition history,
    // not in separate Rejected/NeedsFixes/ReadyForAccept variants.
    assert_eq!(
        board_position(&BoardState::Icebox, false),
        BoardState::Icebox
    );
    assert_eq!(board_position(&BoardState::Ready, false), BoardState::Ready);
    assert_eq!(
        board_position(&BoardState::InProgress(Stage::Dev.id()), false),
        BoardState::InProgress(Stage::Dev.id())
    );
    assert_eq!(
        board_position(&BoardState::InProgress(Stage::DeskCheck.id()), false),
        BoardState::InProgress(Stage::DeskCheck.id())
    );
    assert_eq!(
        board_position(&BoardState::Acceptance, false),
        BoardState::Acceptance
    );
    assert_eq!(
        board_position(&BoardState::Accepted, false),
        BoardState::Accepted
    );
}
