use crate::product_management::iteration::Iteration;
use crate::product_management::scheduling::{ProjectedState, projected_state};
use crate::product_management::story::{Acceptor, Story, Work};
use crate::store::{Presence, RosterMember};

fn ready_queue_is_empty(stories: &[Story], iterations: &[Iteration]) -> bool {
    !stories
        .iter()
        .any(|s| projected_state(s, iterations) == ProjectedState::Ready)
}

fn any_owner_parked_delivery(stories: &[Story]) -> bool {
    stories
        .iter()
        .any(|s| s.state == Work::Acceptance && s.acceptor() == Some(Acceptor::Owner))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivering {
    ById(String),
    ByDisplay(String),
}

fn delivering_matches(delivering: &Delivering, member: &RosterMember) -> bool {
    match delivering {
        Delivering::ById(id) => &member.routing_id == id,
        Delivering::ByDisplay(display) => crate::roster::display_name(&member.identity) == display,
    }
}

pub fn owner_park_presence(
    stories: &[Story],
    iterations: &[Iteration],
    roster: &[RosterMember],
    delivering: Option<&Delivering>,
) -> Vec<(String, Presence)> {
    let parked = any_owner_parked_delivery(stories) && ready_queue_is_empty(stories, iterations);
    let mut changes = Vec::new();
    for member in roster {
        let should_block = parked
            && (delivering.is_some_and(|d| delivering_matches(d, member))
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

impl kernel::Listener<crate::store::AppState, crate::store::AppAction> for OwnerParkListener {
    fn on_action(
        &self,
        reaction: kernel::Reaction<crate::store::AppState, crate::store::AppAction>,
    ) -> kernel::ListenerFuture {
        let crate::store::AppAction::Story(story_action) = &reaction.action else {
            return Box::pin(async {});
        };
        let actor_if_delivered =
            |event: &crate::product_management::story::events::StoryEvent| match &event.kind {
                crate::product_management::story::events::StoryEventKind::StateTransitioned {
                    new_state: Work::Acceptance,
                    ..
                } => Some(match &event.meta.actor_id {
                    Some(id) => Delivering::ById(id.clone()),
                    None => Delivering::ByDisplay(
                        user_routing::short_name(&event.meta.actor).to_string(),
                    ),
                }),
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
        let changes = owner_park_presence(&stories, &iterations, &roster, delivering.as_ref());
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

    fn delivery_mode(&self) -> kernel::DeliveryMode {
        kernel::DeliveryMode::Ordered
    }
}
