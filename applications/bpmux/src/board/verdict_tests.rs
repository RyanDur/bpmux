#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::{Verdict, advance};
use crate::product_management::story::{BoardState, Stage};
use crate::workflow::bundled_default::bundled_default;

#[test]
fn succeed_from_ready_advances_to_in_progress_entry() {
    let wf = bundled_default();
    assert_eq!(
        advance("ready", &Verdict::Succeed, &wf),
        Some((BoardState::InProgress, None))
    );
}

#[test]
fn succeed_from_in_progress_advances_to_desk_check_placement() {
    let wf = bundled_default();
    assert_eq!(
        advance("in-progress", &Verdict::Succeed, &wf),
        Some((BoardState::InProgress, Some(Stage::DeskCheck.id())))
    );
}

#[test]
fn succeed_from_desk_check_crosses_to_acceptance() {
    let wf = bundled_default();
    assert_eq!(
        advance("desk-check", &Verdict::Succeed, &wf),
        Some((BoardState::Acceptance, None))
    );
}

#[test]
fn succeed_from_acceptance_crosses_to_accepted() {
    let wf = bundled_default();
    assert_eq!(
        advance("acceptance", &Verdict::Succeed, &wf),
        Some((BoardState::Accepted, None))
    );
}

#[test]
fn needs_work_from_desk_check_returns_to_in_progress_entry() {
    let wf = bundled_default();
    assert_eq!(
        advance(
            "desk-check",
            &Verdict::NeedsWork(BoardState::InProgress),
            &wf
        ),
        Some((BoardState::InProgress, None))
    );
}

#[test]
fn needs_work_from_acceptance_returns_to_ready() {
    let wf = bundled_default();
    assert_eq!(
        advance("acceptance", &Verdict::NeedsWork(BoardState::Ready), &wf),
        Some((BoardState::Ready, None))
    );
}

#[test]
fn succeed_from_unknown_state_yields_none() {
    let wf = bundled_default();
    assert_eq!(advance("nonexistent-state", &Verdict::Succeed, &wf), None);
}
