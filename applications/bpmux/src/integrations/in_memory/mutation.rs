use super::{InnerState, typed_label_mirror};
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::{CastVerdict, FreeComment};

pub(super) fn apply_push_event(
    s: &mut InnerState,
    event: &StoryEvent,
    context: &crate::sync::PushContext,
) {
    match &event.kind {
        StoryEventKind::FreeCommentAppended { body } => {
            let entry = FreeComment {
                body: body.clone(),
                author: event.meta.actor.clone(),
                ts: event.meta.ts,
            };
            s.free_comments
                .entry(event.id.value())
                .or_default()
                .push(entry);
        }
        StoryEventKind::VerdictCast { verdict, payload } => {
            let entry = CastVerdict {
                verdict: verdict.clone(),
                ts: event.meta.ts,
                actor: event.meta.actor.clone(),
                payload: payload.clone(),
            };
            s.verdicts.entry(event.id.value()).or_default().push(entry);
        }
        StoryEventKind::BodyFetched { body, .. } => {
            s.bodies.insert(event.id.value(), body.clone());
        }
        StoryEventKind::TagAdded { label } => {
            let labels = s.labels.entry(event.id.value()).or_default();
            if !labels.iter().any(|l| l.eq_ignore_ascii_case(label)) {
                labels.push(label.clone());
            }
        }
        StoryEventKind::TagRemoved { label } => {
            if let Some(labels) = s.labels.get_mut(&event.id.value()) {
                labels.retain(|l| !l.eq_ignore_ascii_case(label));
            }
        }
        StoryEventKind::TagSet { tag } => typed_label_mirror::tag_set(s, event.id.value(), *tag),
        StoryEventKind::TagUnset { tag } => {
            typed_label_mirror::tag_unset(s, event.id.value(), *tag)
        }
        StoryEventKind::StoryClosed { .. } => typed_label_mirror::story_closed(s, event.id.value()),
        StoryEventKind::StoryReopened => {
            s.closed.remove(&event.id.value());
        }
        StoryEventKind::StateTransitioned { new_state, .. } => {
            typed_label_mirror::state_transitioned(
                s,
                event.id.value(),
                new_state.clone(),
                context.member_of_current_iteration,
            )
        }
        StoryEventKind::CurrentIterationDesignated => {
            typed_label_mirror::current_iteration_designated(s, event.id.value())
        }
        StoryEventKind::CurrentIterationCleared => {
            typed_label_mirror::current_iteration_cleared(s, event.id.value())
        }
        StoryEventKind::StoryCreated {
            title,
            body,
            initial_state,
            story_type,
        } => {
            let story = s.stories.entry(event.id.value()).or_insert_with(|| {
                product_management::story::Story::new_minimal(
                    event.id,
                    title.clone(),
                    initial_state.clone(),
                    event.meta.ts,
                    event.meta.actor.clone(),
                )
            });
            story.state = initial_state.clone();
            story.title = title.clone();
            story.story_type = *story_type;
            if !body.is_empty() {
                story.body = Some(body.clone());
            }
        }
        StoryEventKind::AcceptorAssigned { acceptor } => {
            typed_label_mirror::acceptor_assigned(s, event.id.value(), *acceptor)
        }
        StoryEventKind::ParentEpicSet { epic_id } => {
            typed_label_mirror::parent_epic_set(s, event.id.value(), epic_id.value())
        }
        StoryEventKind::ParentEpicUnset => {
            typed_label_mirror::parent_epic_unset(s, event.id.value())
        }
        StoryEventKind::BodyUpdated { body } => {
            s.bodies.insert(event.id.value(), body.clone());
            if let Some(story) = s.stories.get_mut(&event.id.value()) {
                story.body = Some(body.clone());
            }
        }
        StoryEventKind::TitleUpdated { title } => {
            if let Some(story) = s.stories.get_mut(&event.id.value()) {
                story.title = title.clone();
            }
        }
        _ => {}
    }
}
