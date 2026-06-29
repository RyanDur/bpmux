#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use crate::bridge;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::server::OrchestratorHandle;
use crate::session::{self, BROADCAST_CHANNEL_CAPACITY};

fn fresh_test_session() -> Arc<session::Session> {
    let dir = tempfile::tempdir().expect("tempdir");
    let (wake_tx, _wake_rx) = tokio::sync::broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
    Arc::new(session::Session::new(
        dir.keep(),
        "test:owner".to_string(),
        wake_tx,
    ))
}

async fn fixture_test_mode_on() -> (axum::Router, Arc<dyn RefreshDriver>, Arc<BoardStore>) {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch_concrete = SyncOrchestrator::new(store.clone(), adapter);
    orch_concrete
        .ingest_once()
        .await
        .expect("seed-empty ingest");
    let orch: Arc<dyn RefreshDriver> = Arc::new(orch_concrete);
    let orchestrator: OrchestratorHandle = Some(orch.clone());
    let none_tracker: super::rest::TrackerHandle = None;
    let router = axum::Router::new()
        .route(
            "/api/test/create_story",
            axum::routing::post(super::test_routes::post_test_create_story),
        )
        .route(
            "/api/test/comment_on_story",
            axum::routing::post(super::test_routes::post_test_comment_on_story),
        )
        .route(
            "/api/test/close_story",
            axum::routing::post(super::test_routes::post_test_close_story),
        )
        .route(
            "/api/test/view_velocity",
            axum::routing::post(super::test_routes::post_test_view_velocity),
        )
        .route("/api/board", axum::routing::get(super::rest::get_board))
        .with_state(fresh_test_session())
        .layer(axum::Extension(orchestrator))
        .layer(axum::Extension(none_tracker));
    (router, orch, store)
}

fn fixture_test_mode_off() -> axum::Router {
    let none_refresh: OrchestratorHandle = None;
    axum::Router::new()
        .route(
            "/api/test/create_story",
            axum::routing::post(super::test_routes::post_test_create_story),
        )
        .with_state(fresh_test_session())
        .layer(axum::Extension(none_refresh))
}

async fn post_create_story(router: axum::Router, body: &str) -> axum::http::Response<Body> {
    let req = Request::builder()
        .method("POST")
        .uri("/api/test/create_story")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build req");
    router.oneshot(req).await.expect("response")
}

async fn read_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

#[tokio::test]
async fn response_correctness_happy_path_returns_message_with_assigned_number() {
    let (router, _refresh, _store) = fixture_test_mode_on().await;
    let body = r#"{
        "title":"[Feature] response-correctness-happy",
        "user_job":"When testing, dev needs response-shape verification.",
        "observation_falsifier":"reject if message omits the assigned number"
    }"#;
    let resp = post_create_story(router, body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    let msg = json["message"].as_str().expect("message string");
    assert!(
        msg.contains("story created:"),
        "happy-path message names creation; got {msg}"
    );
    assert!(
        msg.contains("#1"),
        "first story in fresh fixture is #1; got {msg}"
    );
}

#[tokio::test]
async fn response_correctness_rejects_empty_observation_falsifier() {
    let (router, _refresh, _store) = fixture_test_mode_on().await;
    let body = r#"{
        "title":"[Feature] x",
        "user_job":"When testing, ...",
        "observation_falsifier":""
    }"#;
    let resp = post_create_story(router, body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = read_json(resp).await;
    let err = json["error"].as_str().expect("error string");
    assert!(
        err.contains("observation_falsifier"),
        "rejection names the missing field; got {err}"
    );
}

#[tokio::test]
async fn response_correctness_rejects_empty_user_job() {
    let (router, _refresh, _store) = fixture_test_mode_on().await;
    let body = r#"{
        "title":"[Feature] x",
        "user_job":"",
        "observation_falsifier":"reject if user_job is unspecified"
    }"#;
    let resp = post_create_story(router, body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = read_json(resp).await;
    let err = json["error"].as_str().expect("error string");
    assert!(
        err.contains("user_job"),
        "rejection names the missing field; got {err}"
    );
}

