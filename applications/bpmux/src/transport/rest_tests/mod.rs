#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod board_story_tests;
mod chat_filter_tests;
mod chat_get_tests;
mod chat_misc_tests;
mod chat_post_tests;
mod excerpt_enrichment_tests;
mod reaction_event_tests;
mod terminal_spawn_register_tests;
mod wait_event_tests;

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use httpmock::MockServer;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use crate::bridge::tracker::OctocrabStoryTracker;
use crate::message::MessageType;
use crate::session;
use crate::transport::rest::{TrackerHandle, *};

pub(super) fn test_app(
    dir: &TempDir,
) -> (axum::Router, broadcast::Sender<crate::message::Message>) {
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
        .layer(axum::Extension(Arc::new(
            crate::routing::RoutingStore::new(session.storage.clone()),
        )))
        .with_state(session);
    (router, tx)
}

pub(super) fn init_session(dir: &TempDir) -> PathBuf {
    let messages_file = dir.path().join("messages-current.jsonl");
    std::fs::write(&messages_file, "").expect("create messages file");
    dir.path().to_path_buf()
}

pub(super) fn seed_messages(dir: &TempDir, count: usize) -> Arc<session::Session> {
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    for i in 0..count {
        let msg = crate::message::Message {
            from: "user".to_string(),
            text: format!("msg {i}"),
            ts: chrono::Utc::now(),
            ..crate::message::Message::default()
        };
        session.broadcast(&msg).expect("broadcast");
    }
    session
}

pub(super) fn seed_mixed_messages(dir: &TempDir) -> Arc<session::Session> {
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
        let msg = crate::message::Message {
            from: "dev".to_string(),
            text: text.to_string(),
            ts: chrono::Utc::now(),
            message_type: msg_type,
            ..crate::message::Message::default()
        };
        session.broadcast(&msg).expect("broadcast");
    }
    session
}

pub(super) fn board_test_session(
    dir: &TempDir,
) -> (
    Arc<session::Session>,
    broadcast::Sender<crate::message::Message>,
) {
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "owner".to_string(),
        tx.clone(),
    ));
    (session, tx)
}

pub(super) fn board_author() -> Value {
    json!({
        "login": "owner",
        "id": 1,
        "node_id": "MDQ6VXNlcjE=",
        "avatar_url": "https://example.test/avatar.png",
        "gravatar_id": "",
        "url": "https://example.test/users/owner",
        "html_url": "https://example.test/owner",
        "followers_url": "https://example.test/owner/followers",
        "following_url": "https://example.test/owner/following{/other_user}",
        "gists_url": "https://example.test/owner/gists{/gist_id}",
        "starred_url": "https://example.test/owner/starred{/owner}{/repo}",
        "subscriptions_url": "https://example.test/owner/subscriptions",
        "organizations_url": "https://example.test/owner/orgs",
        "repos_url": "https://example.test/owner/repos",
        "events_url": "https://example.test/owner/events{/privacy}",
        "received_events_url": "https://example.test/owner/received_events",
        "type": "User",
        "site_admin": false
    })
}

pub(super) fn board_issue(number: u64, title: &str, state: &str, body: Option<&str>) -> Value {
    json!({
        "id": number,
        "node_id": "MDU6SXNzdWUx",
        "url": format!("https://example.test/issues/{number}"),
        "repository_url": "https://example.test/repo",
        "labels_url": format!("https://example.test/issues/{number}/labels{{/name}}"),
        "comments_url": format!("https://example.test/issues/{number}/comments"),
        "events_url": format!("https://example.test/issues/{number}/events"),
        "html_url": format!("https://example.test/web/{number}"),
        "number": number,
        "state": state,
        "state_reason": null,
        "title": title,
        "body": body,
        "user": board_author(),
        "labels": [],
        "assignees": [],
        "author_association": "OWNER",
        "locked": false,
        "comments": 0,
        "created_at": "2026-04-18T12:34:56Z",
        "updated_at": "2026-04-18T12:34:56Z"
    })
}

pub(super) fn tracker_for(server: &MockServer) -> crate::integrations::github::GitHubSourceAdapter {
    let inner = OctocrabStoryTracker::with_base_url(
        &format!("{}/", server.base_url()),
        "acme".to_string(),
        "bpmux".to_string(),
    )
    .expect("build tracker against mock");
    crate::integrations::github::GitHubSourceAdapter::new(inner)
}

pub(super) fn board_router(session: Arc<session::Session>, tracker: TrackerHandle) -> axum::Router {
    use axum::routing::{get, post};
    let none_refresh: crate::server::TestRefreshHandle = None;
    axum::Router::new()
        .route("/api/board", get(crate::transport::rest::get_board))
        .route(
            "/api/stories/{number}",
            get(crate::transport::rest::get_story),
        )
        .route(
            "/api/stories/{number}/accept",
            post(crate::transport::rest::post_accept_story),
        )
        .route(
            "/api/stories/{number}/reject",
            post(crate::transport::rest::post_reject_story),
        )
        .with_state(session)
        .layer(axum::Extension(tracker))
        .layer(axum::Extension(none_refresh))
}

pub(super) async fn post_json(
    router: axum::Router,
    uri: &str,
    body: &str,
) -> axum::http::Response<Body> {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    router.oneshot(request).await.expect("response")
}

pub(super) async fn response_json(response: axum::http::Response<Body>) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&body).expect("parse json")
}

pub(super) fn reaction_toggle(user: &str, emoji: &str, message_key: &str) -> ReactionToggle {
    ReactionToggle {
        channel: "broadcast".to_string(),
        message_key: message_key.to_string(),
        emoji: emoji.to_string(),
        user: user.to_string(),
    }
}
