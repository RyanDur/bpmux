use message::MessageType;

pub(super) fn system_announcement(text: &str) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        message_type: MessageType::System,
        ..message::Message::default()
    }
}

pub(super) fn build_state_snapshot_message(store: &crate::store::Store) -> message::Message {
    let snapshot = store.build_snapshot();
    message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: MessageType::StateSnapshot,
        state_snapshot: Some(snapshot),
        ..message::Message::default()
    }
}
