use std::sync::Mutex;

use serde::Serialize;

use crate::store::StoreReduceOutcome;

mod listeners;
mod middleware;

pub use listeners::RosterSessionBroadcaster;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub enum RosterRole {
    Designer,
    Dev,
    Pm,
    Owner,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub enum Presence {
    Online,
    Busy,
    Idle,
    Offline,
    Blocked,
    Help,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RosterMember {
    pub routing_id: String,
    pub name: String,
    pub identity: String,
    pub role: RosterRole,
    pub presence: Presence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RosterAction {
    RosterMemberUpserted {
        routing_id: String,
        name: String,
        identity: String,
        role: RosterRole,
        presence: Presence,
    },
    RosterMemberRemoved {
        routing_id: String,
    },
    RosterPresenceChanged {
        routing_id: String,
        presence: Presence,
    },
}

#[derive(Default)]
pub struct RosterStore {
    inner: Mutex<RosterState>,
}

#[derive(Default, Clone)]
struct RosterState {
    members: Vec<RosterMember>,
}

impl RosterStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn members(&self) -> Vec<RosterMember> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        state.members.clone()
    }

    pub(crate) fn apply(&self, action: &RosterAction) -> StoreReduceOutcome {
        let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match action {
            RosterAction::RosterMemberUpserted {
                routing_id,
                name,
                identity,
                role,
                presence,
            } => {
                if let Some(idx) = state
                    .members
                    .iter()
                    .position(|m| &m.routing_id == routing_id)
                {
                    let m = &mut state.members[idx];
                    if m.name == *name
                        && m.identity == *identity
                        && m.role == *role
                        && m.presence == *presence
                    {
                        return StoreReduceOutcome::Unchanged;
                    }
                    m.name = name.clone();
                    m.identity = identity.clone();
                    m.role = *role;
                    m.presence = *presence;
                    StoreReduceOutcome::Changed
                } else {
                    state.members.push(RosterMember {
                        routing_id: routing_id.clone(),
                        name: name.clone(),
                        identity: identity.clone(),
                        role: *role,
                        presence: *presence,
                    });
                    StoreReduceOutcome::Changed
                }
            }
            RosterAction::RosterMemberRemoved { routing_id } => {
                let before = state.members.len();
                state.members.retain(|m| &m.routing_id != routing_id);
                if state.members.len() == before {
                    StoreReduceOutcome::Unchanged
                } else {
                    StoreReduceOutcome::Changed
                }
            }
            RosterAction::RosterPresenceChanged {
                routing_id,
                presence,
            } => {
                let Some(m) = state
                    .members
                    .iter_mut()
                    .find(|m| &m.routing_id == routing_id)
                else {
                    return StoreReduceOutcome::Unchanged;
                };
                if m.presence == *presence {
                    return StoreReduceOutcome::Unchanged;
                }
                m.presence = *presence;
                StoreReduceOutcome::Changed
            }
        }
    }
}

pub trait RosterBroadcastListener: Send + Sync {
    fn on_changed(&self, members: &[RosterMember]);
}
