//! Contract tests for `OctocrabStoryTracker` against a local httpmock
//! server. These tests run on every `cargo test` — no credentials
//! required, no live-network side effects — so pre-push + CI catch
//! drift between bpmux's use of octocrab and the recorded REST
//! contract.
//!
//! Re-record against the live API via `scripts/rerecord-fixtures.sh`
//! when GitHub changes a response shape or bpmux starts calling a
//! new endpoint; feature-gated integration suite (#299) stays as the
//! drift oracle that surfaces "recorded contract no longer matches
//! reality."
//!
//! Each test spins its own server + tracker so there's no shared
//! state across tests — httpmock's async server binds an ephemeral
//! port per instance. `with_base_url` points octocrab at the mock.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_fixtures::{author, issue};
use super::tracker::{OctocrabStoryTracker, StoryTracker};
use httpmock::{Method, MockServer};
use serde_json::{Value, json};

fn mock_tracker(server: &MockServer) -> OctocrabStoryTracker {
    OctocrabStoryTracker::with_base_url(
        &format!("{}/", server.base_url()),
        "acme".to_string(),
        "bpmux".to_string(),
    )
    .expect("build tracker against mock")
}

fn comment(id: u64, body: &str) -> Value {
    json!({
        "id": id,
        "node_id": "MDEyOklzc3VlQ29tbWVudDE=",
        "url": format!("https://example.test/comments/{id}"),
        "html_url": format!("https://example.test/web/comment/{id}"),
        "body": body,
        "author_association": "OWNER",
        "user": author(),
        "created_at": "2026-04-18T00:00:00Z"
    })
}

fn label(name: &str) -> Value {
    json!({
        "id": 1,
        "node_id": "MDU6TGFiZWwx",
        "url": format!("https://example.test/labels/{name}"),
        "name": name,
        "color": "cccccc",
        "default": false
    })
}

#[tokio::test]
async fn close_story_issues_patch_with_closed_state() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::PATCH)
            .path("/repos/acme/bpmux/issues/42");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(42, "t", "closed", vec![]).to_string());
    });
    let tracker = mock_tracker(&server);
    let result = tracker.close_story(42).await;
    m.assert();
    // #1151: close_story now returns the post-close Issue JSON so the
    // adapter can parse it into a Story and dispatch StoryRefreshed.
    let value = result.expect("close_story should succeed");
    assert_eq!(value["number"].as_u64(), Some(42));
    assert_eq!(value["gh_state"].as_str(), Some("closed"));
}

#[tokio::test]
async fn close_story_maps_404_to_error() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::PATCH)
            .path("/repos/acme/bpmux/issues/999");
        then.status(404)
            .header("content-type", "application/json")
            .body(r#"{"message":"Not Found","documentation_url":"https://docs"}"#);
    });
    let tracker = mock_tracker(&server);
    let result = tracker.close_story(999).await;
    m.assert();
    assert!(result.is_err(), "404 must surface as Err, got {result:?}");
}

#[tokio::test]
async fn add_label_posts_to_labels_endpoint_with_array_body() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST)
            .path("/repos/acme/bpmux/issues/42/labels")
            .body_includes("\"ready\"");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![label("ready")]).to_string());
    });
    let tracker = mock_tracker(&server);
    let result = tracker.add_label(42, "ready").await;
    m.assert();
    assert_eq!(result, Ok("#42 labeled 'ready'".to_string()));
}

#[tokio::test]
async fn remove_label_deletes_and_treats_404_as_idempotent_noop() {
    // GitHub returns 404 when removing a label not attached; the
    // tracker folds this into success so callers treat "already
    // clear" as a no-op. Contract: 404 on DELETE = Ok.
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::DELETE)
            .path("/repos/acme/bpmux/issues/42/labels/stale");
        then.status(404)
            .header("content-type", "application/json")
            .body(r#"{"message":"Label does not exist","documentation_url":"https://docs"}"#);
    });
    let tracker = mock_tracker(&server);
    let result = tracker.remove_label(42, "stale").await;
    m.assert();
    assert!(result.is_ok(), "404 on remove_label must be idempotent Ok");
}

