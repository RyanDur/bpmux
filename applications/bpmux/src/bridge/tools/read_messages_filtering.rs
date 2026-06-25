#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::read_messages::*;

#[test]
fn read_messages_request_path_scopes_by_agent_via_as_param() {
    let path = read_messages_request_path(20, "dev");
    assert!(
        path.contains("as=dev"),
        "read_messages must pass ?as=<agent> so the REST endpoint applies the same \
         should_deliver topic-filter as the WS push path; dropping it diverges REST-read \
         visibility from WS-push. got {path}"
    );
    assert!(
        path.contains("limit=20"),
        "limit must be carried; got {path}"
    );
}
