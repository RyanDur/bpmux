use serde::Serialize;

use crate::routing::short_name;

mod listeners;
mod wire;

pub use listeners::RosterHubBroadcaster;
pub(crate) use wire::members_wire;

pub(crate) fn roster_role_for_identity_prefix(prefix: &str) -> RosterRole {
    let workflow = crate::workflow::current_workflow();
    let purpose = workflow
        .role
        .iter()
        .find(|r| r.name == prefix)
        .and_then(|r| r.purposes.first().map(String::as_str));
    match purpose {
        Some("designer") => RosterRole::Designer,
        Some("dev") => RosterRole::Dev,
        Some("pm") => RosterRole::Pm,
        Some("owner") => RosterRole::Owner,
        _ => RosterRole::Other,
    }
}

pub(crate) fn is_reserved_display(identity: &str) -> bool {
    matches!(
        roster_role_for_identity_prefix(short_name(identity)),
        RosterRole::Owner
    )
}

pub(crate) fn display_name(identity: &str) -> &str {
    short_name(identity)
}

pub(crate) fn role_of(identity: &str) -> RosterRole {
    roster_role_for_identity_prefix(short_name(identity))
}

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

impl From<RosterRole> for crate::message::RosterRoleWire {
    fn from(role: RosterRole) -> Self {
        use crate::message::RosterRoleWire;
        match role {
            RosterRole::Designer => RosterRoleWire::Designer,
            RosterRole::Dev => RosterRoleWire::Dev,
            RosterRole::Pm => RosterRoleWire::Pm,
            RosterRole::Owner => RosterRoleWire::Owner,
            RosterRole::Other => RosterRoleWire::Other,
        }
    }
}

impl From<Presence> for crate::message::PresenceWire {
    fn from(presence: Presence) -> Self {
        use crate::message::PresenceWire;
        match presence {
            Presence::Online => PresenceWire::Online,
            Presence::Busy => PresenceWire::Busy,
            Presence::Idle => PresenceWire::Idle,
            Presence::Offline => PresenceWire::Offline,
            Presence::Blocked => PresenceWire::Blocked,
            Presence::Help => PresenceWire::Help,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RosterMember {
    pub routing_id: String,
    pub identity: String,
    pub presence: Presence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RosterAction {
    RosterMemberUpserted {
        routing_id: String,
        identity: String,
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

pub fn reduce(state: &RosterState, action: &RosterAction) -> Option<RosterState> {
    match action {
        RosterAction::RosterMemberUpserted {
            routing_id,
            identity,
            presence,
        } => {
            let upserted = RosterMember {
                routing_id: routing_id.clone(),
                identity: identity.clone(),
                presence: *presence,
            };
            match state.members.iter().find(|m| &m.routing_id == routing_id) {
                Some(existing) if existing == &upserted => None,
                Some(_) => Some(RosterState {
                    members: state
                        .members
                        .iter()
                        .map(|m| {
                            if &m.routing_id == routing_id {
                                upserted.clone()
                            } else {
                                m.clone()
                            }
                        })
                        .collect(),
                }),
                None => {
                    let mut members = state.members.clone();
                    members.push(upserted);
                    Some(RosterState { members })
                }
            }
        }
        RosterAction::RosterMemberRemoved { routing_id } => {
            if !state.members.iter().any(|m| &m.routing_id == routing_id) {
                return None;
            }
            Some(RosterState {
                members: state
                    .members
                    .iter()
                    .filter(|m| &m.routing_id != routing_id)
                    .cloned()
                    .collect(),
            })
        }
        RosterAction::RosterPresenceChanged {
            routing_id,
            presence,
        } => {
            let existing = state.members.iter().find(|m| &m.routing_id == routing_id)?;
            if existing.presence == *presence {
                return None;
            }
            Some(RosterState {
                members: state
                    .members
                    .iter()
                    .map(|m| {
                        if &m.routing_id == routing_id {
                            RosterMember {
                                presence: *presence,
                                ..m.clone()
                            }
                        } else {
                            m.clone()
                        }
                    })
                    .collect(),
            })
        }
    }
}

pub fn members(state: &RosterState) -> Vec<RosterMember> {
    state.members.clone()
}

pub trait RosterBroadcastListener: Send + Sync {
    fn on_changed(&self, members: &[RosterMember]);
}
