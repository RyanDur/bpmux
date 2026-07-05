#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::{Decision, Verdict, advance};
use crate::product_management::story::{BoardState, Stage};
use crate::workflow::bundled_default::bundled_default;

fn succeed(step: &str) -> Verdict {
    Verdict::Succeed { step: step.into() }
}

fn fail(step: &str) -> Verdict {
    Verdict::Fail { step: step.into() }
}

#[test]
fn succeed_from_ready_advances_to_in_progress_entry() {
    let wf = bundled_default();
    assert_eq!(
        advance(&succeed("ready"), &wf),
        Some(Decision {
            coarse: BoardState::InProgress,
            placement: None
        })
    );
}

#[test]
fn succeed_from_in_progress_advances_to_desk_check_placement() {
    let wf = bundled_default();
    assert_eq!(
        advance(&succeed("in-progress"), &wf),
        Some(Decision {
            coarse: BoardState::InProgress,
            placement: Some(Stage::DeskCheck.id())
        })
    );
}

#[test]
fn succeed_from_desk_check_crosses_to_acceptance() {
    let wf = bundled_default();
    assert_eq!(
        advance(&succeed("desk-check"), &wf),
        Some(Decision {
            coarse: BoardState::Acceptance,
            placement: None
        })
    );
}

#[test]
fn succeed_from_acceptance_crosses_to_accepted() {
    let wf = bundled_default();
    assert_eq!(
        advance(&succeed("acceptance"), &wf),
        Some(Decision {
            coarse: BoardState::Accepted,
            placement: None
        })
    );
}

#[test]
fn fail_from_desk_check_derives_return_to_in_progress_entry() {
    let wf = bundled_default();
    assert_eq!(
        advance(&fail("desk-check"), &wf),
        Some(Decision {
            coarse: BoardState::InProgress,
            placement: None
        })
    );
}

#[test]
fn fail_from_acceptance_derives_return_to_ready() {
    let wf = bundled_default();
    assert_eq!(
        advance(&fail("acceptance"), &wf),
        Some(Decision {
            coarse: BoardState::Ready,
            placement: None
        })
    );
}

#[test]
fn succeed_from_unknown_state_yields_none() {
    let wf = bundled_default();
    assert_eq!(advance(&succeed("nonexistent-state"), &wf), None);
}

#[test]
fn fail_from_state_without_a_backward_transition_yields_none() {
    let wf = bundled_default();
    assert_eq!(advance(&fail("ready"), &wf), None);
}
