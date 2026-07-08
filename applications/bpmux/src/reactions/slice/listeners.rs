use std::sync::Arc;

use crate::hub;
use crate::message::{ReactionAggregateWire, StateDeltaPayload};
use crate::transport::state_delta::broadcast_state_delta;

use super::{AggregatedReaction, ReactionsBroadcastListener};

pub struct ReactionsHubBroadcaster {
    hub: Arc<hub::Hub>,
}

impl ReactionsHubBroadcaster {
    pub fn new(hub: Arc<hub::Hub>) -> Self {
        Self { hub }
    }
}

impl ReactionsBroadcastListener for ReactionsHubBroadcaster {
    fn on_changed(&self, channel: &str, aggregated: &[AggregatedReaction]) {
        broadcast_state_delta(
            &self.hub,
            StateDeltaPayload::Reactions {
                channel: channel.to_string(),
                aggregated: reactions_to_wire(aggregated),
            },
            "reactions_listener",
        );
    }
}

fn reactions_to_wire(aggregated: &[AggregatedReaction]) -> Vec<ReactionAggregateWire> {
    aggregated
        .iter()
        .map(|a| ReactionAggregateWire {
            message_key: a.message_key.clone(),
            emoji: a.emoji.clone(),
            count: a.count,
            users: a.users.clone(),
        })
        .collect()
}
