#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::rpc_client::*;
use httpmock::{Method, MockServer};
use serde_json::{Map, Value};
use std::sync::Mutex;

static AUTH_ENV_LOCK: Mutex<()> = Mutex::new(());

struct AuthTokenGuard {
    _dir: tempfile::TempDir,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl AuthTokenGuard {
    fn with_token(token: &str) -> Self {
        let guard = AUTH_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth_token");
        std::fs::write(&path, token).unwrap();
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("BPMUX_AUTH_TOKEN_PATH", &path);
        }
        Self {
            _dir: dir,
            _guard: guard,
        }
    }
}

impl Drop for AuthTokenGuard {
    fn drop(&mut self) {
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("BPMUX_AUTH_TOKEN_PATH");
        }
    }
}

fn args(id: u64) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("id".into(), serde_json::json!(id));
    map.insert("pickup_paraphrase".into(), serde_json::json!("paraphrase"));
    map.insert("falsifier_reframing".into(), serde_json::json!("falsifier"));
    map
}

#[tokio::test]
async fn dispatch_carries_bearer_and_agent_header_to_endpoint() {
    let _auth = AuthTokenGuard::with_token("test-token-123");
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST)
            .path("/api/rpc/start_story")
            .header("authorization", "Bearer test-token-123")
            .header("X-BPMUX-Agent", "dev")
            .body_includes("\"id\":42")
            .body_includes("\"pickup_paraphrase\":\"paraphrase\"");
        then.status(200)
            .header("content-type", "application/json")
            .body(r##"{"message":"#42 marked as in progress"}"##);
    });
    let port: u16 = server.port();

    let result = dispatch_via_rpc(port, "start_story", "dev", &args(42)).await;

    m.assert();
    assert_eq!(result, Ok("#42 marked as in progress".to_string()));
}

#[tokio::test]
async fn dispatch_surfaces_named_error_on_offline_server() {
    let _auth = AuthTokenGuard::with_token("tk");
    let result = dispatch_via_rpc(1, "start_story", "dev", &args(42)).await;
    let err = result.expect_err("offline server must fail-loud");
    assert!(
        err.contains("electron-server unreachable"),
        "expected named cause in {err}"
    );
}

#[tokio::test]
async fn dispatch_surfaces_server_error_body_on_4xx() {
    let _auth = AuthTokenGuard::with_token("tk");
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST).path("/api/rpc/start_story");
        then.status(400)
            .header("content-type", "application/json")
            .body(r#"{"error":"only dev role can start_story"}"#);
    });
    let port: u16 = server.port();

    let result = dispatch_via_rpc(port, "start_story", "pm", &args(42)).await;

    m.assert();
    let err = result.expect_err("4xx must surface as Err");
    assert!(
        err.contains("only dev role can start_story"),
        "expected server-error message in {err}"
    );
}

#[tokio::test]
async fn dispatch_surfaces_missing_message_field_on_2xx() {
    let _auth = AuthTokenGuard::with_token("tk");
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST).path("/api/rpc/start_story");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"unexpected":"shape"}"#);
    });
    let port: u16 = server.port();

    let result = dispatch_via_rpc(port, "start_story", "dev", &args(42)).await;
    m.assert();
    let err = result.expect_err("missing message field must surface");
    assert!(
        err.contains("missing 'message' field"),
        "expected protocol-error framing in {err}"
    );
}

#[tokio::test]
async fn dispatch_routes_path_param_for_arbitrary_tools() {
    let _auth = AuthTokenGuard::with_token("tk");
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST).path("/api/rpc/accept_story");
        then.status(200)
            .header("content-type", "application/json")
            .body(r##"{"message":"#9 accepted"}"##);
    });
    let port: u16 = server.port();

    let result = dispatch_via_rpc(port, "accept_story", "owner", &args(9)).await;
    m.assert();
    assert_eq!(result, Ok("#9 accepted".to_string()));
}
