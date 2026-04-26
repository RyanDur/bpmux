//! Integration tests for the `/api/test/*` HTTP surface (#476).
//!
//! Three sections per the designer-authored template (broadcast 02:18 UTC):
//!
//! 1. response-correctness: the HTTP route's request/response shape is
//!    what callers expect (status codes + error-message naming).
//! 2. roundtrip-coherence: a write through the route is observable via
//!    the production read path (`handle_view_board` / `RefreshDriver`).
//! 3. contract-equality: identical assertions must pass against every
//!    adapter mode (InMemorySourceAdapter, MockOctocrab, GitHubSourceAdapter-VCR).
//!    First-pass commit covers InMemorySourceAdapter only; subsequent
//!    commits add MockOctocrab + GH-VCR runners that re-execute these
//!    same assertions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::bridge;
use crate::server::TestRefreshHandle;

async fn fixture_test_mode_on() -> (axum::Router, Arc<dyn RefreshDriver>, Arc<BoardStore>) {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch_concrete = SyncOrchestrator::new(store.clone(), adapter);
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
        .route(
            "/api/test/comment_on_story",
            axum::routing::post(super::test_routes::post_test_comment_on_story),
        )
        .route(
            "/api/test/close_story",
            axum::routing::post(super::test_routes::post_test_close_story),
        )
        .route("/api/board", axum::routing::get(super::rest::get_board))
        .layer(axum::Extension(test_refresh))
        .layer(axum::Extension(none_tracker));
    (router, orch, store)
}

fn fixture_test_mode_off() -> axum::Router {
    let none_refresh: TestRefreshHandle = None;
    axum::Router::new()
        .route(
            "/api/test/create_story",
            axum::routing::post(super::test_routes::post_test_create_story),
        )
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

    let view = bridge::dispatch_test_view_board(Some(&refresh))
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
        .fetch_body(stories[0].number)
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
    assert_eq!(stories[0]["number"].as_u64(), Some(1));
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

    let parent_number = refresh.all_stories()[0].number;

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
    let target_number = refresh.all_stories()[0].number;

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
    let target_number = refresh.all_stories()[0].number;

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
    let target_number = refresh.all_stories()[0].number;

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
    let view = bridge::dispatch_test_view_comments(args, Some(&store), Some(&refresh))
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
    let target_number = refresh.all_stories()[0].number;

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
    let target_number = refresh.all_stories()[0].number;

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
        err.contains("reason"),
        "rejection names the missing field; got {err}"
    );
    assert!(
        err.contains("audit"),
        "rejection cites audit-trail rationale per close discipline; got {err}"
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
    let target_number = refresh.all_stories()[0].number;

    let close = format!(
        r#"{{
            "number":{target_number},
            "reason":"close-roundtrip-marker"
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
        Some("accepted"),
        "is_closed=true maps to renderer state=accepted per story_to_board_json"
    );
    assert_eq!(stories[0]["gh_state"].as_str(), Some("closed"));
}
