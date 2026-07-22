#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn messages_persist_across_daemon_restarts_without_any_session_indirection() {
    use chrono::Utc;
    use location::durable_data_dir;
    use message::Message;
    use message_log::{append_message, read_messages};
    use tempfile::TempDir;

    let base = TempDir::new().expect("temp base");

    let first_run = durable_data_dir(base.path()).expect("data dir");
    let msg1 = Message {
        from: "owner:human".to_string(),
        text: "message before the restart".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    let msg2 = Message {
        from: "owner:human".to_string(),
        text: "second message before the restart".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    append_message(&first_run, &msg1).expect("append 1");
    append_message(&first_run, &msg2).expect("append 2");

    let second_run = durable_data_dir(base.path()).expect("data dir after restart");

    assert_eq!(
        first_run, second_run,
        "the data dir IS the base — both runs resolve the same flat directory"
    );
    assert_eq!(
        first_run,
        base.path(),
        "no uuid child, no current symlink — the base itself holds the data"
    );

    let messages = read_messages(&second_run).expect("read messages");
    assert_eq!(messages.len(), 2, "both messages persist");
    assert_eq!(messages[0].text, "message before the restart");
    assert_eq!(messages[1].text, "second message before the restart");
    assert_eq!(messages[0].from, "owner:human");
}
