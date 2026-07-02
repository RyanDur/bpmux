use std::sync::Arc;

use crate::ci::RunEntry;
use crate::message::{CiSnapshotWire, StateDeltaPayload};
use crate::session;
use crate::transport::ci_state_store::CiStateStore;
use crate::transport::state_delta::broadcast_state_delta;

use super::{CiBroadcastListener, CiSnapshot};

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
        let activity_msg = crate::message::Message {
            from: "bpmux".to_string(),
            text: text.clone(),
            ts: chrono::Utc::now(),
            message_type: crate::message::MessageType::Activity,
            multi_depth: Some(multi_depth),
            ..crate::message::Message::default()
        };
        if let Err(e) = self.session.broadcast(&activity_msg) {
            crate::diag::log_diag(&format!("ci_listener: activity broadcast failed: {e}"));
        }
        let dev_signal = crate::ci::build_dev_ci_signal(&text);
        if let Err(e) = self.session.broadcast(&dev_signal) {
            crate::diag::log_diag(&format!("ci_listener: dev signal failed: {e}"));
        }
    }
}

fn ci_to_wire(snap: &CiSnapshot) -> CiSnapshotWire {
    CiSnapshotWire {
        state: snap.state,
        run_id: snap.run_id,
        head_sha: snap.head_sha.clone(),
        display_title: snap.display_title.clone(),
        last_successful_poll_at: None,
        url: Some(crate::ci::github_actions_url()),
    }
}
