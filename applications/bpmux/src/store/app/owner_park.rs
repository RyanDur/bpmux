use crate::product_management::iteration::Iteration;
use crate::product_management::scheduling::{ProjectedState, projected_state};
use crate::product_management::story::{Acceptor, BoardState, Story};
use crate::store::{Presence, RosterMember};

fn ready_queue_is_empty(stories: &[Story], iterations: &[Iteration]) -> bool {
    !stories
        .iter()
        .any(|s| projected_state(s, iterations) == ProjectedState::Ready)
}

fn any_owner_parked_delivery(stories: &[Story]) -> bool {
    stories
        .iter()
        .any(|s| s.state == BoardState::Acceptance && s.acceptor() == Some(Acceptor::Owner))
}

pub fn owner_park_presence(
    stories: &[Story],
    iterations: &[Iteration],
    roster: &[RosterMember],
    delivering_role: Option<&str>,
) -> Vec<(String, Presence)> {
    let parked = any_owner_parked_delivery(stories) && ready_queue_is_empty(stories, iterations);
    let mut changes = Vec::new();
    for member in roster {
        let should_block = parked
            && (Some(member.name.as_str()) == delivering_role
                || member.presence == Presence::Blocked);
        let next = if should_block {
            Presence::Blocked
        } else if member.presence == Presence::Blocked {
            Presence::Online
        } else {
            continue;
        };
        if next != member.presence {
            changes.push((member.routing_id.clone(), next));
        }
    }
    changes
}

pub struct OwnerParkListener;

impl crate::kernel::Listener<crate::store::AppState, crate::store::AppAction>
    for OwnerParkListener
{
    fn on_action(
        &self,
        reaction: crate::kernel::Reaction<crate::store::AppState, crate::store::AppAction>,
    ) -> crate::kernel::ListenerFuture {
        let crate::store::AppAction::Story(story_action) = &reaction.action else {
            return Box::pin(async {});
        };
        let actor_if_delivered =
            |event: &crate::product_management::story::events::StoryEvent| match event {
                crate::product_management::story::events::StoryEvent::StateTransitioned {
                    meta,
                    new_state: BoardState::Acceptance,
                    ..
                } => Some(crate::routing::short_name(&meta.actor).to_string()),
                _ => None,
            };
        let delivering = match story_action {
            crate::product_management::story::slice::StoryAction::Event(event) => {
                actor_if_delivered(event)
            }
            crate::product_management::story::slice::StoryAction::Events(events) => {
                events.iter().rev().find_map(actor_if_delivered)
            }
            _ => None,
        };
        let stories: Vec<Story> = reaction.current.stories.values().cloned().collect();
        let iterations = crate::product_management::iteration::all(&reaction.current.iterations);
        let roster = crate::store::roster::members(&reaction.current.roster);
        let changes = owner_park_presence(&stories, &iterations, &roster, delivering.as_deref());
        let dispatch = std::sync::Arc::clone(&reaction.dispatch);
        Box::pin(async move {
            for (routing_id, presence) in changes {
                dispatch(crate::store::AppAction::Roster(
                    crate::store::RosterAction::RosterPresenceChanged {
                        routing_id,
                        presence,
                    },
                ));
            }
        })
    }
}
