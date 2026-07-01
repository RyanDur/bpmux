use super::action::StoryAction;
use super::reducers::Stories;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::adapters::github::project_labels;
use crate::product_management::board::events::{BoardEvent, LabelDerivedFields};
use crate::product_management::story::StoryKey;
use crate::store::kernel::Middleware;

pub(crate) struct StoryLabelMiddleware;

impl Middleware<Stories, StoryAction> for StoryLabelMiddleware {
    fn transform(&self, state: &Stories, action: StoryAction) -> StoryAction {
        let StoryAction::Event(event) = &action else {
            return action;
        };
        match project_label_delta(state, event) {
            Some(fields) => StoryAction::RawLabelsAssigned {
                id: event.story_id(),
                fields,
            },
            None => action,
        }
    }
}

fn project_label_delta(stories: &Stories, event: &BoardEvent) -> Option<LabelDerivedFields> {
    let new_labels = match event {
        BoardEvent::TagAdded { id, label, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.raw_labels().to_vec();
            if !labels.iter().any(|l| l == label) {
                labels.push(label.clone());
            }
            labels
        }
        BoardEvent::TagRemoved { id, label, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.raw_labels().to_vec();
            labels.retain(|l| l != label);
            labels
        }
        BoardEvent::CurrentIterationDesignated { id, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.raw_labels().to_vec();
            if !labels
                .iter()
                .any(|l| GhLabelCodec.is_current_iteration_label(l))
            {
                labels.push(GhLabelCodec.encode_current_iteration());
            }
            labels
        }
        BoardEvent::CurrentIterationCleared { id, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.raw_labels().to_vec();
            labels.retain(|l| !GhLabelCodec.is_current_iteration_label(l));
            labels
        }
        BoardEvent::IterationMembershipSet {
            id,
            iteration_id,
            position,
            ..
        } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.raw_labels().to_vec();
            labels.retain(|l| GhLabelCodec.decode_parent_iteration(l).is_none());
            labels.push(GhLabelCodec.encode_parent_iteration(iteration_id.value(), *position));
            labels
        }
        BoardEvent::IterationMembershipCleared { id, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.raw_labels().to_vec();
            labels.retain(|l| GhLabelCodec.decode_parent_iteration(l).is_none());
            labels
        }
        _ => return None,
    };
    Some(project_labels(&new_labels))
}
