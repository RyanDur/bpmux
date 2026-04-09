#![allow(clippy::unwrap_used, clippy::expect_used)]

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
