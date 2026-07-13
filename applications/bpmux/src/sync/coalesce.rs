use crate::integrations::github::mapper::stage_github_label;
use product_management::story::{Stage, Story};

pub(super) fn stale_state_labels_for(
    story: &Story,
    raw_labels: &[String],
    fine: Option<Stage>,
    iterations: &[product_management::iteration::Iteration],
) -> Vec<String> {
    use product_management::scheduling::ProjectedState;
    let expected = match product_management::scheduling::projected_state(story, iterations) {
        ProjectedState::Icebox => "",
        ProjectedState::InProgress => stage_github_label(fine.unwrap_or(Stage::Dev)),
        other => other.github_label(),
    };
    let canonical_state_labels = [
        ProjectedState::Ready.github_label(),
        stage_github_label(Stage::Dev),
        stage_github_label(Stage::DeskCheck),
        ProjectedState::Acceptance.github_label(),
        ProjectedState::Accepted.github_label(),
    ];
    let legacy_state_labels = crate::integrations::github::gh_label_codec::LEGACY_STATE_LABELS;
    raw_labels
        .iter()
        .filter(|l| {
            let lower = l.to_ascii_lowercase();
            let is_state_label = canonical_state_labels.contains(&lower.as_str())
                || legacy_state_labels.contains(&lower.as_str());
            is_state_label && lower != expected
        })
        .cloned()
        .collect()
}
