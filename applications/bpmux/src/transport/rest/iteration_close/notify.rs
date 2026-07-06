use crate::session;

const ITERATION_CLOSE_FLUSH_PROMPT: &str = "[iteration-close] iteration closing — flush unsaved \
     findings now via Write / save_lesson / add_principle / doc edit. pre-close context is still \
     available.";

pub(super) fn broadcast_iteration_closed(session: &session::Session) {
    let closed = crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(crate::message::StateDeltaPayload::IterationClose {
            promoted: Vec::new(),
            deleted_marker_id: None,
        }),
        ..crate::message::Message::default()
    };
    if let Err(e) = session.broadcast(&closed) {
        crate::diag::log_diag(&format!(
            "iteration-close: state-delta broadcast failed: {e}"
        ));
    }
    if let Err(e) = session.broadcast(&iteration_close_flush_prompt()) {
        crate::diag::log_diag(&format!(
            "iteration-close: flush-prompt broadcast failed: {e}"
        ));
    }
}

pub(super) fn iteration_close_flush_prompt() -> crate::message::Message {
    crate::message::Message {
        from: "bpmux".to_string(),
        text: ITERATION_CLOSE_FLUSH_PROMPT.to_string(),
        ts: chrono::Utc::now(),
        addressee: Some(crate::message::BROADCAST_ALL.to_string()),
        ..crate::message::Message::default()
    }
}