#[tokio::test]
async fn response_correctness_returns_503_when_test_mode_disabled() {
    let router = fixture_test_mode_off();
    let body = r#"{
        "title":"[Feature] x",
        "user_job":"y",
        "observation_falsifier":"z"
    }"#;
    let resp = post_create_story(router, body).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json = read_json(resp).await;
    let err = json["error"].as_str().expect("error string");
    assert!(
        err.contains("BPMUX_TEST_MODE"),
        "503 explains test-mode requirement; got {err}"
    );
}

#[tokio::test]
async fn roundtrip_coherence_view_board_reflects_new_story() {
    let (router, refresh, _store) = fixture_test_mode_on().await;
    let body = r#"{
        "title":"[Feature] roundtrip-marker-A",
        "user_job":"When testing, dev needs roundtrip verification.",
        "observation_falsifier":"reject if view_board does not reflect new story"
    }"#;
    let resp = post_create_story(router, body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let view = bridge::dispatch_view_board(serde_json::Map::new(), Some(&refresh))
        .await
        .expect("view_board returns Ok");
    assert!(
        view.contains("roundtrip-marker-A"),
        "view_board output reflects new story title; got:\n{view}"
    );
    assert!(
        view.contains("#1"),
        "view_board output names the assigned number; got:\n{view}"
    );
}

#[tokio::test]
async fn roundtrip_coherence_user_job_and_falsifier_preserved_in_body() {
    let (router, refresh, _store) = fixture_test_mode_on().await;
    let body = r#"{
        "title":"[Feature] roundtrip-body-marker",
        "user_job":"USER_JOB_MARKER_42",
        "observation_falsifier":"FALSIFIER_MARKER_42"
    }"#;
    let resp = post_create_story(router, body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let stories = refresh.all_stories();
    assert_eq!(stories.len(), 1, "exactly one story present");

    let stored_body = refresh
        .fetch_body(stories[0].id.value())
        .await
        .expect("fetch_body");
    assert!(
        stored_body.contains("USER_JOB_MARKER_42"),
        "user_job preserved verbatim in stored body; got:\n{stored_body}"
    );
    assert!(
        stored_body.contains("FALSIFIER_MARKER_42"),
        "observation_falsifier preserved verbatim in stored body; got:\n{stored_body}"
    );
}

#[tokio::test]
async fn rest_get_board_auto_reingests_when_orchestrator_store_is_empty() {
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, StoryId, StoryState};
    use chrono::{TimeZone, Utc};

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap();
    adapter.seed_stories(vec![Story::new_minimal(
        StoryId::new(5050),
        "post-restart-rest-recovery".into(),
        StoryState::Accepted,
        ts,
        "dev".into(),
    )]);
    let orch_concrete = SyncOrchestrator::new(store.clone(), adapter);
    let orch: Arc<dyn RefreshDriver> = Arc::new(orch_concrete);
    let orchestrator: OrchestratorHandle = Some(orch.clone());
    let none_tracker: super::rest::TrackerHandle = None;
    let router = axum::Router::new()
        .route("/api/board", axum::routing::get(super::rest::get_board))
        .with_state(fresh_test_session())
        .layer(axum::Extension(orchestrator))
        .layer(axum::Extension(none_tracker));

    assert!(
        orch.all_stories().is_empty(),
        "store starts empty (simulates startup-ingest-failed or post-restart cache-loss)"
    );

    let req = Request::builder()
        .method("GET")
        .uri("/api/board")
        .body(Body::empty())
        .expect("build req");
    let resp = router.oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    let stories = json["stories"].as_array().expect("stories array");
    assert_eq!(
        stories.len(),
        1,
        "rest get_board should have triggered re-ingest from adapter; got body: {json}"
    );
    assert_eq!(stories[0]["id"].as_u64(), Some(5050));
    assert!(
        !orch.all_stories().is_empty(),
        "post-call store populated; recovery completed"
    );
}

