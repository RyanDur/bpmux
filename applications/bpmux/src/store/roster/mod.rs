use serde::Serialize;

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

pub fn reduce(state: &RosterState, action: &RosterAction) -> Option<RosterState> {
    match action {
        RosterAction::RosterMemberUpserted {
            routing_id,
            name,
            identity,
            role,
            presence,
        } => {
            let upserted = RosterMember {
                routing_id: routing_id.clone(),
                name: name.clone(),
                identity: identity.clone(),
                role: *role,
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
