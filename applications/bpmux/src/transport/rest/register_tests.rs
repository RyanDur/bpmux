#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::RegisterRequest;

#[test]
fn an_old_shape_session_id_payload_still_deserializes_into_user_id() {
    let old_shape = r#"{"session_id":"u-1","display":"dev"}"#;
    let parsed: RegisterRequest =
        serde_json::from_str(old_shape).expect("an in-flight old CLI still posts session_id");
    assert_eq!(
        parsed.user_id, "u-1",
        "the serde alias reads an old-shape session_id payload into the renamed user_id field — an in-flight old CLI (built before this rename) must keep registering successfully"
    );
}