#[tokio::test]
async fn remove_label_succeeds_on_200() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::DELETE)
            .path("/repos/acme/bpmux/issues/42/labels/ready");
        then.status(200)
            .header("content-type", "application/json")
            .body("[]");
    });
    let tracker = mock_tracker(&server);
    let result = tracker.remove_label(42, "ready").await;
    m.assert();
    assert_eq!(result, Ok("#42 unlabeled 'ready'".to_string()));
}

#[tokio::test]
async fn create_story_posts_to_issues_with_title_and_body() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST)
            .path("/repos/acme/bpmux/issues")
            .body_includes("\"title\":\"new story\"")
            .body_includes("\"body\":\"body-text\"");
        then.status(201)
            .header("content-type", "application/json")
            .body(issue(501, "new story", "open", vec![]).to_string());
    });
    let tracker = mock_tracker(&server);
    let result = tracker.create_story("new story", "body-text").await;
    m.assert();
    // #1151: create_story now returns the freshly-created Issue JSON.
    let value = result.expect("create_story should succeed");
    assert_eq!(value["number"].as_u64(), Some(501));
    assert_eq!(value["title"].as_str(), Some("new story"));
}

#[tokio::test]
async fn list_stories_emits_condensed_json_shape_callers_expect() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                Value::Array(vec![
                    issue(1, "one", "open", vec!["ready"]),
                    issue(2, "two", "closed", vec![]),
                ])
                .to_string(),
            );
    });
    let tracker = mock_tracker(&server);
    let json = tracker.list_stories().await.expect("list_stories ok");
    m.assert();
    let parsed: Value = serde_json::from_str(&json).expect("condensed output must parse");
    let arr = parsed.as_array().expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["number"], 1);
    assert_eq!(arr[0]["state"], "OPEN");
    assert_eq!(arr[0]["labels"][0]["name"], "ready");
    assert_eq!(arr[1]["state"], "CLOSED");
}

#[tokio::test]
async fn comment_on_story_posts_to_comments_endpoint() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST)
            .path("/repos/acme/bpmux/issues/42/comments")
            .body_includes("\"body\":\"a comment\"");
        then.status(201)
            .header("content-type", "application/json")
            .body(comment(1, "a comment").to_string());
    });
    let tracker = mock_tracker(&server);
    let result = tracker.comment_on_story(42, "a comment").await;
    m.assert();
    assert_eq!(result, Ok("#42 commented".to_string()));
}

#[tokio::test]
async fn get_story_labels_returns_names_from_labels_array() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues/42");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(42, "t", "open", vec!["Feature", "ready"]).to_string());
    });
    let tracker = mock_tracker(&server);
    let labels = tracker.get_story_labels(42).await;
    m.assert();
    assert_eq!(labels, vec!["Feature".to_string(), "ready".to_string()]);
}

#[tokio::test]
async fn get_story_labels_returns_empty_on_404() {
    // 404 on get-issue means ambiguous-labels per the fail-soft
    // contract — caller defaults addressee up to Ryan, not a panic.
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/9999");
        then.status(404)
            .header("content-type", "application/json")
            .body(r#"{"message":"Not Found","documentation_url":"https://docs"}"#);
    });
    let tracker = mock_tracker(&server);
    let labels = tracker.get_story_labels(9999).await;
    m.assert();
    assert!(labels.is_empty(), "404 must fail-soft to empty labels");
}

#[tokio::test]
async fn get_story_title_returns_title_on_200() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues/42");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(42, "the title", "open", vec![]).to_string());
    });
    let tracker = mock_tracker(&server);
    let title = tracker.get_story_title(42).await;
    m.assert();
    assert_eq!(title.as_deref(), Some("the title"));
}

