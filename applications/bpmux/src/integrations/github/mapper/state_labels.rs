use crate::product_management::story::{BoardState, Stage};
use crate::workflow::StageId;

pub fn warn_on_inconsistent_state_labels(labels: &[String]) {
    let state_labels = [
        BoardState::Ready.github_label(),
        BoardState::InProgress(Stage::Dev.id()).github_label(),
        BoardState::InProgress(Stage::DeskCheck.id()).github_label(),
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
        return (BoardState::Ready, None);
    }
    if has("needs fixes") {
        return (BoardState::InProgress(Stage::Dev.id()), None);
    }
    if has("ready for accept") {
        return (BoardState::Acceptance, None);
    }
    if has(BoardState::Acceptance.github_label()) {
        return (BoardState::Acceptance, None);
    }
    if has(Stage::DeskCheck.id().as_str()) {
        return (
            BoardState::InProgress(Stage::DeskCheck.id()),
            Some(Stage::DeskCheck.id()),
        );
    }
    if has(BoardState::InProgress(Stage::Dev.id()).github_label()) {
        return (BoardState::InProgress(Stage::Dev.id()), None);
    }
    if has(BoardState::Ready.github_label()) {
        return (BoardState::Ready, None);
    }
    (BoardState::Icebox, None)
}

pub fn state_to_label(state: BoardState) -> &'static str {
    state.github_label()
}

pub fn outbound_state_label(state: &BoardState, placement: Option<&StageId>) -> &'static str {
    match state {
        BoardState::InProgress(_) => {
            let stage = placement.and_then(Stage::from_id).unwrap_or(Stage::Dev);
            BoardState::InProgress(stage.id()).github_label()
        }
        other => other.github_label(),
    }
}
