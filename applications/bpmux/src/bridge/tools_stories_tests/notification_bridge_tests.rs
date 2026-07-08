#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::transport::protocol::ClientMessage;

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod notification_bridge_push_filter_integration_1432 {
    use crate::bridge::message_type_reaches_llm_stream;
    use crate::hub::Hub;
    use crate::message::Message;
    use crate::message::MessageType;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::broadcast;

    fn make_hub(dir: PathBuf) -> Arc<Hub> {
        let (tx, _) = broadcast::channel(16);
        Arc::new(Hub::new(dir, "owner".to_string(), tx))
    }

    fn make_message(message_type: MessageType, text: &str, addressee: Option<&str>) -> Message {
        Message {
            from: "bpmux".to_string(),
            text: text.to_string(),
            ts: chrono::Utc::now(),
            message_type,
            addressee: addressee.map(str::to_string),
            ..Message::default()
        }
    }

    async fn fire_notification_and_assert_reaches_llm_stream(
        hub: &Arc<Hub>,
        message: Message,
        expected_reaches: bool,
    ) {
        let mut rx = hub.subscribe();
        hub.broadcast(&message).expect("broadcast");
        let received = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
            .await
            .ok()
            .and_then(Result::ok);
        let received_message =
            received.expect("ws-layer always delivers; bridge-push filters later");
        let push_decision =
            message_type_reaches_llm_stream(received_message.message_type.to_wire().as_deref());
        assert_eq!(
            push_decision, expected_reaches,
            "MessageType {:?} bridge-push-filter decision should be {} but observed {}",
            received_message.message_type, expected_reaches, push_decision,
        );
    }

    #[test]
    fn chat_class_reaches_llm_stream() {
        assert!(message_type_reaches_llm_stream(None));
        assert!(message_type_reaches_llm_stream(Some("insight")));
        assert!(message_type_reaches_llm_stream(Some("nudge")));
        assert!(message_type_reaches_llm_stream(Some("state-delta")));
        assert!(message_type_reaches_llm_stream(Some("state-snapshot")));
    }

    #[test]
    fn activity_class_filtered_at_llm_stream() {
        assert!(!message_type_reaches_llm_stream(Some("activity")));
    }

    #[test]
    fn system_class_filtered_at_llm_stream() {
        assert!(!message_type_reaches_llm_stream(Some("system")));
    }

    #[tokio::test]
    async fn fire_notification_helper_pins_chat_class_reaches_pm() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let msg = make_message(MessageType::Chat, "verdict cast on #42", Some("pm"));
        fire_notification_and_assert_reaches_llm_stream(&hub, msg, true).await;
    }

    #[tokio::test]
    async fn fire_notification_helper_pins_activity_class_does_not_reach_pm() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let msg = make_message(MessageType::Activity, "board-state-tick", Some("pm"));
        fire_notification_and_assert_reaches_llm_stream(&hub, msg, false).await;
    }

    #[tokio::test]
    async fn fire_notification_helper_pins_system_class_does_not_reach_pm() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let msg = make_message(MessageType::System, "system-event", None);
        fire_notification_and_assert_reaches_llm_stream(&hub, msg, false).await;
    }

    #[tokio::test]
    async fn verdict_cast_1429_chat_class_reaches_pm_retroactive_coverage() {
        let dir = TempDir::new().unwrap();
        let hub = make_hub(dir.path().to_path_buf());
        let msg = make_message(
            MessageType::Chat,
            "verdict cast on #1429 from designer: verified",
            Some("pm"),
        );
        fire_notification_and_assert_reaches_llm_stream(&hub, msg, true).await;
    }
}