#[tokio::test]
async fn roundtrip_coherence_api_board_reflects_new_story_in_test_mode() {
    let (router, _refresh, _store) = fixture_test_mode_on().await;
    let body = r#"{
        "title":"[Feature] http-roundtrip-marker",
        "user_job":"When testing, dev needs HTTP /api/board to reflect mutations in test mode.",
        "observation_falsifier":"reject if /api/board does not show new story when BPMUX_TEST_MODE=1"
    }"#;
    let resp = post_create_story(router.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method("GET")
        .uri("/api/board")
        .body(Body::empty())
        .expect("build req");
    let resp = router.oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;

    let stories = json["stories"].as_array().expect("stories array");
    assert_eq!(stories.len(), 1);
    assert_eq!(stories[0]["id"].as_u64(), Some(1));
    assert_eq!(
        stories[0]["title"].as_str(),
        Some("[Feature] http-roundtrip-marker")
    );
    assert_eq!(stories[0]["state"].as_str(), Some("icebox"));
    assert_eq!(stories[0]["gh_state"].as_str(), Some("open"));
    assert!(json["fetched_at"].is_string());
}

#[tokio::test]
async fn roundtrip_coherence_rejects_parent_epic_referencing_non_epic() {
    let (router, refresh, _store) = fixture_test_mode_on().await;
    let feature = r#"{
        "title":"[Feature] not-an-epic",
        "user_job":"When testing, dev needs negative-path coverage.",
        "observation_falsifier":"reject if non-Epic parent is accepted silently"
    }"#;
    let resp = post_create_story(router.clone(), feature).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let parent_number = refresh.all_stories()[0].id.value();

    let child = format!(
        r#"{{
            "title":"[Feature] child-of-feature",
            "user_job":"When testing, dev needs ...",
            "observation_falsifier":"reject ...",
            "parent_epic":{parent_number}
        }}"#
    );
    let resp = post_create_story(router, &child).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = read_json(resp).await;
    let err = json["error"].as_str().expect("error string");
    assert!(
        err.contains("not an Epic"),
        "rejection names the type-mismatch reason; got {err}"
    );
    assert_eq!(
        refresh.all_stories().len(),
        1,
        "rejected request did not create a child"
    );
}

async fn post_comment_on_story(router: axum::Router, body: &str) -> axum::http::Response<Body> {
    let req = Request::builder()
        .method("POST")
        .uri("/api/test/comment_on_story")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build req");
    router.oneshot(req).await.expect("response")
}

#[tokio::test]
async fn comment_on_story_response_correctness_happy_path_returns_message() {
    let (router, refresh, _store) = fixture_test_mode_on().await;
    let create = r#"{
        "title":"[Feature] comment-target",
        "user_job":"When testing comment_on_story, dev needs a target story.",
        "observation_falsifier":"reject if comment-on-non-existent-story silently succeeds"
    }"#;
    let resp = post_create_story(router.clone(), create).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let target_number = refresh.all_stories()[0].id.value();

    let comment = format!(
        r#"{{
            "number":{target_number},
            "body":"first comment marker"
        }}"#
    );
    let resp = post_comment_on_story(router, &comment).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    let msg = json["message"].as_str().expect("message string");
    assert!(
        msg.contains(&format!("#{target_number}")),
        "message names target story number; got {msg}"
    );
    assert!(
        msg.contains("commented"),
        "happy-path message names the commented affordance; got {msg}"
    );
}

#[tokio::test]
async fn comment_on_story_response_correctness_rejects_empty_body() {
    let (router, refresh, _store) = fixture_test_mode_on().await;
    let create = r#"{
        "title":"[Feature] comment-target",
        "user_job":"When testing empty-body validation, dev needs a target story.",
        "observation_falsifier":"reject if empty-body comment is silently accepted"
    }"#;
    let resp = post_create_story(router.clone(), create).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let target_number = refresh.all_stories()[0].id.value();

    let comment = format!(
        r#"{{
            "number":{target_number},
            "body":""
        }}"#
    );
    let resp = post_comment_on_story(router, &comment).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = read_json(resp).await;
    let err = json["error"].as_str().expect("error string");
    assert!(
        err.contains("body"),
        "rejection names the missing field; got {err}"
    );
}

