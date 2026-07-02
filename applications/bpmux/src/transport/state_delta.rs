use crate::session;

pub(crate) fn broadcast_state_delta(
    session: &session::Session,
    payload: crate::message::StateDeltaPayload,
    label: &str,
) {
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
