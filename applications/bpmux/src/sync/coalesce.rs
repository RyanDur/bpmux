use product_management::label_codec::LabelCodec;
use product_management::story::{Stage, Story};

pub(super) fn stale_state_labels_for(
    codec: &dyn LabelCodec,
    story: &Story,
    raw_labels: &[String],
    fine: Option<Stage>,
    iterations: &[product_management::iteration::Iteration],
) -> Vec<String> {
    use product_management::scheduling::ProjectedState;
    let expected = match product_management::scheduling::projected_state(story, iterations) {
        ProjectedState::Icebox => "",
        ProjectedState::InProgress => codec.stage_label(fine.unwrap_or(Stage::Dev)),
        other => other.github_label(),
    };
    let canonical_state_labels = [
        ProjectedState::Ready.github_label(),
        codec.stage_label(Stage::Dev),
        codec.stage_label(Stage::DeskCheck),
        ProjectedState::Acceptance.github_label(),
        ProjectedState::Accepted.github_label(),
    ];
    raw_labels
        .iter()
        .filter(|l| {
            let lower = l.to_ascii_lowercase();
            let is_state_label = canonical_state_labels.contains(&lower.as_str())
                || codec.is_legacy_state_label(&lower);
            is_state_label && lower != expected
        })
        .cloned()
        .collect()
}
