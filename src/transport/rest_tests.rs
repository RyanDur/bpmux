#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::rest::*;
use crate::session;
use crate::session::MessageType;
use axum::body::Body;
use axum::http::Request;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use axum::http::StatusCode;

fn test_app(dir: &TempDir) -> (axum::Router, broadcast::Sender<session::Message>) {
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx.clone(),
    ));
    let router = axum::Router::new()
        .route(
            "/api/messages",
            axum::routing::get(get_messages).post(post_message),
        )
        .with_state(session);
    (router, tx)
}

fn init_session(dir: &TempDir) -> PathBuf {
    let messages_file = dir.path().join("messages-current.jsonl");
    std::fs::write(&messages_file, "").expect("create messages file");
    dir.path().to_path_buf()
}

#[tokio::test]
async fn posts_message_and_broadcasts() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app(&dir);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"from":"test:hook","text":"hello from hook"}"#,
        ))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert_eq!(msg.from, "test:hook");
    assert_eq!(msg.text, "hello from hook");
}

#[tokio::test]
async fn rejects_empty_text() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, _tx) = test_app(&dir);
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"from":"test:hook","text":""}"#))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn rejects_missing_fields() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, _tx) = test_app(&dir);
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"text":"no from field"}"#))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

fn seed_messages(dir: &TempDir, count: usize) -> Arc<session::Session> {
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    for i in 0..count {
        let msg = session::Message {
            from: "user".to_string(),
            text: format!("msg {i}"),
            ts: chrono::Utc::now(),
            message_type: MessageType::Chat,
            to: None,
            addressee: None,
            explicit_privacy: false,
            multi_depth: None,
            topic: None,
        };
        session.broadcast(&msg).expect("broadcast");
    }
    session
}

#[tokio::test]
async fn gets_messages_with_limit() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_messages(&dir, 5);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages?limit=2")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 2);
    // Most recent first
    assert_eq!(msgs[0]["text"], "msg 4");
    assert_eq!(msgs[1]["text"], "msg 3");
}

#[tokio::test]
async fn gets_messages_with_offset() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_messages(&dir, 5);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages?limit=2&offset=2")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 2);
    // Reversed: [4,3,2,1,0], skip 2 -> [2,1,0], take 2 -> [2,1]
    assert_eq!(msgs[0]["text"], "msg 2");
    assert_eq!(msgs[1]["text"], "msg 1");
}

fn seed_mixed_messages(dir: &TempDir) -> Arc<session::Session> {
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    for (text, msg_type) in [
        ("hello", MessageType::Chat),
        ("busy", MessageType::Activity),
        ("world", MessageType::Chat),
        ("available", MessageType::Activity),
        ("goodbye", MessageType::Chat),
    ] {
        let msg = session::Message {
            from: "dev".to_string(),
            text: text.to_string(),
            ts: chrono::Utc::now(),
            message_type: msg_type,
            to: None,
            addressee: None,
            explicit_privacy: false,
            multi_depth: None,
            topic: None,
        };
        session.broadcast(&msg).expect("broadcast");
    }
    session
}

#[tokio::test]
async fn chat_only_excludes_activity_by_default() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_mixed_messages(&dir);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages?limit=50")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 3, "should only return chat messages");
    assert!(msgs.iter().all(|m| m["message_type"].is_null()));
}

#[tokio::test]
async fn chat_only_false_returns_all() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_mixed_messages(&dir);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages?limit=50&chat_only=false")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 5, "should return all messages");
}

#[tokio::test]
async fn defaults_to_twenty_messages() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_messages(&dir, 25);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 20);
}

// ---------- Classifier misfire endpoint ----------

#[tokio::test]
async fn classifier_misfire_accepts_post_with_reason() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let router = axum::Router::new()
        .route(
            "/api/classifier-misfire",
            axum::routing::post(post_classifier_misfire),
        )
        .with_state(session);
    let request = Request::builder()
        .method("POST")
        .uri("/api/classifier-misfire")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"agent":"dev","nudge_hash":"abc123","reason":"recursive match on bpmux nudge"}"#,
        ))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn classifier_misfire_accepts_post_without_reason() {
    // Per the story's acceptance criteria: reason is OPTIONAL.
    // State the observation, not the conclusion — agents don't have
    // to explain WHY a fire was wrong, just marking it as wrong is
    // valuable feedback.
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let router = axum::Router::new()
        .route(
            "/api/classifier-misfire",
            axum::routing::post(post_classifier_misfire),
        )
        .with_state(session);
    let request = Request::builder()
        .method("POST")
        .uri("/api/classifier-misfire")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"agent":"dev","nudge_hash":"xyz789"}"#))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn classifier_misfire_requires_agent_and_hash() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let router = axum::Router::new()
        .route(
            "/api/classifier-misfire",
            axum::routing::post(post_classifier_misfire),
        )
        .with_state(session);
    // Missing nudge_hash
    let request = Request::builder()
        .method("POST")
        .uri("/api/classifier-misfire")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"agent":"dev"}"#))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    // serde rejection produces 4xx, not 201
    assert_ne!(response.status(), StatusCode::CREATED);
}

