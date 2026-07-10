#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::UserRecord;

#[test]
fn an_old_shape_session_id_record_still_deserializes_into_user_id() {
    let old_shape = r#"{"session_id":"u-1","name":"dev","role":"dev","last_delivered_at":null}"#;
    let parsed: UserRecord =
        serde_json::from_str(old_shape).expect("Ryan's existing users.json holds session_id rows");
    assert_eq!(
        parsed.user_id, "u-1",
        "the serde alias reads an old-shape session_id row from Ryan's file-backed users.json into the renamed user_id field — his existing on-disk agent identities must keep resolving after this rename"
    );
}
