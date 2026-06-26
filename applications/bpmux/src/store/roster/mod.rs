use serde::Serialize;

use crate::store::kernel::Changed;

mod listeners;

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

#[derive(Default, Clone)]
pub struct RosterState {
    pub members: Vec<RosterMember>,
}

pub fn reduce(state: &mut RosterState, action: &RosterAction) -> Changed {
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
                    return Changed::No;
                }
                m.name = name.clone();
                m.identity = identity.clone();
                m.role = *role;
                m.presence = *presence;
                Changed::Yes
            } else {
                state.members.push(RosterMember {
                    routing_id: routing_id.clone(),
                    name: name.clone(),
                    identity: identity.clone(),
                    role: *role,
                    presence: *presence,
                });
                Changed::Yes
            }
        }
        RosterAction::RosterMemberRemoved { routing_id } => {
            let before = state.members.len();
            state.members.retain(|m| &m.routing_id != routing_id);
            if state.members.len() == before {
                Changed::No
            } else {
                Changed::Yes
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
                return Changed::No;
            };
            if m.presence == *presence {
                return Changed::No;
            }
            m.presence = *presence;
            Changed::Yes
        }
    }
}

pub fn members(state: &RosterState) -> Vec<RosterMember> {
    state.members.clone()
}

pub trait RosterBroadcastListener: Send + Sync {
    fn on_changed(&self, members: &[RosterMember]);
}
