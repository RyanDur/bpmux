#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::wake::{parse_wake_filter, wake_payload};

#[test]
fn parse_wake_filter_returns_none_when_no_filter_is_given() {
    assert_eq!(
        parse_wake_filter(&None).unwrap(),
        None,
        "no --filter is a valid unfiltered commitment"
    );
}

#[test]
fn parse_wake_filter_rejects_a_non_object_filter() {
    let err = parse_wake_filter(&Some("\"green\"".to_string())).unwrap_err();
    assert!(
        err.to_string().contains("JSON object"),
        "a bare JSON string can never match the key/value payload semantics; got {err}"
    );
}

#[test]
fn wake_payload_carries_both_the_role_and_the_dir_held_user_id() {
    let body = wake_payload("dev", "id-dev-1", "ci_state_changed", None);
    assert_eq!(body["role"], "dev");
    assert_eq!(body["user_id"], "id-dev-1");
    assert_eq!(body["event_class"], "ci_state_changed");
    assert!(
        body.get("filter").is_none(),
        "an absent filter is omitted, not sent as null"
    );
}
