#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::Extension;
use chrono::{TimeZone, Utc};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use auth as auth_token;
use hub::Hub;
use integrations::in_memory::InMemorySourceAdapter;
use product_management::story::{Story, StoryId, Work};
use sync::SyncOrchestrator;
use transport::auth::{AuthState, gate};
use transport::rpc_routes::post_rpc_tool;

fn seed_story(adapter: &InMemorySourceAdapter, id: u64, title: &str) {
    let story = Story {
        id: StoryId::new(id),
        title: title.to_string(),
        body: Some(
            "## What\n\nfixture\n\n## Acceptance test path\n\ntests/integration_main_modules/rpc_canonical_store.rs\n"
                .to_string(),
        ),
        state: Work::Ready,
        verdicts: None,
        free_comments: None,
        filed_by: "test".to_string(),
        filed_at: Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
        body_etag: None,
        story_type: product_management::story::story_type_from_title(title),
        acceptor: None,
        parent_epic: None,
        tags: Vec::new(),
        remote_updated_at: None,
        state_written_at: None,
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

#[expect(
    clippy::too_many_lines,
    reason = "an end-to-end journey pinned as one narrative — splitting it into helpers would hide the arc the test documents"
)]
#[tokio::test]
async fn two_callers_observe_same_canonical_store_through_rpc() {
    let dir = tempfile::tempdir().expect("tempdir");
    let token_path = dir.path().join("auth_token");
    let token = auth_token::ensure(&token_path).expect("ensure token");

    let adapter = Arc::new(InMemorySourceAdapter::new());
    seed_story(&adapter, 42, "fixture story");
    let hub_dir = tempfile::tempdir().expect("hub dir");
    let (wake_tx, _) = broadcast::channel(64);
    let hub = Arc::new(Hub::new(
        hub_dir.path().to_path_buf(),
        "test".to_string(),
        wake_tx,
    ));
    let app_store = Arc::new(server::store_wiring::build_store(
        hub.clone(),
        storage::open(&hub.dir),
        None,
        CancellationToken::new(),
    ));
    let orch = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        app_store.selector(),
        app_store.dispatcher(),
    ));
    let refresh: Arc<SyncOrchestrator> = orch.clone();
    let test_refresh: Option<Arc<SyncOrchestrator>> = Some(Arc::clone(&refresh));
    orch.ingest_once().await.expect("seed ingest");
    let auth_state = AuthState::new(token.clone());

    let dispatch = app_store.dispatcher();

    let gated = axum::Router::new()
        .route("/api/rpc/{tool}", axum::routing::post(post_rpc_tool))
        .layer(axum::middleware::from_fn_with_state(auth_state, gate));

    let no_race_window: sync::RaceWindowHandle = None;
    let app = axum::Router::new()
        .merge(gated)
        .with_state(Arc::clone(&hub))
        .layer(Extension(test_refresh))
        .layer(Extension(dispatch))
        .layer(Extension(no_race_window))
        .layer(Extension(app_store.selector()))
        .layer(Extension(storage::open(&hub.dir).transitions))
        .layer(Extension(storage::open(&hub.dir).ci_runs));

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

    let res_a = fire_rpc(
        port,
        "start_story",
        "dev",
        &token,
        serde_json::json!({
            "id": 42,
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

    let res_b = fire_rpc(port, "view_board", "pm", &token, serde_json::json!({})).await;
    assert_eq!(res_b.status(), reqwest::StatusCode::OK, "view_board OK");
    let body_b: serde_json::Value = res_b.json().await.expect("json");
    let view = body_b
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("view_board returns message string; body: {body_b}"));

    let in_progress_section = view
        .split("\n\n")
        .find(|section| section.starts_with("IN PROGRESS"))
        .unwrap_or_else(|| panic!("view_board missing IN PROGRESS section; raw: {view}"));
    assert!(
        in_progress_section.contains("#42"),
        "caller B (view_board) must observe caller A's start_story transition; \
         IN PROGRESS section = {in_progress_section:?}"
    );

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
