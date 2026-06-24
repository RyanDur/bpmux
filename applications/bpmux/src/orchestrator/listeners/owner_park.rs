use std::sync::Arc;

use crate::orchestrator::RosterMember;
use crate::orchestrator::ci::{CiPollFetch, Orchestrator, OrchestratorAction};
use crate::orchestrator::stores::{Presence, RosterAction};
use crate::product_management::board::domain::{Acceptor, StoryState};
use crate::product_management::board::iteration_label::current_iteration_number;
use crate::product_management::board::{BoardEvent, BoardStore, Story};

fn ready_queue_is_empty(stories: &[Story]) -> bool {
    let current = current_iteration_number(stories);
    !stories.iter().any(|s| {
        s.state == StoryState::Ready
            && match (current, s.iteration_membership) {
                (Some(ci), Some((it, _))) => it == ci,
                (None, _) => true,
                _ => false,
            }
    })
}

fn any_owner_parked_delivery(stories: &[Story]) -> bool {
    stories
        .iter()
        .any(|s| s.state == StoryState::Delivered && s.acceptor == Some(Acceptor::Owner))
}

pub fn owner_park_presence(
    stories: &[Story],
    roster: &[RosterMember],
    delivering_role: Option<&str>,
) -> Vec<(String, Presence)> {
    let parked = any_owner_parked_delivery(stories) && ready_queue_is_empty(stories);
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

pub struct OwnerParkBroadcaster<T: CiPollFetch> {
    orchestrator: Arc<Orchestrator<T>>,
    store: Arc<BoardStore>,
}

impl<T: CiPollFetch> OwnerParkBroadcaster<T> {
    pub fn new(orchestrator: Arc<Orchestrator<T>>, store: Arc<BoardStore>) -> Self {
        Self {
            orchestrator,
            store,
        }
    }

    fn delivering_role(event: &BoardEvent) -> Option<String> {
        match event {
            BoardEvent::StateTransitioned {
                meta,
                new_state: StoryState::Delivered,
                ..
            } => Some(crate::transport::ws::short_name(&meta.actor).to_string()),
            _ => None,
        }
    }

    async fn dispatch_changes(&self, delivering: Option<&str>) {
        let stories = self.store.all_stories();
        let roster = self.orchestrator.roster_store().members();
        let changes = owner_park_presence(&stories, &roster, delivering);
        for (routing_id, presence) in changes {
            self.orchestrator
                .dispatch(OrchestratorAction::Roster(
                    RosterAction::RosterPresenceChanged {
                        routing_id,
                        presence,
                    },
                ))
                .await;
        }
    }

    pub async fn on_board_event(&self, event: &BoardEvent) {
        let delivering = Self::delivering_role(event);
        self.dispatch_changes(delivering.as_deref()).await;
    }

    pub async fn recompute_from_snapshot(&self) {
        self.dispatch_changes(None).await;
    }
}
