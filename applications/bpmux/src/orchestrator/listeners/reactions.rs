use std::sync::Arc;

use crate::orchestrator::{AggregatedReaction, ReactionsBroadcastListener};
use crate::session;
use crate::session::message::{ReactionAggregateWire, StateDeltaPayload};

use super::broadcast_state_delta;

pub struct ReactionsSessionBroadcaster {
    session: Arc<session::Session>,
}

impl ReactionsSessionBroadcaster {
    pub fn new(session: Arc<session::Session>) -> Self {
        Self { session }
    }
}

impl ReactionsBroadcastListener for ReactionsSessionBroadcaster {
    fn on_changed(&self, channel: &str, aggregated: &[AggregatedReaction]) {
        broadcast_state_delta(
            &self.session,
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
