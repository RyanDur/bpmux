use super::messaging::activity_payload;

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
