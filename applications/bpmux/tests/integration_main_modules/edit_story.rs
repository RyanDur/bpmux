//! MCP `edit_story` cascade test for #530.
//!
//! Stands up a real `axum::serve` against the production
//! `/api/rpc/edit_story` route, then drives a body-edit through
//! `reqwest` and asserts the canonical store + InMemoryAdapter both
//! observe the new body. Closes the gh-bypass class today's session
//! repeatedly hit on ATP-corrections.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::Extension;
use chrono::{TimeZone, Utc};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use bpmux::auth as auth_token;
use bpmux::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use bpmux::product_management::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use bpmux::product_management::story::{Story, StoryState, StoryType};
use bpmux::session::Session;
use bpmux::transport::auth::{AuthState, gate};
use bpmux::transport::rpc_routes::post_rpc_tool;

fn seed_story(adapter: &InMemorySourceAdapter, number: u64) {
    let story = Story {
        number,
        title: "fixture story".to_string(),
        body: Some("## What\n\noriginal body\n".to_string()),
        state: StoryState::Ready,
        is_closed: false,
        story_type: StoryType::Feature,
        gate_comments: None,
        free_comments: None,
        acceptance_test_path: Some("tests/integration_main_modules/edit_story.rs".to_string()),
        filed_by: "test".to_string(),
        filed_at: Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
        raw_labels: Vec::new(),
        body_etag: None,
    };
    adapter.seed_stories(vec![story]);
    adapter.seed_body(number, "## What\n\noriginal body\n".to_string());
}

#[tokio::test]
async fn edit_story_replaces_body_at_canonical_store() {
    let dir = tempfile::tempdir().expect("tempdir");
    let token_path = dir.path().join("auth_token");
    let token = auth_token::ensure(&token_path).expect("ensure token");

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    seed_story(&adapter, 99);
    let orch = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.ingest_once().await.expect("seed ingest");
    let refresh: Arc<dyn RefreshDriver> = orch.clone();
    let test_refresh: Option<Arc<dyn RefreshDriver>> = Some(Arc::clone(&refresh));

    let session_dir = tempfile::tempdir().expect("session dir");
    let (wake_tx, _) = broadcast::channel(64);
    let session = Arc::new(Session::new(
        session_dir.path().to_path_buf(),
        "test".to_string(),
        wake_tx,
    ));
    let auth_state = AuthState::new(token.clone());

    let gated = axum::Router::new()
        .route("/api/rpc/{tool}", axum::routing::post(post_rpc_tool))
        .layer(axum::middleware::from_fn_with_state(auth_state, gate));
    let app = axum::Router::new()
        .merge(gated)
        .with_state(Arc::clone(&session))
        .layer(Extension(test_refresh));

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();

    let ct = CancellationToken::new();
    let server_ct = ct.clone();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(async move { server_ct.cancelled_owned().await })
        .await
        .expect("serve");
    });

    let new_body = "## What\n\nrewritten body via edit_story\n\n## Acceptance test path\n\ntests/integration_main_modules/edit_story.rs\n";
    let res = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/api/rpc/edit_story"))
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-BPMUX-Agent", "pm")
        .json(&serde_json::json!({
            "number": 99,
            "body": new_body,
        }))
        .send()
        .await
        .expect("post edit_story");

    assert_eq!(res.status(), reqwest::StatusCode::OK);
    let body_resp: serde_json::Value = res.json().await.expect("json");
    assert!(
        body_resp
            .get("message")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.contains("body updated")),
        "edit_story response carries success message; got {body_resp}"
    );

    let stored = store.story(99).expect("story still in store");
    assert_eq!(
        stored.body.as_deref(),
        Some(new_body),
        "BoardStore body must reflect the edit post-cascade"
    );

    let non_pm = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/api/rpc/edit_story"))
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-BPMUX-Agent", "designer")
        .json(&serde_json::json!({
            "number": 99,
            "body": "would-be-rejected",
        }))
        .send()
        .await
        .expect("post edit_story as designer");
    assert_eq!(
        non_pm.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "non-PM (and non-owner) role must be rejected from edit_story"
    );

    ct.cancel();
    let _ = server.await;
}
