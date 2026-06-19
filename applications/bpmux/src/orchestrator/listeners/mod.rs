mod ci;
mod owner_park;
mod reactions;
mod roster;

pub use ci::{CiSessionBroadcaster, CiStateStoreWriter};
pub use owner_park::{OwnerParkBroadcaster, owner_park_presence};
pub use reactions::ReactionsSessionBroadcaster;
pub use roster::RosterSessionBroadcaster;

use crate::session;
use crate::session::message::StateDeltaPayload;

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
