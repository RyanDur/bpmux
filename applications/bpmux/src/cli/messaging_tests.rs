use super::messaging::{activity_payload, send_payload};

#[test]
fn activity_payload_is_activity_class_for_the_quiet_stream() {
    let body = activity_payload("dev:pre-commit-hook", "anything");
    assert_eq!(body["message_type"], "activity");
}

#[test]
fn activity_payload_passes_sender_and_text_through_verbatim() {
    let body = activity_payload(
        "designer:pre-commit-hook",
        "pre-commit gates passed @ abc1234",
    );
    assert_eq!(body["from"], "designer:pre-commit-hook");
    assert_eq!(body["text"], "pre-commit gates passed @ abc1234");
}

#[test]
fn send_payload_includes_the_dir_held_sender_id_when_resolved() {
    let body = send_payload("dev", "hi", &[], &None, Some("id-dev-1".to_string()));
    assert_eq!(body["sender_id"], "id-dev-1");
}

#[test]
fn send_payload_omits_sender_id_when_resolution_failed() {
    let body = send_payload("dev", "hi", &[], &None, None);
    assert!(
        body.get("sender_id").is_none(),
        "a resolve_user_id error must not get flakier than today's send — omit the field and send anyway"
    );
}