#[tokio::test]
async fn fetch_board_stories_concatenates_six_pages_full_repo_scale() {
    // #522: the canonical store had ~285 fewer stories than gh CLI
    // ground-truth post-#511 pagination shipped. To rule out a multi-
    // page concat regression at the chain boundary, this test pins the
    // 6-page case (525 issues across pages of 100 + last partial 25).
    // If the chain holds end-to-end on the mocked side, the gap lives
    // somewhere outside fetch_board_stories — at GH-side, ingest
    // scheduling, or further downstream — and the diagnostic count-log
    // added in this slice will surface the actual point of loss in the
    // running canonical store.
    use super::tracker::StoryTracker;

    let server = MockServer::start();

    let make_page = |start: u64, count: usize| {
        Value::Array(
            (0..count)
                .map(|i| issue(start + i as u64, "fixture", "open", vec!["ready"]))
                .collect::<Vec<_>>(),
        )
        .to_string()
    };

    for page in 1..=5u64 {
        let next_url = format!(
            "{}/repos/acme/bpmux/issues?page={}",
            server.base_url(),
            page + 1
        );
        let path_query = if page == 1 { None } else { Some(page) };
        let body = make_page((page - 1) * 100 + 1, 100);
        server.mock(|when, then| {
            let when = when.method(Method::GET).path("/repos/acme/bpmux/issues");
            let _when = match path_query {
                Some(p) => when.query_param("page", p.to_string()),
                None => when.query_param_missing("page"),
            };
            then.status(200)
                .header("content-type", "application/json")
                .header("Link", format!("<{next_url}>; rel=\"next\""))
                .body(body);
        });
    }

    let last_body = make_page(501, 25);
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("page", "6");
        then.status(200)
            .header("content-type", "application/json")
            .body(last_body);
    });

    let tracker = mock_tracker(&server);
    let stories = tracker
        .fetch_board_stories()
        .await
        .expect("fetch_board_stories ok");

    assert_eq!(
        stories.len(),
        525,
        "all 6 pages must concatenate; first-page-only would return 100"
    );
    let numbers: std::collections::HashSet<u64> = stories
        .iter()
        .filter_map(|s| s.get("number").and_then(|n| n.as_u64()))
        .collect();
    assert_eq!(numbers.len(), 525, "all numbers should be unique");
    assert!(numbers.contains(&1), "first issue must be present");
    assert!(numbers.contains(&525), "last issue must be present");
}

#[tokio::test]
async fn fetch_board_stories_follows_next_link_until_exhaustion() {
    // #511: GitHub returns issues in pages of up to 100 ordered by
    // updated_at desc. Without next-link follow-through, the renderer
    // sees only the first page; after a heavy iteration crowds the
    // first page with recently-touched-but-old stories, today's just-
    // filed items disappear from the board until something else
    // updates them. This test pins the multi-page contract: serve two
    // pages, assert the implementation collects items from both.
    use super::tracker::StoryTracker;

    let server = MockServer::start();
    let next_url = format!("{}/repos/acme/bpmux/issues?page=2", server.base_url());

    let p1 = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param_missing("page");
        then.status(200)
            .header("content-type", "application/json")
            .header("Link", format!("<{next_url}>; rel=\"next\""))
            .body(
                Value::Array(vec![
                    issue(101, "page-one-a", "open", vec!["ready"]),
                    issue(102, "page-one-b", "open", vec!["in progress"]),
                ])
                .to_string(),
            );
    });
    let p2 = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("page", "2");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![issue(103, "page-two", "open", vec!["ready"])]).to_string());
    });

    let tracker = mock_tracker(&server);
    let stories = tracker
        .fetch_board_stories()
        .await
        .expect("fetch_board_stories ok");
    p1.assert();
    p2.assert();

    let numbers: Vec<u64> = stories
        .iter()
        .filter_map(|s| s.get("number").and_then(|n| n.as_u64()))
        .collect();
    assert_eq!(
        numbers,
        vec![101, 102, 103],
        "must concat across pages in order; first-page-only would drop #103"
    );
}
