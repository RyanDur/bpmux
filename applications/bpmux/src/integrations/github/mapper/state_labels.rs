use crate::product_management::story::{BoardState, Stage};
use crate::workflow::StageId;

pub fn stage_github_label(stage: Stage) -> &'static str {
    match stage {
        Stage::Dev => "in progress",
        Stage::DeskCheck => "desk-check",
    }
}

pub fn warn_on_inconsistent_state_labels(labels: &[String]) {
    let state_labels = [
        crate::product_management::scheduling::ProjectedState::Ready.github_label(),
        stage_github_label(Stage::Dev),
        stage_github_label(Stage::DeskCheck),
        BoardState::Acceptance.github_label(),
        BoardState::Accepted.github_label(),
        "ready for accept",
        "needs fixes",
        "rejected",
    ];
    let present: Vec<&str> = state_labels
        .iter()
        .copied()
        .filter(|needle| labels.iter().any(|l| l.eq_ignore_ascii_case(needle)))
        .collect();
    if present.len() >= 2 {
        crate::diag::log_diag(&format!(
            "decoder: inconsistent state-labels present {present:?} — priority decoder picks first per spec; expected: only one state-label per story"
        ));
    }
}

pub fn state_from_labels(labels: &[String], is_closed: bool) -> BoardState {
    resolve_state_and_placement(labels, is_closed).0
}

pub fn placement_from_labels(labels: &[String], is_closed: bool) -> Option<StageId> {
    resolve_state_and_placement(labels, is_closed).1
}

pub(super) fn resolve_state_and_placement(
    labels: &[String],
    is_closed: bool,
) -> (BoardState, Option<StageId>) {
    let has = |needle: &str| labels.iter().any(|l| l.eq_ignore_ascii_case(needle));
    warn_on_inconsistent_state_labels(labels);
    if has(BoardState::Accepted.github_label()) {
        return (BoardState::Accepted, None);
    }
    if has(BoardState::Retired.github_label()) {
        return (BoardState::Retired, None);
    }
    if is_closed {
        return (BoardState::Accepted, None);
    }
    if has("rejected") {
        return (BoardState::NotStarted, None);
    }
    if has("needs fixes") {
        return (BoardState::InProgress, None);
    }
    if has("ready for accept") {
        return (BoardState::Acceptance, None);
    }
    if has(BoardState::Acceptance.github_label()) {
        return (BoardState::Acceptance, None);
    }
    if has(Stage::DeskCheck.id().as_str()) {
        return (BoardState::InProgress, Some(Stage::DeskCheck.id()));
    }
    if has(stage_github_label(Stage::Dev)) {
        return (BoardState::InProgress, None);
    }
    (BoardState::NotStarted, None)
}

pub fn outbound_state_label(
    state: &BoardState,
    member_of_current_iteration: bool,
) -> Option<&'static str> {
    match state {
        BoardState::NotStarted if member_of_current_iteration => {
            Some(crate::product_management::scheduling::ProjectedState::Ready.github_label())
        }
        BoardState::NotStarted => None,
        other => Some(other.github_label()),
    }
}