#[tokio::test]
async fn comment_on_story_roundtrip_view_comments_reflects_new_comment() {
    let (router, refresh, store) = fixture_test_mode_on().await;
    let create = r#"{
        "title":"[Feature] comment-roundtrip",
        "user_job":"When testing comment roundtrip, dev needs a target story.",
        "observation_falsifier":"reject if view_comments doesn't reflect appended comment"
    }"#;
    let resp = post_create_story(router.clone(), create).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let target_number = refresh.all_stories()[0].id.value();

    let marker = "ROUNDTRIP_MARKER_C1";
    let comment = format!(
        r#"{{
            "number":{target_number},
            "body":"{marker}"
        }}"#
    );
    let resp = post_comment_on_story(router, &comment).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let mut args = serde_json::Map::new();
    args.insert(
        "number".to_string(),
        serde_json::Value::Number(target_number.into()),
    );
    let _ = store;
    let view = bridge::dispatch_view_comments(args, Some(&refresh))
        .await
        .expect("view_comments returns Ok");
    assert!(
        view.contains(marker),
        "view_comments output reflects new comment; got:\n{view}"
    );
}

async fn post_close_story(router: axum::Router, body: &str) -> axum::http::Response<Body> {
    let req = Request::builder()
        .method("POST")
        .uri("/api/test/close_story")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build req");
    router.oneshot(req).await.expect("response")
}

#[tokio::test]
async fn close_story_response_correctness_happy_path_returns_message() {
    let (router, refresh, _store) = fixture_test_mode_on().await;
    let create = r#"{
        "title":"[Feature] close-target",
        "user_job":"When testing close_story, dev needs a target story.",
        "observation_falsifier":"reject if close on non-existent story silently succeeds"
    }"#;
    let resp = post_create_story(router.clone(), create).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let target_number = refresh.all_stories()[0].id.value();

    let close = format!(
        r#"{{
            "number":{target_number},
            "reason":"superseded by alternate path"
        }}"#
    );
    let resp = post_close_story(router, &close).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    let msg = json["message"].as_str().expect("message string");
    assert!(
        msg.contains(&format!("#{target_number}")),
        "message names target story; got {msg}"
    );
    assert!(
        msg.contains("closed"),
        "happy-path message names the closed affordance; got {msg}"
    );
    assert!(
        msg.contains("superseded by alternate path"),
        "message includes the close-reason for audit trail; got {msg}"
    );
}

#[tokio::test]
async fn close_story_response_correctness_rejects_empty_reason() {
    let (router, refresh, _store) = fixture_test_mode_on().await;
    let create = r#"{
        "title":"[Feature] close-target",
        "user_job":"When testing empty-reason validation, dev needs a target story.",
        "observation_falsifier":"reject if empty-reason close is silently accepted"
    }"#;
    let resp = post_create_story(router.clone(), create).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let target_number = refresh.all_stories()[0].id.value();

    let close = format!(
        r#"{{
            "number":{target_number},
            "reason":""
        }}"#
    );
    let resp = post_close_story(router, &close).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = read_json(resp).await;
    let err = json["error"].as_str().expect("error string");
    assert!(
        err.contains("\"missing_evidence\":[\"reason-cite\"]"),
        "structured rejection cites missing reason-cite; got {err}"
    );
    assert!(
        err.contains("\"attempted_transition\":\"close\""),
        "rejection cites attempted_transition; got {err}"
    );
}

