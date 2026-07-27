#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use httpmock::MockServer;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use crate::rest::{TrackerHandle, *};
use integrations::github::tracker::OctocrabStoryTracker;
use message::MessageType;

pub fn test_app(dir: &TempDir) -> (axum::Router, broadcast::Sender<message::Message>) {
    test_app_with_routing(dir, &[])
}

pub fn test_app_with_routing(
    dir: &TempDir,
    registrations: &[(&str, &str)],
) -> (axum::Router, broadcast::Sender<message::Message>) {
    let (tx, _rx) = broadcast::channel(16);
    let hub = Arc::new(hub::Hub::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx.clone(),
    ));
    let seeded = store_test_support::SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    for (display, user_id) in registrations {
        seeded.dispatch(
            store::routing::RoutingAction::Registered {
                user_id: (*user_id).to_string(),
                display: (*display).to_string(),
                role: None,
            }
            .into(),
        );
    }
    let router = axum::Router::new()
        .route(
            "/api/messages",
            axum::routing::get(get_messages).post(post_message),
        )
        .layer(axum::Extension(seeded.select()))
        .with_state(hub);
    std::mem::forget(seeded);
    (router, tx)
}

pub fn init_session(dir: &TempDir) -> PathBuf {
    let messages_file = dir.path().join("messages-current.jsonl");
    std::fs::write(&messages_file, "").expect("create messages file");
    dir.path().to_path_buf()
}

pub fn seed_messages(dir: &TempDir, count: usize) -> Arc<hub::Hub> {
    let (tx, _rx) = broadcast::channel(16);
    let hub = Arc::new(hub::Hub::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    for i in 0..count {
        let msg = message::Message {
            from: "user".to_string(),
            text: format!("msg {i}"),
            ts: chrono::Utc::now(),
            ..message::Message::default()
        };
        hub.broadcast(&msg).expect("broadcast");
    }
    hub
}

pub fn seed_mixed_messages(dir: &TempDir) -> Arc<hub::Hub> {
    let (tx, _rx) = broadcast::channel(16);
    let hub = Arc::new(hub::Hub::new(
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
        let msg = message::Message {
            from: "dev".to_string(),
            text: text.to_string(),
            ts: chrono::Utc::now(),
            message_type: msg_type,
            ..message::Message::default()
        };
        hub.broadcast(&msg).expect("broadcast");
    }
    hub
}

pub fn board_test_session(dir: &TempDir) -> (Arc<hub::Hub>, broadcast::Sender<message::Message>) {
    let (tx, _rx) = broadcast::channel(16);
    let hub = Arc::new(hub::Hub::new(
        dir.path().to_path_buf(),
        "owner".to_string(),
        tx.clone(),
    ));
    (hub, tx)
}

pub fn board_author() -> Value {
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

pub fn board_issue(number: u64, title: &str, state: &str, body: Option<&str>) -> Value {
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

pub fn tracker_for(server: &MockServer) -> integrations::github::GitHubSourceAdapter {
    let inner = OctocrabStoryTracker::with_base_url(
        &format!("{}/", server.base_url()),
        "acme".to_string(),
        "bpmux".to_string(),
    )
    .expect("build tracker against mock");
    integrations::github::GitHubSourceAdapter::new(inner)
}

pub fn board_router(hub: Arc<hub::Hub>, tracker: TrackerHandle) -> axum::Router {
    use axum::routing::{get, post};
    let none_refresh: sync::OrchestratorHandle = None;
    let seeded = store_test_support::SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    let select = seeded.select();
    std::mem::forget(seeded);
    axum::Router::new()
        .route("/api/board", get(crate::rest::get_board))
        .route("/api/stories/{number}", get(crate::rest::get_story))
        .route(
            "/api/stories/{number}/accept",
            post(crate::rest::post_accept_story),
        )
        .route(
            "/api/stories/{number}/reject",
            post(crate::rest::post_reject_story),
        )
        .with_state(hub)
        .layer(axum::Extension(
            integrations::github::live::TrackerCell::seeded(tracker),
        ))
        .layer(axum::Extension(none_refresh))
        .layer(axum::Extension(select))
}

pub async fn post_json(router: axum::Router, uri: &str, body: &str) -> axum::http::Response<Body> {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    router.oneshot(request).await.expect("response")
}

pub async fn response_json(response: axum::http::Response<Body>) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&body).expect("parse json")
}

pub fn reaction_toggle(user: &str, emoji: &str, message_key: &str) -> ReactionToggle {
    ReactionToggle {
        channel: "broadcast".to_string(),
        message_key: message_key.to_string(),
        emoji: emoji.to_string(),
        user: user.to_string(),
    }
}
