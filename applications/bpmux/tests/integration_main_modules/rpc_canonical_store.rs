//! Cross-process integration test for #516's canonical-store RPC layer.
//!
//! Falsifier F7 from the spec: "spawn 2 bridge processes + 1 electron-
//! server; fire start_story from bridge A; assert bridge B + electron-
//! server + renderer all observe state-transition." This test stands up
//! a real `axum::serve` against a real `SyncOrchestrator` + a seeded
//! `InMemorySourceAdapter`, then fires two independent
//! `bridge::rpc_client::dispatch_via_rpc` calls — one as the mutating
//! caller ("bridge A": start_story), one as the read-only observer
//! ("bridge B": view_board) — over real HTTP. A passing assertion
//! demonstrates the canonical store sees both callers' traffic and
//! returns the same data to each.
//!
//! No GitHub access; no Electron renderer; no PTY. Within-process axum +
//! within-process reqwest + a shared file for the auth token. The two
//! `dispatch_via_rpc` calls are still genuinely cross-`Client`: each
//! creates its own reqwest::Client, opens its own TCP connection, and
//! gets its own response — the same shape a separately-spawned bridge
//! process would experience.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::Extension;
use chrono::{TimeZone, Utc};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use bpmux::board::adapters::in_memory::InMemorySourceAdapter;
use bpmux::board::domain::StoryType;
use bpmux::board::{BoardStore, RefreshDriver, Story, StoryState, SyncOrchestrator};
use bpmux::session::{Session, auth_token};
use bpmux::transport::auth::{AuthState, gate};
use bpmux::transport::rpc_routes::post_rpc_tool;

fn seed_story(adapter: &InMemorySourceAdapter, number: u64, title: &str) {
    let story = Story {
        number,
        title: title.to_string(),
        body: Some(
            "## What\n\nfixture\n\n## Acceptance test path\n\ntests/integration_main_modules/rpc_canonical_store.rs\n"
                .to_string(),
        ),
        state: StoryState::Ready,
        is_closed: false,
        story_type: StoryType::Feature,
        tags: Vec::new(),
        parent_epic: None,
        gate_comments: None,
        free_comments: None,
        acceptance_test_path: Some("tests/integration_main_modules/rpc_canonical_store.rs".to_string()),
        filed_by: "test".to_string(),
        filed_at: Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
    };
    adapter.seed_stories(vec![story]);
}

async fn fire_rpc(
    port: u16,
    tool: &str,
    agent: &str,
    token: &str,
    body: serde_json::Value,
) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/api/rpc/{tool}"))
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-BPMUX-Agent", agent)
        .json(&body)
        .send()
        .await
        .expect("post rpc")
}

#[tokio::test]
async fn two_callers_observe_same_canonical_store_through_rpc() {
    let dir = tempfile::tempdir().expect("tempdir");
    let token_path = dir.path().join("auth_token");
    let token = auth_token::ensure(&token_path).expect("ensure token");

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    seed_story(&adapter, 42, "fixture story");
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

    // Caller A: a "dev" bridge fires start_story against the canonical store.
    let res_a = fire_rpc(
        port,
        "start_story",
        "dev",
        &token,
        serde_json::json!({
            "number": 42,
            "pickup_paraphrase": "drive #42 fixture user-job end-to-end",
            "falsifier_reframing": "reject if state never transitions to in-progress",
        }),
    )
    .await;
    let status_a = res_a.status();
    let text_a = res_a.text().await.expect("text");
    assert_eq!(
        status_a,
        reqwest::StatusCode::OK,
        "start_story OK; got {status_a} body: {text_a}"
    );
    let res_a_body: serde_json::Value = serde_json::from_str(&text_a).expect("json");
    assert!(
        res_a_body
            .get("message")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.contains("#42")),
        "start_story response carries story number; got {res_a_body}"
    );

    // Caller B: a different reqwest::Client (independent TCP) fires
    // view_board, simulating a separate bridge process reading the same
    // canonical store. The story it sees must reflect caller A's mutation.
    let res_b = fire_rpc(port, "view_board", "pm", &token, serde_json::json!({})).await;
    assert_eq!(res_b.status(), reqwest::StatusCode::OK, "view_board OK");
    let body_b: serde_json::Value = res_b.json().await.expect("json");
    let view = body_b
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("view_board returns message string; body: {body_b}"));

    // view_board emits a text bucket-by-state listing. Caller B must
    // see story #42 under IN PROGRESS — that's the same canonical
    // state caller A's start_story produced. A failure here would
    // mean the two callers see different boards (the divergence #516
    // exists to eliminate).
    let in_progress_section = view
        .split("\n\n")
        .find(|section| section.starts_with("IN PROGRESS"))
        .unwrap_or_else(|| panic!("view_board missing IN PROGRESS section; raw: {view}"));
    assert!(
        in_progress_section.contains("#42"),
        "caller B (view_board) must observe caller A's start_story transition; \
         IN PROGRESS section = {in_progress_section:?}"
    );

    // Symmetric: a 4xx reaches caller B as a structured error too.
    let res_unknown = fire_rpc(port, "no_such_tool", "dev", &token, serde_json::json!({})).await;
    assert_eq!(res_unknown.status(), reqwest::StatusCode::BAD_REQUEST);
    let body_unknown: serde_json::Value = res_unknown.json().await.expect("json");
    assert!(
        body_unknown
            .get("error")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.contains("unknown tool")),
        "unknown tool error is structured; got {body_unknown}"
    );

    ct.cancel();
    let _ = server.await;
}