#[tokio::test]
async fn close_story_roundtrip_api_board_reflects_closed_state() {
    let (router, refresh, _store) = fixture_test_mode_on().await;
    let create = r#"{
        "title":"[Feature] close-roundtrip",
        "user_job":"When testing close_story roundtrip, dev needs a target story.",
        "observation_falsifier":"reject if /api/board does not reflect closed state after close_story"
    }"#;
    let resp = post_create_story(router.clone(), create).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let target_number = refresh.all_stories()[0].id.value();

    let close = format!(
        r#"{{
            "number":{target_number},
            "reason":"close-roundtrip-marker",
            "disposition":"accept"
        }}"#
    );
    let resp = post_close_story(router.clone(), &close).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method("GET")
        .uri("/api/board")
        .body(Body::empty())
        .expect("build req");
    let resp = router.oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
    let board = read_json(resp).await;
    let stories = board["stories"].as_array().expect("stories array");
    assert_eq!(stories.len(), 1);
    assert_eq!(
        stories[0]["state"].as_str(),
        Some("icebox"),
        "post-#505 boundary-principle: close_story sets is_closed=true but preserves Story.state; the GH-side close → state=Accepted translation now happens at INGEST not at serialization (or via explicit state-transition tools like accept_story)"
    );
    assert_eq!(stories[0]["gh_state"].as_str(), Some("closed"));
}

async fn post_view_velocity(router: axum::Router, body: &str) -> axum::http::Response<Body> {
    let req = Request::builder()
        .method("POST")
        .uri("/api/test/view_velocity")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build req");
    router.oneshot(req).await.expect("response")
}

#[tokio::test]
async fn view_velocity_response_correctness_empty_corpus_returns_metrics_shape() {
    let (router, _refresh, _store) = fixture_test_mode_on().await;
    let resp = post_view_velocity(router, r#"{"messages":[]}"#).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    let metrics = &json["metrics"];
    assert!(metrics.is_object(), "metrics is JSON object; got {metrics}");
    assert!(metrics.get("delivered_per_day").is_some());
    assert!(metrics.get("sparse").is_some());
    assert_eq!(
        metrics["sparse"].as_bool(),
        Some(true),
        "empty corpus is sparse"
    );
}

#[tokio::test]
async fn view_velocity_response_correctness_returns_503_when_test_mode_disabled() {
    let none_refresh: OrchestratorHandle = None;
    let router = axum::Router::new()
        .route(
            "/api/test/view_velocity",
            axum::routing::post(super::test_routes::post_test_view_velocity),
        )
        .with_state(fresh_test_session())
        .layer(axum::Extension(none_refresh));
    let resp = post_view_velocity(router, r#"{"messages":[]}"#).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json = read_json(resp).await;
    let err = json["error"].as_str().expect("error string");
    assert!(
        err.contains("BPMUX_TEST_MODE"),
        "503 explains test-mode requirement; got {err}"
    );
}

#[tokio::test]
async fn view_velocity_roundtrip_coherence_lifecycle_corpus_yields_delivered_count() {
    let (router, _refresh, _store) = fixture_test_mode_on().await;
    let create = r#"{
        "title":"[Feature] velocity-target",
        "user_job":"When testing view_velocity roundtrip, dev needs a delivered feature.",
        "observation_falsifier":"reject if view_velocity does not see the delivered event"
    }"#;
    let resp = post_create_story(router.clone(), create).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let now = chrono::Utc::now();
    let started_ts = now - chrono::Duration::hours(2);
    let delivered_ts = now - chrono::Duration::hours(1);
    let corpus = serde_json::json!({
        "messages": [
            {
                "from": "dev:test",
                "text": "#1 started by dev",
                "ts": started_ts.to_rfc3339(),
                "message_type": "chat"
            },
            {
                "from": "dev:test",
                "text": "#1 delivered: verified observation",
                "ts": delivered_ts.to_rfc3339(),
                "message_type": "chat"
            }
        ]
    });
    let resp = post_view_velocity(router, &corpus.to_string()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    let metrics = &json["metrics"];
    let dpd = metrics["delivered_per_day"]
        .as_f64()
        .expect("delivered_per_day numeric");
    assert!(
        dpd > 0.0,
        "delivered lifecycle event yields non-zero delivered_per_day; got {dpd}"
    );
}
