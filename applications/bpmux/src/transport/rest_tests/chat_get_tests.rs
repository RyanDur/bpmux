#![allow(unused_imports)]

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use super::{init_session, seed_messages, seed_mixed_messages, test_app};
use crate::message::MessageType;
use crate::session;
use crate::transport::rest::*;

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
    assert_eq!(msgs[0]["text"], "msg 4");
    assert_eq!(msgs[1]["text"], "msg 3");
}

#[tokio::test]
async fn gets_messages_before_timestamp_returns_only_older() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let base = chrono::Utc::now() - chrono::Duration::seconds(60);
    for i in 0..5 {
        let msg = crate::message::Message {
            from: "user".to_string(),
            text: format!("msg {i}"),
            ts: base + chrono::Duration::seconds(i as i64),
            ..crate::message::Message::default()
        };
        session.broadcast(&msg).expect("broadcast");
    }
    let boundary = base + chrono::Duration::seconds(3);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let boundary_z = boundary.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
    let uri = format!("/api/messages?limit=10&before={boundary_z}");
    let request = Request::builder()
        .method("GET")
        .uri(&uri)
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(
        msgs.len(),
        3,
        "expected msgs 0/1/2 strictly before boundary; got {msgs:?}"
    );
    assert_eq!(msgs[0]["text"], "msg 2");
    assert_eq!(msgs[1]["text"], "msg 1");
    assert_eq!(msgs[2]["text"], "msg 0");
}

#[tokio::test]
async fn gets_messages_before_invalid_timestamp_falls_back_to_no_filter() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_messages(&dir, 3);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages?limit=10&before=not-a-timestamp")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 3);
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
    assert_eq!(msgs[0]["text"], "msg 2");
    assert_eq!(msgs[1]["text"], "msg 1");
}
