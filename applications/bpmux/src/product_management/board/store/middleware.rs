use super::apply_events::Stories;
use crate::product_management::board::adapters::github::project_labels;
use crate::product_management::board::events::{BoardEvent, LabelDerivedFields};

pub(super) fn project_label_delta(
    stories: &Stories,
    event: &BoardEvent,
) -> Option<LabelDerivedFields> {
    let new_labels = match event {
        BoardEvent::TagAdded { number, label, .. } => {
            let mut labels = stories.get(number)?.raw_labels.clone();
            if !labels.iter().any(|l| l == label) {
                labels.push(label.clone());
            }
            labels
        }
        BoardEvent::TagRemoved { number, label, .. } => {
            let mut labels = stories.get(number)?.raw_labels.clone();
            labels.retain(|l| l != label);
            labels
        }
        _ => return None,
    };
    Some(project_labels(&new_labels))
}
