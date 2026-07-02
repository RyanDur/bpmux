use super::action::StoryAction;
use super::reducers::Stories;
use crate::product_management::board::adapters::github::project_labels;
use crate::product_management::board::events::{LabelDerivedFields, StoryEvent};
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

fn project_label_delta(stories: &Stories, event: &StoryEvent) -> Option<LabelDerivedFields> {
    let new_labels = match event {
        StoryEvent::TagAdded { id, label, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.raw_labels().to_vec();
            if !labels.iter().any(|l| l == label) {
                labels.push(label.clone());
            }
            labels
        }
        StoryEvent::TagRemoved { id, label, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.raw_labels().to_vec();
            labels.retain(|l| l != label);
            labels
        }
        _ => return None,
    };
    Some(project_labels(&new_labels))
}
