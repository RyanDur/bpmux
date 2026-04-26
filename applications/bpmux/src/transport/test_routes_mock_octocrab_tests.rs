//! Integration tests for the `/api/test/*` HTTP surface against
//! the MockOctocrab adapter mode (#476).
//!
//! Per the 3-mode contract-test-suite design (Pact-style):
//!
//!   - InMemorySourceAdapter — fast/isolation; covered by
//!     `test_routes_tests` (commit 3).
//!   - **MockOctocrabAdapter** — GitHubSourceAdapter pointed at a
//!     httpmock server; exercises the real `decode_story_metadata`
//!     path that the InMemoryAdapter bypasses. THIS FILE.
//!   - GitHubSourceAdapter-VCR — recorded responses (deferred).
//!
//! Same assertions must pass identically across all three modes per
//! `feedback_test_fixtures_must_not_short_circuit_critical_substrate`.
//! First-pass commit covers a single round-trip assertion against
//! the new mode; remaining assertions backfill in subsequent commits.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use httpmock::{Method, MockServer};
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::board::adapters::github::GitHubSourceAdapter;
use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::bridge::tracker::OctocrabStoryTracker;
use crate::server::TestRefreshHandle;

fn author() -> Value {
    json!({
        "login": "ryan",
        "id": 1,
        "node_id": "MDQ6VXNlcjE=",
        "avatar_url": "https://example.test/avatar.png",
        "gravatar_id": "",
        "url": "https://example.test/users/ryan",
        "html_url": "https://example.test/ryan",
        "followers_url": "https://example.test/ryan/followers",
        "following_url": "https://example.test/ryan/following{/other_user}",
        "gists_url": "https://example.test/ryan/gists{/gist_id}",
        "starred_url": "https://example.test/ryan/starred{/owner}{/repo}",
        "subscriptions_url": "https://example.test/ryan/subscriptions",
        "organizations_url": "https://example.test/ryan/orgs",
        "repos_url": "https://example.test/ryan/repos",
        "events_url": "https://example.test/ryan/events{/privacy}",
        "received_events_url": "https://example.test/ryan/received_events",
        "type": "User",
        "site_admin": false
    })
}

fn issue(number: u64, title: &str, state: &str, labels: Vec<&str>) -> Value {
    let labels_json: Vec<Value> = labels
        .into_iter()
        .map(|name| {
            json!({
                "id": 1,
                "node_id": "MDU6TGFiZWwx",
                "url": format!("https://example.test/labels/{name}"),
                "name": name,
                "color": "cccccc",
                "default": false
            })
        })
        .collect();
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
        "body": null,
        "user": author(),
        "labels": labels_json,
        "assignees": [],
        "author_association": "OWNER",
        "locked": false,
        "comments": 0,
        "created_at": "2026-04-18T00:00:00Z",
        "updated_at": "2026-04-18T00:00:00Z"
    })
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
    let test_refresh: TestRefreshHandle = Some(orch.clone());
    let none_tracker: super::rest::TrackerHandle = None;
    let router = axum::Router::new()
        .route(
            "/api/test/create_story",
            axum::routing::post(super::test_routes::post_test_create_story),
        )
        .route("/api/board", axum::routing::get(super::rest::get_board))
        .layer(axum::Extension(test_refresh))
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
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
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
    assert_eq!(stories[0]["number"].as_u64(), Some(42));
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
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
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
