use crate::product_management::iteration::Iteration;
use crate::product_management::iteration::label::current_iteration_id;
use crate::product_management::story::{Acceptor, BoardState, Story};
use crate::store::{Presence, RosterMember};

fn ready_queue_is_empty(stories: &[Story], iterations: &[Iteration]) -> bool {
    let current = current_iteration_id(iterations);
    let in_current_iteration = |s: &Story| match current {
        Some(ci) => iterations
            .iter()
            .any(|it| it.id == ci && it.stories.contains(&s.id)),
        None => true,
    };
    !stories
        .iter()
        .any(|s| s.state == BoardState::Ready && in_current_iteration(s))
}

fn any_owner_parked_delivery(stories: &[Story]) -> bool {
    stories
        .iter()
        .any(|s| s.state == BoardState::Delivered && s.acceptor() == Some(Acceptor::Owner))
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
