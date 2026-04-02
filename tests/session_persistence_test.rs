//! Integration test: messages persist across session resumes
//!
//! This test proves the core Story 1 acceptance criteria:
//! "Restarting bpmux and reconnecting to the same session shows the previous messages"

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn messages_persist_across_session_resumes() {
    use bpmux::session::{Message, append_message, read_messages, resume_or_create_session};
    use chrono::Utc;
    use tempfile::TempDir;

    let base = TempDir::new().expect("temp base");

    // First "run": create session, write messages
    let session1 = resume_or_create_session(base.path()).expect("create session");
    let msg1 = Message {
        from: "ryan:human".to_string(),
        text: "first session message".to_string(),
        ts: Utc::now(),
    };
    let msg2 = Message {
        from: "ryan:human".to_string(),
        text: "second session message".to_string(),
        ts: Utc::now(),
    };
    append_message(&session1, &msg1).expect("append 1");
    append_message(&session1, &msg2).expect("append 2");

    // Second "run": resume session, verify messages are there
    let session2 = resume_or_create_session(base.path()).expect("resume session");

    // Should be the same directory
    assert_eq!(
        session1, session2,
        "resumed session should be the same directory"
    );

    // Messages should persist
    let messages = read_messages(&session2).expect("read messages");
    assert_eq!(messages.len(), 2, "both messages should persist");
    assert_eq!(messages[0].text, "first session message");
    assert_eq!(messages[1].text, "second session message");
    assert_eq!(messages[0].from, "ryan:human");
}
