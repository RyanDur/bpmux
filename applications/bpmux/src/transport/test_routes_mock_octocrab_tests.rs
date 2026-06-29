#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use httpmock::{Method, MockServer};
use serde_json::Value;
use tower::ServiceExt;

use crate::bridge::test_fixtures::issue;
use crate::bridge::tracker::OctocrabStoryTracker;
use crate::integrations::github::GitHubSourceAdapter;
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

async fn fixture_mock_octocrab(server: &MockServer) -> (axum::Router, Arc<dyn RefreshDriver>) {
    let tracker = OctocrabStoryTracker::with_base_url(
        &format!("{}/", server.base_url()),
        "acme".to_string(),
        "bpmux".to_string(),
    )
    .expect("build tracker against mock");
    let adapter = Arc::new(GitHubSourceAdapter::new(tracker));
    let store = Arc::new(BoardStore::new());
    let orch_concrete = SyncOrchestrator::new(store, adapter);
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
        .route("/api/board", axum::routing::get(super::rest::get_board))
        .with_state(fresh_test_session())
        .layer(axum::Extension(orchestrator))
        .layer(axum::Extension(none_tracker));
    (router, orch)
}

async fn read_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

fn issue_with_body(number: u64, title: &str, body: &str) -> Value {
    let mut v = issue(number, title, "open", vec![]);
    v["body"] = Value::String(body.to_string());
    v
}

#[tokio::test]
async fn roundtrip_coherence_create_story_through_real_github_decode_path() {
    let server = MockServer::start();

    let list_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("state", "open");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("state", "closed");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let create_mock = server.mock(|when, then| {
        when.method(Method::POST)
            .path("/repos/acme/bpmux/issues")
            .body_includes("\"title\":\"[Feature] mock-octocrab-roundtrip\"");
        then.status(201)
            .header("content-type", "application/json")
            .body(issue(42, "[Feature] mock-octocrab-roundtrip", "open", vec![]).to_string());
    });

    let (router, _refresh) = fixture_mock_octocrab(&server).await;

    let body = r#"{
        "title":"[Feature] mock-octocrab-roundtrip",
        "user_job":"When testing 3-mode contract-suite, MockOctocrab exercises real github.rs decode path.",
        "observation_falsifier":"reject if create_story → /api/board round-trip diverges from InMemory mode"
    }"#;
    let req = Request::builder()
        .method("POST")
        .uri("/api/test/create_story")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build req");
    let resp = router.clone().oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    assert!(json["message"].as_str().unwrap().contains("#42"));

    list_mock.assert();
    create_mock.assert();

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
    assert_eq!(stories[0]["id"].as_u64(), Some(42));
    assert_eq!(
        stories[0]["title"].as_str(),
        Some("[Feature] mock-octocrab-roundtrip")
    );
    assert_eq!(stories[0]["state"].as_str(), Some("icebox"));
    assert_eq!(stories[0]["gh_state"].as_str(), Some("open"));
}

#[tokio::test]
async fn roundtrip_coherence_user_job_and_falsifier_preserved_through_real_github_decode_path() {
    let server = MockServer::start();

    let composed_body =
        "## User job\n\nUSER_JOB_MARKER_43\n\n## Observation falsifier\n\nFALSIFIER_MARKER_43\n";

    let list_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("state", "open");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("state", "closed");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let create_mock = server.mock(|when, then| {
        when.method(Method::POST)
            .path("/repos/acme/bpmux/issues")
            .body_includes("USER_JOB_MARKER_43")
            .body_includes("FALSIFIER_MARKER_43");
        then.status(201)
            .header("content-type", "application/json")
            .body(
                issue_with_body(43, "[Feature] body-preservation-marker", composed_body)
                    .to_string(),
            );
    });

    let get_story_mock = server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues/43");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                issue_with_body(43, "[Feature] body-preservation-marker", composed_body)
                    .to_string(),
            );
    });

    let (router, refresh) = fixture_mock_octocrab(&server).await;

    let body = r#"{
        "title":"[Feature] body-preservation-marker",
        "user_job":"USER_JOB_MARKER_43",
        "observation_falsifier":"FALSIFIER_MARKER_43"
    }"#;
    let req = Request::builder()
        .method("POST")
        .uri("/api/test/create_story")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build req");
    let resp = router.oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::OK);

    list_mock.assert();
    create_mock.assert();

    let stored_body = refresh.fetch_body(43).await.expect("fetch_body");
    get_story_mock.assert();
    assert!(
        stored_body.contains("USER_JOB_MARKER_43"),
        "user_job preserved verbatim through real github.rs decode path; got:\n{stored_body}"
    );
    assert!(
        stored_body.contains("FALSIFIER_MARKER_43"),
        "observation_falsifier preserved verbatim through real github.rs decode path; got:\n{stored_body}"
    );
}

#[tokio::test]
async fn roundtrip_coherence_rejects_parent_epic_referencing_non_epic_through_real_github_decode_path()
 {
    let server = MockServer::start();

    let list_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("state", "open");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                Value::Array(vec![issue(99, "[Feature] not-an-epic", "open", vec![])]).to_string(),
            );
    });
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("state", "closed");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let (router, refresh) = fixture_mock_octocrab(&server).await;
    assert!(
        matches!(
            refresh.story(99).map(|s| s.story_type()),
            Some(crate::product_management::story::StoryType::Story(
                crate::product_management::story::StoryKind::Feature
            ))
        ),
        "fixture must seed #99 as a non-Epic (Feature) in the canonical store: create_story \
         reads its parent from the store, so without the seed the not-an-Epic rejection would \
         be exercised via the wrong parent-missing path (both map to BAD_REQUEST)"
    );

    let body = r#"{
        "title":"[Feature] child-of-non-epic",
        "user_job":"When testing parent_epic validation, dev needs negative-path coverage via real decode path.",
        "observation_falsifier":"reject if non-Epic parent is silently accepted through MockOctocrab",
        "parent_epic":99
    }"#;
    let req = Request::builder()
        .method("POST")
        .uri("/api/test/create_story")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build req");
    let resp = router.oneshot(req).await.expect("response");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    list_mock.assert();

    let json = read_json(resp).await;
    let err = json["error"].as_str().expect("error string");
    assert!(
        err.contains("not an Epic"),
        "rejection names the type-mismatch reason; got {err}"
    );
}
