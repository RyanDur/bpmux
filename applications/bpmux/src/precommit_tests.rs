#![allow(clippy::unwrap_used)]

use super::precommit::{build_precommit_activity_payload, short_sha};

#[test]
fn short_sha_truncates_to_seven_chars() {
    assert_eq!(
        short_sha("c192b06c5550de0f4f61eb30811a2452804fbb3c"),
        "c192b06"
    );
}

#[test]
fn short_sha_leaves_shorter_input_intact() {
    assert_eq!(short_sha("abc123"), "abc123");
}

#[test]
fn payload_is_activity_class_for_quiet_stream() {
    let payload =
        build_precommit_activity_payload("dev", "c192b06c5550de0f4f61eb30811a2452804fbb3c");
    assert_eq!(payload["message_type"], "activity");
}

#[test]
fn payload_from_field_uses_role_hook_convention() {
    let payload =
        build_precommit_activity_payload("dev", "c192b06c5550de0f4f61eb30811a2452804fbb3c");
    assert_eq!(payload["from"], "dev:pre-commit-hook");
}

#[test]
fn payload_is_team_shape_independent_role_arg_threaded_to_from() {
    let payload = build_precommit_activity_payload("designer", "abc123");
    assert_eq!(payload["from"], "designer:pre-commit-hook");
}

#[test]
fn payload_text_cites_short_sha_and_the_gates_that_ran() {
    let payload =
        build_precommit_activity_payload("dev", "c192b06c5550de0f4f61eb30811a2452804fbb3c");
    let text = payload["text"].as_str().unwrap();
    assert!(
        text.contains("c192b06"),
        "text should mention short sha: {text}"
    );
    assert!(text.contains("fmt"), "text should cite cargo fmt: {text}");
    assert!(text.contains("clippy"), "text should cite clippy: {text}");
    assert!(
        !text.contains("c192b06c5550"),
        "text should not leak long sha: {text}"
    );
}
