use std::sync::Arc;

use crate::ci::RunEntry;
use crate::orchestrator::{
    AggregatedReaction, CiBroadcastListener, CiSnapshot, PinnedMessage, PinsBroadcastListener,
    ReactionsBroadcastListener, RosterBroadcastListener, RosterMember,
};
use crate::session;
use crate::session::message::{
    CiSnapshotWire, PinnedMessageWire, ReactionAggregateWire, RosterMemberWire, StateDeltaPayload,
};
use crate::transport::ci_state_store::CiStateStore;

pub struct CiStateStoreWriter {
    store: CiStateStore,
}

impl CiStateStoreWriter {
    pub fn new(store: CiStateStore) -> Self {
        Self { store }
    }
}

impl CiBroadcastListener for CiStateStoreWriter {
    fn on_changed(&self, snap: &CiSnapshot) {
        let runs = vec![RunEntry {
            status: snap.conclusion.to_string(),
            conclusion: Some(snap.conclusion.to_string()),
            head_sha: snap.head_sha.clone(),
            display_title: snap.display_title.clone(),
            database_id: snap.run_id,
            name: None,
            annotation_message: None,
        }];
        let report = crate::bridge::tools::build_ci_state_report(&runs);
        self.store.write(report);
    }
}

pub struct CiSessionBroadcaster {
    session: Arc<session::Session>,
}

impl CiSessionBroadcaster {
    pub fn new(session: Arc<session::Session>) -> Self {
        Self { session }
    }
}

impl CiBroadcastListener for CiSessionBroadcaster {
    fn on_changed(&self, snap: &CiSnapshot) {
        let event = snap.to_ci_event();
        broadcast_state_delta(
            &self.session,
            StateDeltaPayload::Ci {
                snapshot: ci_to_wire(snap),
            },
            "ci_listener",
        );
        let Some(text) = crate::ci::compute_broadcast_text(&event) else {
            return;
        };
        let multi_depth = crate::ci::build_multi_depth_for_transition(&event);
        let activity_msg = session::Message {
            from: "bpmux".to_string(),
            text: text.clone(),
            ts: chrono::Utc::now(),
            message_type: session::MessageType::Activity,
            multi_depth: Some(multi_depth),
            ..session::Message::default()
        };
        if let Err(e) = self.session.broadcast(&activity_msg) {
            session::diag::log_diag(&format!("ci_listener: activity broadcast failed: {e}"));
        }
        let dev_signal = crate::ci::build_dev_ci_signal(&text);
        if let Err(e) = self.session.broadcast(&dev_signal) {
            session::diag::log_diag(&format!("ci_listener: dev signal failed: {e}"));
        }
    }
}

fn broadcast_state_delta(session: &session::Session, payload: StateDeltaPayload, label: &str) {
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(payload),
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!("{label}: state-delta broadcast failed: {e}"));
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

fn pins_to_wire(pins: &[PinnedMessage]) -> Vec<PinnedMessageWire> {
    pins.iter()
        .map(|p| PinnedMessageWire {
            message_key: p.message_key.clone(),
            pinner: p.pinner.clone(),
            ts: p.ts.clone(),
            text: p.text.clone(),
            from: p.from.clone(),
            pinned_at: p.pinned_at.clone(),
        })
        .collect()
}

fn ci_to_wire(snap: &CiSnapshot) -> CiSnapshotWire {
    CiSnapshotWire {
        state: snap.state,
        run_id: snap.run_id,
        head_sha: snap.head_sha.clone(),
        display_title: snap.display_title.clone(),
    }
}

fn roster_to_wire(members: &[RosterMember]) -> Vec<RosterMemberWire> {
    members
        .iter()
        .map(|m| RosterMemberWire {
            name: m.name.clone(),
            identity: m.identity.clone(),
            role: m.role,
            presence: m.presence,
        })
        .collect()
}

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

pub struct PinsSessionBroadcaster {
    session: Arc<session::Session>,
}

impl PinsSessionBroadcaster {
    pub fn new(session: Arc<session::Session>) -> Self {
        Self { session }
    }
}

impl PinsBroadcastListener for PinsSessionBroadcaster {
    fn on_changed(&self, channel: &str, pins: &[PinnedMessage]) {
        broadcast_state_delta(
            &self.session,
            StateDeltaPayload::Pins {
                channel: channel.to_string(),
                pins: pins_to_wire(pins),
            },
            "pins_listener",
        );
    }
}

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
