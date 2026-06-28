use super::InnerState;
use crate::product_management::board::events::BoardEvent;
use crate::product_management::story::{FreeComment, GateComment};

pub(super) fn apply_push_event(s: &mut InnerState, event: &BoardEvent) {
    match event {
        BoardEvent::FreeCommentAppended { number, body, meta } => {
            let entry = FreeComment {
                body: body.clone(),
                author: meta.actor.clone(),
                ts: meta.ts,
            };
            s.free_comments.entry(*number).or_default().push(entry);
        }
        BoardEvent::GateCommentAppended {
            number,
            gate,
            payload,
            meta,
        } => {
            let entry = GateComment {
                gate: *gate,
                ts: meta.ts,
                actor: meta.actor.clone(),
                payload: payload.clone(),
            };
            s.gate_comments.entry(*number).or_default().push(entry);
        }
        BoardEvent::BodyFetched { number, body, .. } => {
            s.bodies.insert(*number, body.clone());
        }
        BoardEvent::TagAdded { number, label, .. } => {
            let labels = s.labels.entry(*number).or_default();
            if !labels.iter().any(|l| l.eq_ignore_ascii_case(label)) {
                labels.push(label.clone());
            }
            if label.eq_ignore_ascii_case("blocked")
                && let Some(story) = s.stories.get_mut(number)
                && !story
                    .raw_labels
                    .iter()
                    .any(|l| l.eq_ignore_ascii_case("blocked"))
            {
                story.raw_labels.push("blocked".to_string());
            }
        }
        BoardEvent::TagRemoved { number, label, .. } => {
            if let Some(labels) = s.labels.get_mut(number) {
                labels.retain(|l| !l.eq_ignore_ascii_case(label));
            }
            if label.eq_ignore_ascii_case("blocked")
                && let Some(story) = s.stories.get_mut(number)
            {
                story
                    .raw_labels
                    .retain(|l| !l.eq_ignore_ascii_case("blocked"));
            }
        }
        BoardEvent::StoryClosed { number, .. } => {
            if let Some(story) = s.stories.get_mut(number) {
                story.is_closed = true;
            }
        }
        BoardEvent::StoryReopened { number, .. } => {
            if let Some(story) = s.stories.get_mut(number) {
                story.is_closed = false;
            }
        }
        BoardEvent::StateTransitioned {
            number, new_state, ..
        } => {
            if let Some(story) = s.stories.get_mut(number) {
                story.state = *new_state;
            }
        }
        BoardEvent::CurrentIterationDesignated { number, .. } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            if let Some(story) = s.stories.get_mut(number)
                && !story
                    .raw_labels
                    .iter()
                    .any(|l| GhLabelCodec.is_current_iteration_label(l))
            {
                story
                    .raw_labels
                    .push(GhLabelCodec.encode_current_iteration());
            }
        }
        BoardEvent::CurrentIterationCleared { number, .. } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            if let Some(story) = s.stories.get_mut(number) {
                story
                    .raw_labels
                    .retain(|l| !GhLabelCodec.is_current_iteration_label(l));
            }
        }
        BoardEvent::StoryCreated {
            number,
            initial_state,
            ..
        } => {
            if let Some(story) = s.stories.get_mut(number) {
                story.state = *initial_state;
            }
        }
        BoardEvent::ParentEpicSet {
            number,
            epic_number,
            ..
        } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            use crate::product_management::board::label_codec::LabelCodec;
            if let Some(story) = s.stories.get_mut(number) {
                story
                    .raw_labels
                    .retain(|l| GhLabelCodec.decode_parent_epic(l).is_none());
                story
                    .raw_labels
                    .push(GhLabelCodec.encode_parent_epic(*epic_number));
            }
        }
        BoardEvent::ParentEpicUnset { number, .. } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            if let Some(story) = s.stories.get_mut(number) {
                story
                    .raw_labels
                    .retain(|l| GhLabelCodec.decode_parent_epic(l).is_none());
            }
        }
        BoardEvent::IterationMembershipSet {
            number,
            iteration_number,
            position,
            ..
        } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            if let Some(story) = s.stories.get_mut(number) {
                story
                    .raw_labels
                    .retain(|l| GhLabelCodec.decode_parent_iteration(l).is_none());
                story
                    .raw_labels
                    .push(GhLabelCodec.encode_parent_iteration(*iteration_number, *position));
            }
        }
        BoardEvent::IterationMembershipCleared { number, .. } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            if let Some(story) = s.stories.get_mut(number) {
                story
                    .raw_labels
                    .retain(|l| GhLabelCodec.decode_parent_iteration(l).is_none());
            }
        }
        _ => {}
    }
}
