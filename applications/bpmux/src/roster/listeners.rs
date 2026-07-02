use std::sync::Arc;

use crate::message::{RosterMemberWire, StateDeltaPayload};
use crate::session;
use crate::transport::state_delta::broadcast_state_delta;

use super::{RosterBroadcastListener, RosterMember};

pub struct RosterSessionBroadcaster {
    session: Arc<session::Session>,
}

impl RosterSessionBroadcaster {
    pub fn new(session: Arc<session::Session>) -> Self {
        Self { session }
    }
}

impl RosterBroadcastListener for RosterSessionBroadcaster {
    fn on_changed(&self, members: &[RosterMember]) {
        broadcast_state_delta(
            &self.session,
            StateDeltaPayload::Roster {
                members: roster_to_wire(members),
            },
            "roster_listener",
        );
    }
}

fn roster_to_wire(members: &[RosterMember]) -> Vec<RosterMemberWire> {
    members
        .iter()
        .map(|m| RosterMemberWire {
            routing_id: m.routing_id.clone(),
            name: m.name.clone(),
            identity: m.identity.clone(),
            role: m.role,
            presence: m.presence,
        })
        .collect()
}
