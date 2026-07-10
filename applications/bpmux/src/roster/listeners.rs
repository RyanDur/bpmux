use std::sync::Arc;

use crate::hub;
use crate::store::Select;
use crate::transport::state_delta::broadcast_state_delta;
use message::StateDeltaPayload;

use super::{RosterBroadcastListener, RosterMember};

pub struct RosterHubBroadcaster {
    hub: Arc<hub::Hub>,
    select: Select,
}

impl RosterHubBroadcaster {
    pub fn new(hub: Arc<hub::Hub>, select: Select) -> Self {
        Self { hub, select }
    }
}

impl RosterBroadcastListener for RosterHubBroadcaster {
    fn on_changed(&self, members: &[RosterMember]) {
        broadcast_state_delta(
            &self.hub,
            StateDeltaPayload::Roster {
                members: super::members_wire(&self.select, members),
            },
            "roster_listener",
        );
    }
}
