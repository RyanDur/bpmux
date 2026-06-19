use crate::session;
use crate::session::MessageType;

pub(super) fn system_announcement(text: &str) -> session::Message {
    session::Message {
        from: "bpmux".to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        message_type: MessageType::System,
        ..session::Message::default()
    }
}

pub(super) fn build_state_snapshot_message(
    orchestrator: &crate::orchestrator::Orchestrator<
        crate::board::adapters::github::GitHubSourceAdapter,
    >,
) -> session::Message {
    let snapshot = orchestrator.build_snapshot();
    session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: MessageType::StateSnapshot,
        state_snapshot: Some(snapshot),
        ..session::Message::default()
    }
}
