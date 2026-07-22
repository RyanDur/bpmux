const ITERATION_CLOSE_FLUSH_PROMPT: &str = "[iteration-close] iteration closing — flush unsaved \
     findings now via Write / save_lesson / add_principle / doc edit. pre-close context is still \
     available.";

pub(super) fn broadcast_iteration_closed(hub: &hub::Hub) {
    let closed = message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::IterationClose {
            promoted: Vec::new(),
            deleted_marker_id: None,
        }),
        ..message::Message::default()
    };
    if let Err(e) = hub.broadcast(&closed) {
        diag::log_diag(&format!(
            "iteration-close: state-delta broadcast failed: {e}"
        ));
    }
    if let Err(e) = hub.broadcast(&iteration_close_flush_prompt()) {
        diag::log_diag(&format!(
            "iteration-close: flush-prompt broadcast failed: {e}"
        ));
    }
}

pub(super) fn iteration_close_flush_prompt() -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: ITERATION_CLOSE_FLUSH_PROMPT.to_string(),
        ts: chrono::Utc::now(),
        addressee: Some(message::BROADCAST_ALL.to_string()),
        ..message::Message::default()
    }
}
