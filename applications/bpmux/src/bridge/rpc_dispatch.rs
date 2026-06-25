#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::rpc_client::*;
use httpmock::{Method, MockServer};
use serde_json::Value;

fn body(number: u64) -> Value {
    serde_json::json!({
        "number": number,
        "pickup_paraphrase": "paraphrase",
        "falsifier_reframing": "falsifier",
    })
}

#[tokio::test]
async fn post_rpc_carries_bearer_and_agent_header_to_endpoint() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST)
            .path("/api/rpc/start_story")
            .header("authorization", "Bearer test-token-123")
            .header(RPC_AGENT_HEADER, "dev")
            .body_includes("\"number\":42")
            .body_includes("\"pickup_paraphrase\":\"paraphrase\"");
        then.status(200)
            .header("content-type", "application/json")
            .body(r##"{"message":"#42 marked as in progress"}"##);
    });
    let port: u16 = server.port();

    let result = post_rpc(port, "start_story", "dev", &body(42), "test-token-123").await;

    m.assert();
    assert_eq!(result, Ok("#42 marked as in progress".to_string()));
}

#[tokio::test]
async fn post_rpc_surfaces_named_error_on_offline_server() {
    let result = post_rpc(1, "start_story", "dev", &body(42), "tk").await;
    let err = result.expect_err("offline server must fail-loud");
    assert!(
        err.contains("electron-server unreachable"),
        "expected named cause in {err}"
    );
}

#[tokio::test]
async fn post_rpc_surfaces_server_error_body_on_4xx() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST).path("/api/rpc/start_story");
        then.status(400)
            .header("content-type", "application/json")
            .body(r#"{"error":"only dev role can start_story"}"#);
    });
    let port: u16 = server.port();

    let result = post_rpc(port, "start_story", "pm", &body(42), "tk").await;

    m.assert();
    let err = result.expect_err("4xx must surface as Err");
    assert!(
        err.contains("only dev role can start_story"),
        "expected server-error message in {err}"
    );
}

#[tokio::test]
async fn post_rpc_surfaces_missing_message_field_on_2xx() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST).path("/api/rpc/start_story");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"unexpected":"shape"}"#);
    });
    let port: u16 = server.port();

    let result = post_rpc(port, "start_story", "dev", &body(42), "tk").await;
    m.assert();
    let err = result.expect_err("missing message field must surface");
    assert!(
        err.contains("missing 'message' field"),
        "expected protocol-error framing in {err}"
    );
}

#[tokio::test]
async fn post_rpc_routes_path_param_for_arbitrary_tools() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST).path("/api/rpc/accept_story");
        then.status(200)
            .header("content-type", "application/json")
            .body(r##"{"message":"#9 accepted"}"##);
    });
    let port: u16 = server.port();

    let result = post_rpc(port, "accept_story", "owner", &body(9), "tk").await;
    m.assert();
    assert_eq!(result, Ok("#9 accepted".to_string()));
}
