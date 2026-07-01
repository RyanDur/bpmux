use super::InnerState;
use crate::product_management::board::events::StoryEvent;
use crate::product_management::story::{FreeComment, GateComment};

pub(super) fn apply_push_event(s: &mut InnerState, event: &StoryEvent) {
    match event {
        StoryEvent::FreeCommentAppended { id, body, meta } => {
            let entry = FreeComment {
                body: body.clone(),
                author: meta.actor.clone(),
                ts: meta.ts,
            };
            s.free_comments.entry(id.value()).or_default().push(entry);
        }
        StoryEvent::GateCommentAppended {
            id,
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
            s.gate_comments.entry(id.value()).or_default().push(entry);
        }
        StoryEvent::BodyFetched { id, body, .. } => {
            s.bodies.insert(id.value(), body.clone());
        }
        StoryEvent::TagAdded { id, label, .. } => {
            let labels = s.labels.entry(id.value()).or_default();
            if !labels.iter().any(|l| l.eq_ignore_ascii_case(label)) {
                labels.push(label.clone());
            }
            if label.eq_ignore_ascii_case("blocked")
                && let Some(story) = s.stories.get_mut(&id.value())
                && !story
                    .raw_labels
                    .iter()
                    .any(|l| l.eq_ignore_ascii_case("blocked"))
            {
                story.raw_labels.push("blocked".to_string());
            }
        }
        StoryEvent::TagRemoved { id, label, .. } => {
            if let Some(labels) = s.labels.get_mut(&id.value()) {
                labels.retain(|l| !l.eq_ignore_ascii_case(label));
            }
            if label.eq_ignore_ascii_case("blocked")
                && let Some(story) = s.stories.get_mut(&id.value())
            {
                story
                    .raw_labels
                    .retain(|l| !l.eq_ignore_ascii_case("blocked"));
            }
        }
        StoryEvent::StoryClosed { id, .. } => {
            if let Some(story) = s.stories.get_mut(&id.value()) {
                story.is_closed = true;
            }
        }
        StoryEvent::StoryReopened { id, .. } => {
            if let Some(story) = s.stories.get_mut(&id.value()) {
                story.is_closed = false;
            }
        }
        StoryEvent::StateTransitioned { id, new_state, .. } => {
            if let Some(story) = s.stories.get_mut(&id.value()) {
                story.state = *new_state;
            }
        }
        StoryEvent::CurrentIterationDesignated { id, .. } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            if let Some(story) = s.stories.get_mut(&id.value())
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
        StoryEvent::CurrentIterationCleared { id, .. } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            if let Some(story) = s.stories.get_mut(&id.value()) {
                story
                    .raw_labels
                    .retain(|l| !GhLabelCodec.is_current_iteration_label(l));
            }
        }
        StoryEvent::StoryCreated {
            id,
            title,
            body,
            initial_state,
            story_type,
            meta,
        } => {
            let story = s.stories.entry(id.value()).or_insert_with(|| {
                crate::product_management::story::Story::new_minimal(
                    *id,
                    title.clone(),
                    *initial_state,
                    meta.ts,
                    meta.actor.clone(),
                )
            });
            story.state = *initial_state;
            story.title = title.clone();
            story.story_type = *story_type;
            if !body.is_empty() {
                story.body = Some(body.clone());
            }
        }
        StoryEvent::AcceptorAssigned { id, acceptor, .. } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            if let Some(story) = s.stories.get_mut(&id.value()) {
                story
                    .raw_labels
                    .retain(|l| GhLabelCodec.decode_acceptor(l).is_none());
                story
                    .raw_labels
                    .push(GhLabelCodec.encode_acceptor(*acceptor).to_string());
            }
        }
        StoryEvent::ParentEpicSet { id, epic_id, .. } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            use crate::product_management::board::label_codec::LabelCodec;
            if let Some(story) = s.stories.get_mut(&id.value()) {
                story
                    .raw_labels
                    .retain(|l| GhLabelCodec.decode_parent_epic(l).is_none());
                story
                    .raw_labels
                    .push(GhLabelCodec.encode_parent_epic(epic_id.value()));
            }
        }
        StoryEvent::ParentEpicUnset { id, .. } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            if let Some(story) = s.stories.get_mut(&id.value()) {
                story
                    .raw_labels
                    .retain(|l| GhLabelCodec.decode_parent_epic(l).is_none());
            }
        }
        StoryEvent::IterationMembershipSet {
            id,
            iteration_id,
            position,
            ..
        } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            if let Some(story) = s.stories.get_mut(&id.value()) {
                story
                    .raw_labels
                    .retain(|l| GhLabelCodec.decode_parent_iteration(l).is_none());
                story
                    .raw_labels
                    .push(GhLabelCodec.encode_parent_iteration(iteration_id.value(), *position));
            }
        }
        StoryEvent::IterationMembershipCleared { id, .. } => {
            use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
            if let Some(story) = s.stories.get_mut(&id.value()) {
                story
                    .raw_labels
                    .retain(|l| GhLabelCodec.decode_parent_iteration(l).is_none());
            }
        }
        _ => {}
    }
}
