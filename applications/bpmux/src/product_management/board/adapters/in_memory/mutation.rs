use super::{InnerState, typed_label_mirror};
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
        }
        StoryEvent::TagRemoved { id, label, .. } => {
            if let Some(labels) = s.labels.get_mut(&id.value()) {
                labels.retain(|l| !l.eq_ignore_ascii_case(label));
            }
        }
        StoryEvent::TagSet { id, tag, .. } => typed_label_mirror::tag_set(s, id.value(), *tag),
        StoryEvent::TagUnset { id, tag, .. } => typed_label_mirror::tag_unset(s, id.value(), *tag),
        StoryEvent::BlockedBySet { id, blocker, .. } => {
            typed_label_mirror::blocked_by_set(s, id.value(), blocker.value())
        }
        StoryEvent::BlockedByCleared { id, .. } => {
            typed_label_mirror::blocked_by_cleared(s, id.value())
        }
        StoryEvent::IterationSlotSet {
            id, name, position, ..
        } => typed_label_mirror::iteration_slot_set(s, id.value(), name, *position),
        StoryEvent::IterationSlotCleared { id, .. } => {
            typed_label_mirror::iteration_slot_cleared(s, id.value())
        }
        StoryEvent::StoryClosed { id, .. } => typed_label_mirror::story_closed(s, id.value()),
        StoryEvent::StoryReopened { id, .. } => {
            if let Some(story) = s.stories.get_mut(&id.value()) {
                story.is_closed = false;
            }
        }
        StoryEvent::StateTransitioned { id, new_state, .. } => {
            typed_label_mirror::state_transitioned(s, id.value(), new_state.clone())
        }
        StoryEvent::CurrentIterationDesignated { id, .. } => {
            typed_label_mirror::current_iteration_designated(s, id.value())
        }
        StoryEvent::CurrentIterationCleared { id, .. } => {
            typed_label_mirror::current_iteration_cleared(s, id.value())
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
                    initial_state.clone(),
                    meta.ts,
                    meta.actor.clone(),
                )
            });
            story.state = initial_state.clone();
            story.title = title.clone();
            story.story_type = *story_type;
            if !body.is_empty() {
                story.body = Some(body.clone());
            }
        }
        StoryEvent::AcceptorAssigned { id, acceptor, .. } => {
            typed_label_mirror::acceptor_assigned(s, id.value(), *acceptor)
        }
        StoryEvent::ParentEpicSet { id, epic_id, .. } => {
            typed_label_mirror::parent_epic_set(s, id.value(), epic_id.value())
        }
        StoryEvent::ParentEpicUnset { id, .. } => {
            typed_label_mirror::parent_epic_unset(s, id.value())
        }
        _ => {}
    }
}
