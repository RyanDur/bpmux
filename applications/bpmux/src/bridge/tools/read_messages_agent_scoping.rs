#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::read_messages::handle_read_messages;
use httpmock::{Method, MockServer};

#[tokio::test]
async fn read_messages_scopes_the_rest_fetch_to_the_caller_via_the_as_param() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/api/messages")
            .query_param("as", "dev")
            .query_param("limit", "20");
        then.status(200)
            .header("content-type", "application/json")
            .body("[]");
    });
    let mut args = serde_json::Map::new();
    args.insert("limit".into(), serde_json::json!(20));

    let out = handle_read_messages(&args, server.port(), "dev:claude-code")
        .await
        .unwrap();

    mock.assert();
    assert_eq!(
        out, "No messages.",
        "the REST read must carry ?as=<agent> so the endpoint applies the same should_deliver \
         topic-filter as the WS push path; otherwise REST-read visibility diverges from WS-push"
    );
}
