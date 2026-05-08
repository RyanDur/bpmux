use std::sync::Arc;

use crate::bridge::{
    CI_STATE_UPDATE_MARKER, PINS_UPDATE_MARKER, REACTIONS_UPDATE_MARKER, ROSTER_UPDATE_MARKER,
};
use crate::ci::RunEntry;
use crate::orchestrator::{
    AggregatedReaction, CiBroadcastListener, CiSnapshot, PinnedMessage, PinsBroadcastListener,
    ReactionsBroadcastListener, RosterBroadcastListener, RosterMember,
};
use crate::session;
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
        let marker_msg = session::Message {
            from: "bpmux".to_string(),
            text: CI_STATE_UPDATE_MARKER.to_string(),
            ts: chrono::Utc::now(),
            message_type: session::MessageType::System,
            ..session::Message::default()
        };
        if let Err(e) = self.session.broadcast(&marker_msg) {
            session::diag::log_diag(&format!("ci_listener: marker broadcast failed: {e}"));
        }
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

fn broadcast_marker(session: &session::Session, marker: &str, label: &str) {
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: marker.to_string(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::System,
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!("{label}: marker broadcast failed: {e}"));
    }
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
    fn on_changed(&self, _channel: &str, _aggregated: &[AggregatedReaction]) {
        broadcast_marker(&self.session, REACTIONS_UPDATE_MARKER, "reactions_listener");
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
    fn on_changed(&self, _channel: &str, _pins: &[PinnedMessage]) {
        broadcast_marker(&self.session, PINS_UPDATE_MARKER, "pins_listener");
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
    fn on_changed(&self, _members: &[RosterMember]) {
        broadcast_marker(&self.session, ROSTER_UPDATE_MARKER, "roster_listener");
    }
}