// ─────────────────────────────────────────────────────────────────────
// /api/simulate/key tests — #170
//
// Behavior-framed names because each test drives the real axum
// router through the real handler and asserts on the emitted
// crossterm event (received via the mpsc receiver). The handler is
// the system under test; the tests describe its observable behavior.
// ─────────────────────────────────────────────────────────────────────

fn test_app_with_key_injector() -> (
    axum::Router,
    tokio::sync::mpsc::Receiver<crossterm::event::Event>,
) {
    let (inject_tx, inject_rx) = tokio::sync::mpsc::channel::<crossterm::event::Event>(16);
    let router = axum::Router::new()
        .route("/api/simulate/key", axum::routing::post(simulate_key))
        .layer(axum::Extension(inject_tx));
    (router, inject_rx)
}

async fn post_key_payload(router: axum::Router, body: &str) -> axum::http::Response<Body> {
    let request = Request::builder()
        .method("POST")
        .uri("/api/simulate/key")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    router.oneshot(request).await.expect("response")
}

#[tokio::test]
async fn simulate_key_emits_plain_char_event_when_no_modifiers_field_present() {
    // Backward-compat: payload without `modifiers` field must behave
    // identically to the current no-modifier path. This guards the
    // default-empty deserialization.
    let (router, mut rx) = test_app_with_key_injector();
    let response = post_key_payload(router, r#"{"key":"1"}"#).await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = rx.recv().await.expect("receive event");
    match event {
        crossterm::event::Event::Key(k) => {
            assert_eq!(k.code, crossterm::event::KeyCode::Char('1'));
            assert_eq!(k.modifiers, crossterm::event::KeyModifiers::empty());
        }
        other => panic!("expected Key event, got {other:?}"),
    }
}

#[tokio::test]
async fn simulate_key_emits_alt_modified_event_when_modifiers_contains_alt() {
    // GIVEN `{"key":"1","modifiers":["alt"]}`
    // WHEN the handler processes the request
    // THEN the emitted KeyEvent carries the Alt bit
    let (router, mut rx) = test_app_with_key_injector();
    let response = post_key_payload(router, r#"{"key":"1","modifiers":["alt"]}"#).await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = rx.recv().await.expect("receive event");
    match event {
        crossterm::event::Event::Key(k) => {
            assert_eq!(k.code, crossterm::event::KeyCode::Char('1'));
            assert!(
                k.modifiers.contains(crossterm::event::KeyModifiers::ALT),
                "expected Alt in modifiers, got {:?}",
                k.modifiers
            );
        }
        other => panic!("expected Key event, got {other:?}"),
    }
}

#[tokio::test]
async fn simulate_key_emits_multi_modifier_event_when_ctrl_and_shift_both_set() {
    let (router, mut rx) = test_app_with_key_injector();
    let response = post_key_payload(router, r#"{"key":"x","modifiers":["ctrl","shift"]}"#).await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = rx.recv().await.expect("receive event");
    match event {
        crossterm::event::Event::Key(k) => {
            assert_eq!(k.code, crossterm::event::KeyCode::Char('x'));
            assert!(
                k.modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            );
            assert!(k.modifiers.contains(crossterm::event::KeyModifiers::SHIFT));
        }
        other => panic!("expected Key event, got {other:?}"),
    }
}

#[tokio::test]
async fn simulate_key_returns_bad_request_when_modifier_name_is_unknown() {
    // Unknown modifier strings must fail loudly — silent-ignore was
    // the bug #170 fixes, where `"alt"` was accepted and dropped so
    // agents believed they had tested a modifier combination when
    // they had only tested the plain key.
    let (router, mut rx) = test_app_with_key_injector();
    let response = post_key_payload(router, r#"{"key":"1","modifiers":["meta"]}"#).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    // No event should have been queued.
    assert!(
        rx.try_recv().is_err(),
        "unknown modifier must not enqueue event"
    );
}

#[tokio::test]
async fn simulate_key_is_case_insensitive_on_modifier_names() {
    // Agents type "Alt" / "CTRL" / "shift" variably. Normalize on
    // the handler side so the adapter contract stays forgiving.
    let (router, mut rx) = test_app_with_key_injector();
    let response = post_key_payload(router, r#"{"key":"a","modifiers":["ALT","Shift"]}"#).await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = rx.recv().await.expect("receive event");
    match event {
        crossterm::event::Event::Key(k) => {
            assert!(k.modifiers.contains(crossterm::event::KeyModifiers::ALT));
            assert!(k.modifiers.contains(crossterm::event::KeyModifiers::SHIFT));
        }
        other => panic!("expected Key event, got {other:?}"),
    }
}
