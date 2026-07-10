#![allow(unused_imports)]

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use super::{init_session, seed_messages, seed_mixed_messages, test_app, test_app_with_routing};
use crate::hub;
use crate::message::MessageType;
use crate::transport::rest::*;

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
async fn post_message_with_to_sets_explicit_privacy_true() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app(&dir);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"from":"dev:claude-code","text":"shh","to":["pm"]}"#,
        ))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert!(
        msg.explicit_privacy,
        "DM with `to` filter must set explicit_privacy=true"
    );
    assert_eq!(msg.to.as_deref(), Some(["pm".to_string()].as_slice()));
}

#[tokio::test]
async fn post_message_without_to_leaves_explicit_privacy_false() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app(&dir);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"from":"dev:claude-code","text":"open"}"#))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert!(
        !msg.explicit_privacy,
        "broadcast must not set explicit_privacy"
    );
    assert!(msg.to.is_none(), "broadcast must have no `to` filter");
}

#[tokio::test]
async fn post_with_sender_id_carries_it_verbatim_no_resolve_guess() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app_with_routing(&dir, &[("dev", "id-dev-guess")]);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"from":"dev:claude-code","text":"hi","sender_id":"id-dev-explicit"}"#,
        ))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert_eq!(
        msg.sender_id.as_deref(),
        Some("id-dev-explicit"),
        "an explicit payload sender_id rides verbatim, never overridden by the resolve guess"
    );
}

#[tokio::test]
async fn post_without_sender_id_falls_back_to_the_resolve_guess() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app_with_routing(&dir, &[("dev", "id-dev-guess")]);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"from":"dev:claude-code","text":"hi"}"#))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert_eq!(
        msg.sender_id.as_deref(),
        Some("id-dev-guess"),
        "a legacy sender with no payload sender_id keeps today's resolve-guess fallback"
    );
}

#[tokio::test]
async fn post_to_a_display_shared_by_two_users_carries_both_ids() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app_with_routing(&dir, &[("dev", "id-dev-1"), ("dev", "id-dev-2")]);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"from":"owner","text":"hi","to":["dev"]}"#))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    let mut ids = msg.to_ids.expect("to_ids present for an explicit `to`");
    ids.sort();
    assert_eq!(
        ids,
        vec!["id-dev-1".to_string(), "id-dev-2".to_string()],
        "a display shared by two users must address both ids"
    );
}

#[tokio::test]
async fn post_broadcast_leaves_to_ids_none() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app_with_routing(&dir, &[("dev", "id-dev-1")]);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"from":"owner","text":"hi"}"#))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert!(
        msg.to_ids.is_none(),
        "a broadcast (`to: None`) must not gain a to_ids list"
    );
}

#[tokio::test]
async fn post_with_any_unresolvable_display_does_not_freeze_partial_to_ids() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app_with_routing(&dir, &[("dev", "id-dev-1"), ("dev", "id-dev-2")]);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"from":"owner","text":"hi","to":["dev","ghost"]}"#,
        ))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert!(
        msg.to_ids.is_none(),
        "ghost resolves to zero ids — a PARTIAL resolution must not freeze: the whole message stays display-matched (to_ids None) until every addressee exists, else ghost's future user never receives it; got {:?}",
        msg.to_ids
    );
}

#[tokio::test]
async fn post_to_a_display_with_no_registered_users_leaves_to_ids_none() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app_with_routing(&dir, &[]);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"from":"owner","text":"hi","to":["dev"]}"#))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert!(
        msg.to_ids.is_none(),
        "nobody is registered at send time — stamping Some(empty) makes the message a permanent black hole (Some-ids matches only ids, and there are none); it must stay display-matched (to_ids None); got {:?}",
        msg.to_ids
    );
}

#[tokio::test]
async fn post_addressee_with_no_registered_users_leaves_addressee_ids_none() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app_with_routing(&dir, &[]);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"from":"owner","text":"hi","addressee":"dev"}"#,
        ))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert!(
        msg.addressee_ids.is_none(),
        "an addressee resolving to zero ids must stamp None, not Some(empty) — a dev who registers later must still match the @dev marker by display fallback; got {:?}",
        msg.addressee_ids
    );
}

#[tokio::test]
async fn post_addressee_shared_by_two_users_carries_both_ids() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app_with_routing(&dir, &[("dev", "id-dev-1"), ("dev", "id-dev-2")]);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"from":"owner","text":"hi","addressee":"dev"}"#,
        ))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    let mut ids = msg
        .addressee_ids
        .expect("addressee_ids present for an explicit addressee");
    ids.sort();
    assert_eq!(ids, vec!["id-dev-1".to_string(), "id-dev-2".to_string()]);
}

#[tokio::test]
async fn post_addressee_all_leaves_addressee_ids_none() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app_with_routing(&dir, &[("dev", "id-dev-1")]);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"from":"owner","text":"hi","addressee":"all"}"#,
        ))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert!(
        msg.addressee_ids.is_none(),
        "addressee \"all\" must not gain an addressee_ids list"
    );
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
