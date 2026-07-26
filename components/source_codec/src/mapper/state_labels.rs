use product_management::scheduling::ProjectedState;
use product_management::story::{Stage, Work};
use workflow::StageId;

pub fn stage_github_label(stage: Stage) -> &'static str {
    match stage {
        Stage::Dev => "in progress",
        Stage::DeskCheck => "desk-check",
    }
}

pub fn warn_on_inconsistent_state_labels(labels: &[String]) {
    let state_labels = [
        ProjectedState::Ready.github_label(),
        stage_github_label(Stage::Dev),
        stage_github_label(Stage::DeskCheck),
        ProjectedState::Acceptance.github_label(),
        ProjectedState::Accepted.github_label(),
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
        diag::log_diag(&format!(
            "decoder: inconsistent state-labels present {present:?} — priority decoder picks first per spec; expected: only one state-label per story"
        ));
    }
}

pub fn state_from_labels(labels: &[String], is_closed: bool) -> Work {
    resolve_state_and_placement(labels, is_closed).0
}

pub fn placement_from_labels(labels: &[String], is_closed: bool) -> Option<StageId> {
    resolve_state_and_placement(labels, is_closed).1
}

pub(super) fn resolve_state_and_placement(
    labels: &[String],
    is_closed: bool,
) -> (Work, Option<StageId>) {
    let has = |needle: &str| labels.iter().any(|l| l.eq_ignore_ascii_case(needle));
    warn_on_inconsistent_state_labels(labels);
    if has(ProjectedState::Accepted.github_label()) {
        return (Work::Accepted, None);
    }
    if has(ProjectedState::Retired.github_label()) {
        return (Work::Retired, None);
    }
    if is_closed {
        return (Work::Accepted, None);
    }
    if has("rejected") {
        return (Work::Ready, None);
    }
    if has("needs fixes") {
        return (Work::InProgress, None);
    }
    if has("ready for accept") {
        return (Work::Acceptance, None);
    }
    if has(ProjectedState::Acceptance.github_label()) {
        return (Work::Acceptance, None);
    }
    if has(Stage::DeskCheck.id().as_str()) {
        return (Work::InProgress, Some(Stage::DeskCheck.id()));
    }
    if has(stage_github_label(Stage::Dev)) {
        return (Work::InProgress, None);
    }
    (Work::Ready, None)
}

pub fn outbound_state_label(
    state: &Work,
    member_of_current_iteration: bool,
) -> Option<&'static str> {
    match state {
        Work::Ready if member_of_current_iteration => Some(ProjectedState::Ready.github_label()),
        other => other.github_label(),
    }
}
