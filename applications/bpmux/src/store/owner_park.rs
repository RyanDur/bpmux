use std::sync::Arc;

use crate::product_management::board::{BoardStore, StoryEvent};
use crate::product_management::iteration::Iteration;
use crate::product_management::iteration::label::current_iteration_id;
use crate::product_management::story::{Acceptor, BoardState, Story};
use crate::store::{AppAction, RosterMember, Store};
use crate::store::{Presence, RosterAction};

fn ready_queue_is_empty(stories: &[Story], iterations: &[Iteration]) -> bool {
    let current = current_iteration_id(iterations);
    !stories.iter().any(|s| {
        s.state == BoardState::Ready
            && match (current, s.iteration_membership()) {
                (Some(ci), Some((it, _))) => it == ci,
                (None, _) => true,
                _ => false,
            }
    })
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

pub struct OwnerParkBroadcaster {
    store: Arc<Store>,
    board_store: Arc<BoardStore>,
}

impl OwnerParkBroadcaster {
    pub fn new(store: Arc<Store>, board_store: Arc<BoardStore>) -> Self {
        Self { store, board_store }
    }

    fn delivering_role(event: &StoryEvent) -> Option<String> {
        match event {
            StoryEvent::StateTransitioned {
                meta,
                new_state: BoardState::Delivered,
                ..
            } => Some(crate::transport::ws::short_name(&meta.actor).to_string()),
            _ => None,
        }
    }

    async fn dispatch_changes(&self, delivering: Option<&str>) {
        let stories = self.board_store.all_stories();
        let iterations = self.store.iterations();
        let roster = self.store.roster_members();
        let changes = owner_park_presence(&stories, &iterations, &roster, delivering);
        for (routing_id, presence) in changes {
            self.store
                .dispatch(AppAction::Roster(RosterAction::RosterPresenceChanged {
                    routing_id,
                    presence,
                }))
                .await;
        }
    }

    pub async fn on_board_event(&self, event: &StoryEvent) {
        let delivering = Self::delivering_role(event);
        self.dispatch_changes(delivering.as_deref()).await;
    }

    pub async fn recompute_from_snapshot(&self) {
        self.dispatch_changes(None).await;
    }
}
