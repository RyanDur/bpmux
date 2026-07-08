#![allow(unused_imports)]

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use httpmock::{Method, MockServer};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use super::{
    board_author, board_issue, board_router, board_test_session, post_json, reaction_toggle,
    response_json, tracker_for,
};

use super::{init_session, seed_messages, seed_mixed_messages, test_app};
use crate::hub;
use crate::message::MessageType;
use crate::transport::rest::*;

use crate::bridge::tracker::OctocrabStoryTracker;
use crate::transport::rest::TrackerHandle;

async fn get_json(router: axum::Router, uri: &str) -> axum::http::Response<Body> {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("build request");
    router.oneshot(request).await.expect("response")
}

#[tokio::test]
async fn get_board_returns_503_when_tracker_missing() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let router = board_router(session, None);

    let response = get_json(router, "/api/board").await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response_json(response).await;
    assert_eq!(body["error"], "tracker not configured");
}

#[tokio::test]
async fn get_board_returns_stories_with_fetched_at() {
    let server = MockServer::start();
    let open_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("state", "open");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![board_issue(1, "first", "open", Some("hello"))]).to_string());
    });
    let closed_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("state", "closed");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![board_issue(2, "second", "closed", None)]).to_string());
    });

    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = get_json(router, "/api/board").await;
    open_mock.assert();
    closed_mock.assert();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let stories = body["stories"].as_array().expect("stories array");
    assert_eq!(stories.len(), 2);
    assert_eq!(stories[0]["number"], 1);
    assert_eq!(stories[0]["title"], "first");
    assert_eq!(stories[0]["state"], "icebox");
    assert_eq!(stories[0]["body"], "hello");
    assert_eq!(stories[1]["body"], "");
    assert_eq!(stories[1]["state"], "accepted");
    assert!(stories[0]["labels"].is_array());
    assert!(stories[0]["assignees"].is_array());
    assert!(stories[0]["created_at"].is_string());
    assert!(
        body["fetched_at"].is_string(),
        "fetched_at timestamp required"
    );
}

#[tokio::test]
async fn get_story_returns_single_issue_with_body() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues/42");
        then.status(200)
            .header("content-type", "application/json")
            .body(board_issue(42, "answer", "open", Some("full body text")).to_string());
    });

    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = get_json(router, "/api/stories/42").await;
    m.assert();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["number"], 42);
    assert_eq!(body["title"], "answer");
    assert_eq!(body["body"], "full body text");
}

#[tokio::test]
async fn get_story_returns_404_when_github_returns_404() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/9999");
        then.status(404)
            .header("content-type", "application/json")
            .body(r#"{"message":"Not Found","documentation_url":"https://docs"}"#);
    });

    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = get_json(router, "/api/stories/9999").await;
    m.assert();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert!(body["error"].as_str().unwrap().contains("#9999 not found"));
}

#[tokio::test]
async fn get_story_returns_503_when_tracker_missing() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let router = board_router(session, None);

    let response = get_json(router, "/api/stories/1").await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn post_accept_story_returns_503_when_orchestrator_missing() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let router = board_router(session, None);

    let response = post_json(
        router,
        "/api/stories/42/accept",
        r#"{"would_file_bug":false}"#,
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn post_accept_story_400_when_would_file_bug_missing() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let server = MockServer::start();
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = post_json(router, "/api/stories/42/accept", "{}").await;
    assert!(
        response.status() == StatusCode::UNPROCESSABLE_ENTITY
            || response.status() == StatusCode::BAD_REQUEST,
        "missing required would_file_bug must 4xx; got {}",
        response.status()
    );
}

#[tokio::test]
async fn post_accept_story_400_when_would_file_bug_true_without_force() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let server = MockServer::start();
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = post_json(
        router,
        "/api/stories/42/accept",
        r#"{"would_file_bug":true}"#,
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert!(
        body["error"]
            .as_str()
            .unwrap_or("")
            .contains("reject_story"),
        "would_file_bug=true must route the user to reject_story; got {body}"
    );
}

#[tokio::test]
async fn post_reject_story_400_on_empty_note() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let server = MockServer::start();
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = post_json(router, "/api/stories/7/reject", r#"{"note":""}"#).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["error"], "note must not be empty");
}

#[tokio::test]
async fn post_reject_story_400_on_whitespace_note() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let server = MockServer::start();
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = post_json(router, "/api/stories/7/reject", r#"{"note":"   \n\t "}"#).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_reject_story_returns_503_when_tracker_missing() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let router = board_router(session, None);

    let response = post_json(router, "/api/stories/7/reject", r#"{"note":"x"}"#).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
