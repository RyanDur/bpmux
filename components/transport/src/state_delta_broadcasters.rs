use std::sync::Arc;

use crate::state_delta::{broadcast_state_delta, broadcast_state_delta_ephemeral};
use ci::slice::{CiBroadcastListener, CiSnapshot};
use message::{CiSnapshotWire, ReactionAggregateWire, StateDeltaPayload};
use reactions::slice::{AggregatedReaction, ReactionsBroadcastListener};
use roster::{RosterBroadcastListener, RosterMember};
use store::Select;

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
        broadcast_state_delta_ephemeral(
            &self.hub,
            StateDeltaPayload::Roster {
                members: roster::members_wire(members, |routing_id| {
                    self.select
                        .select(|state| store::routing::role_for(&state.routing, routing_id))
                }),
            },
        );
    }
}

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

pub struct CiHubBroadcaster {
    hub: Arc<hub::Hub>,
    actions_url: ci::ActionsUrl,
}

impl CiHubBroadcaster {
    pub fn new(hub: Arc<hub::Hub>, actions_url: ci::ActionsUrl) -> Self {
        Self { hub, actions_url }
    }
}

impl CiBroadcastListener for CiHubBroadcaster {
    fn on_changed(&self, snap: &CiSnapshot) {
        let event = snap.to_ci_event();
        broadcast_state_delta(
            &self.hub,
            StateDeltaPayload::Ci {
                snapshot: ci_to_wire(snap, &self.actions_url),
            },
            "ci_listener",
        );
        let Some(text) = ci::compute_broadcast_text(&event) else {
            return;
        };
        let multi_depth = ci::build_multi_depth_for_transition(&event);
        let activity_msg = message::Message {
            from: "bpmux".to_string(),
            text: text.clone(),
            ts: chrono::Utc::now(),
            message_type: message::MessageType::Activity,
            multi_depth: Some(multi_depth),
            ..message::Message::default()
        };
        if let Err(e) = self.hub.broadcast(&activity_msg) {
            diag::log_diag(&format!("ci_listener: activity broadcast failed: {e}"));
        }
        let dev_signal = ci::build_dev_ci_signal(&text);
        if let Err(e) = self.hub.broadcast(&dev_signal) {
            diag::log_diag(&format!("ci_listener: dev signal failed: {e}"));
        }
    }
}

fn ci_to_wire(snap: &CiSnapshot, actions_url: &ci::ActionsUrl) -> CiSnapshotWire {
    CiSnapshotWire {
        state: snap.state.into(),
        run_id: snap.run_id,
        head_sha: snap.head_sha.clone(),
        display_title: snap.display_title.clone(),
        last_successful_poll_at: None,
        url: Some(actions_url.as_str().to_string()),
    }
}
