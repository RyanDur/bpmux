use std::sync::Arc;

use crate::hub;
use crate::message::{RosterMemberWire, StateDeltaPayload};
use crate::transport::state_delta::broadcast_state_delta;

use super::{RosterBroadcastListener, RosterMember};

pub struct RosterHubBroadcaster {
    hub: Arc<hub::Hub>,
}

impl RosterHubBroadcaster {
    pub fn new(hub: Arc<hub::Hub>) -> Self {
        Self { hub }
    }
}

impl RosterBroadcastListener for RosterHubBroadcaster {
    fn on_changed(&self, members: &[RosterMember]) {
        broadcast_state_delta(
            &self.hub,
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
            role: m.role.into(),
            presence: m.presence.into(),
        })
        .collect()
}
