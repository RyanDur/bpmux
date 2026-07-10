use crate::hub;

pub(crate) fn broadcast_state_delta(
    hub: &hub::Hub,
    payload: message::StateDeltaPayload,
    label: &str,
) {
    let msg = message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(payload),
        ..message::Message::default()
    };
    if let Err(e) = hub.broadcast(&msg) {
        diag::log_diag(&format!("{label}: state-delta broadcast failed: {e}"));
    }
}
