mod ci;
mod owner_park;
mod reactions;
mod roster;

pub use ci::{CiSessionBroadcaster, CiStateStoreWriter};
pub use owner_park::{OwnerParkBroadcaster, owner_park_presence};
pub use reactions::ReactionsSessionBroadcaster;
pub use roster::RosterSessionBroadcaster;

use crate::message::StateDeltaPayload;
use crate::session;

fn broadcast_state_delta(session: &session::Session, payload: StateDeltaPayload, label: &str) {
    let msg = crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(payload),
        ..crate::message::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        crate::diag::log_diag(&format!("{label}: state-delta broadcast failed: {e}"));
    }
}
