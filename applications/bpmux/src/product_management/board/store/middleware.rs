use super::reducers::Stories;
use super::{BoardAction, BoardState};
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::adapters::github::project_labels;
use crate::product_management::board::events::{BoardEvent, LabelDerivedFields};
use crate::product_management::board::label_codec::LabelCodec;
use crate::product_management::story::StoryKey;
use crate::store::kernel::Middleware;

pub(super) struct BoardLabelMiddleware;

impl Middleware<BoardState, BoardAction> for BoardLabelMiddleware {
    fn transform(&self, state: &BoardState, action: BoardAction) -> BoardAction {
        let BoardAction::Event(event) = &action else {
            return action;
        };
        match project_label_delta(&state.stories, event) {
            Some(fields) => BoardAction::RawLabelsAssigned {
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
            let mut labels = stories.get(&StoryKey::Saved(*id))?.current_labels();
            if !labels.iter().any(|l| l == label) {
                labels.push(label.clone());
            }
            labels
        }
        BoardEvent::TagRemoved { id, label, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.current_labels();
            labels.retain(|l| l != label);
            labels
        }
        BoardEvent::CurrentIterationDesignated { id, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.current_labels();
            if !labels
                .iter()
                .any(|l| GhLabelCodec.is_current_iteration_label(l))
            {
                labels.push(GhLabelCodec.encode_current_iteration());
            }
            labels
        }
        BoardEvent::CurrentIterationCleared { id, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.current_labels();
            labels.retain(|l| !GhLabelCodec.is_current_iteration_label(l));
            labels
        }
        BoardEvent::IterationMembershipSet {
            id,
            iteration_number,
            position,
            ..
        } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.current_labels();
            labels.retain(|l| GhLabelCodec.decode_parent_iteration(l).is_none());
            labels.push(GhLabelCodec.encode_parent_iteration(*iteration_number, *position));
            labels
        }
        BoardEvent::IterationMembershipCleared { id, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.current_labels();
            labels.retain(|l| GhLabelCodec.decode_parent_iteration(l).is_none());
            labels
        }
        BoardEvent::ParentEpicSet {
            id, epic_number, ..
        } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.current_labels();
            labels.retain(|l| GhLabelCodec.decode_parent_epic(l).is_none());
            labels.push(GhLabelCodec.encode_parent_epic(*epic_number));
            labels
        }
        BoardEvent::ParentEpicUnset { id, .. } => {
            let mut labels = stories.get(&StoryKey::Saved(*id))?.current_labels();
            labels.retain(|l| GhLabelCodec.decode_parent_epic(l).is_none());
            labels
        }
        _ => return None,
    };
    Some(project_labels(&new_labels))
}
