use crate::message::MessageType;

pub(super) fn system_announcement(text: &str) -> crate::message::Message {
    crate::message::Message {
        from: "bpmux".to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        message_type: MessageType::System,
        ..crate::message::Message::default()
    }
}

pub(super) fn build_state_snapshot_message(
    orchestrator: &crate::orchestrator::Orchestrator<
        crate::board::adapters::github::GitHubSourceAdapter,
    >,
) -> crate::message::Message {
    let snapshot = orchestrator.build_snapshot();
    crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: MessageType::StateSnapshot,
        state_snapshot: Some(snapshot),
        ..crate::message::Message::default()
    }
}
