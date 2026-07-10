#![allow(unused_imports)]

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use super::{init_session, seed_messages, seed_mixed_messages, test_app};
use crate::hub;
use crate::message::MessageType;
use crate::transport::rest::*;

fn leaked_select() -> crate::store::Select {
    let seeded = crate::store::SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    let select = seeded.select();
    std::mem::forget(seeded);
    select
}

#[tokio::test]
async fn gets_messages_with_limit() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let hub = seed_messages(&dir, 5);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .layer(axum::Extension(leaked_select()))
        .with_state(hub);
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
    let hub = Arc::new(hub::Hub::new(
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
        hub.broadcast(&msg).expect("broadcast");
    }
    let boundary = base + chrono::Duration::seconds(3);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .layer(axum::Extension(leaked_select()))
        .with_state(hub);
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
    let hub = seed_messages(&dir, 3);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .layer(axum::Extension(leaked_select()))
        .with_state(hub);
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
    let hub = seed_messages(&dir, 5);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .layer(axum::Extension(leaked_select()))
        .with_state(hub);
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

fn routed_app(
    dir: &TempDir,
    registrations: &[(&str, &str)],
) -> (Arc<hub::Hub>, axum::Router, crate::store::Dispatch) {
    let (tx, _rx) = broadcast::channel(16);
    let hub = Arc::new(hub::Hub::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let seeded = crate::store::SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    for (display, user_id) in registrations {
        seeded.dispatch(
            crate::store::routing::RoutingAction::Registered {
                user_id: (*user_id).to_string(),
                display: (*display).to_string(),
            }
            .into(),
        );
    }
    let dispatch = seeded.dispatcher();
    let router = axum::Router::new()
        .route(
            "/api/messages",
            axum::routing::get(get_messages).post(post_message),
        )
        .layer(axum::Extension(seeded.select()))
        .with_state(hub.clone());
    std::mem::forget(seeded);
    (hub, router, dispatch)
}

async fn post_as_owner(router: &axum::Router, body: &str) {
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    let response = router.clone().oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
}

async fn get_as(router: &axum::Router, peer: &str) -> Vec<serde_json::Value> {
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/messages?chat_only=false&as={peer}"))
        .body(Body::empty())
        .expect("build request");
    let response = router.clone().oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&body).expect("parse json")
}

#[tokio::test]
async fn get_messages_reaches_by_frozen_to_ids_even_after_the_recipients_display_is_renamed() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (hub, router, dispatch) = routed_app(&dir, &[("dev", "id-dev-1")]);
    let mut msg = crate::message::Message {
        from: "pm".to_string(),
        text: "for id-dev-1 before the rename".to_string(),
        ts: chrono::Utc::now(),
        to: Some(vec!["dev".to_string()]),
        to_ids: Some(vec!["id-dev-1".to_string()]),
        ..crate::message::Message::default()
    };
    msg.ensure_id();
    hub.broadcast(&msg).expect("broadcast");

    let seen_before = get_as(&router, "dev").await;
    assert!(
        seen_before
            .iter()
            .any(|m| m["text"] == "for id-dev-1 before the rename"),
        "id-dev-1 sees its own message under its display at stamp-time; got {seen_before:?}"
    );

    dispatch.dispatch(
        crate::store::routing::RoutingAction::Renamed {
            user_id: "id-dev-1".to_string(),
            display: "devon".to_string(),
        }
        .into(),
    );

    let seen_after_rename = get_as(&router, "devon").await;
    assert!(
        seen_after_rename
            .iter()
            .any(|m| m["text"] == "for id-dev-1 before the rename"),
        "id-dev-1's frozen to_ids still reaches it under its NEW display 'devon' — a display-only match would have missed it since the message's `to` field still literally says 'dev'; got {seen_after_rename:?}"
    );
}

#[tokio::test]
async fn get_messages_legacy_row_with_no_frozen_to_ids_falls_back_to_current_display() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (hub, router, _dispatch) = routed_app(&dir, &[("dev", "id-dev-1")]);
    let mut legacy = crate::message::Message {
        from: "pm".to_string(),
        text: "legacy addressed to dev".to_string(),
        ts: chrono::Utc::now(),
        to: Some(vec!["dev".to_string()]),
        to_ids: None,
        ..crate::message::Message::default()
    };
    legacy.ensure_id();
    hub.broadcast(&legacy).expect("broadcast");

    let seen = get_as(&router, "dev").await;
    assert!(
        seen.iter().any(|m| m["text"] == "legacy addressed to dev"),
        "a legacy row (to_ids None) still reaches its display via the fallback; got {seen:?}"
    );
}

#[tokio::test]
async fn a_message_sent_before_its_recipient_registers_reaches_them_after_registration() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (_hub, router, dispatch) = routed_app(&dir, &[]);

    post_as_owner(
        &router,
        r#"{"from":"owner","text":"early ping for dev","to":["dev"]}"#,
    )
    .await;

    dispatch.dispatch(
        crate::store::routing::RoutingAction::Registered {
            user_id: "id-dev-1".to_string(),
            display: "dev".to_string(),
        }
        .into(),
    );

    let seen = get_as(&router, "dev").await;
    assert!(
        seen.iter().any(|m| m["text"] == "early ping for dev"),
        "a message sent to a display BEFORE any user holds it must reach the user who registers that display LATER — the send stays display-matched instead of freezing an empty id set into a permanent black hole; got {seen:?}"
    );
}
