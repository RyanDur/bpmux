//! External-mutation E2E tests (#476 Phase 1 backfill — external-
//! mutation user-class). Use the MockOctocrab adapter mode (commits
//! 6/7/8 substrate) to exercise bpmux's adapter observing state
//! changes that originate OUTSIDE bpmux (gh CLI, GitHub UI, automation).
//!
//! Per the 3-class framing — external-mutation applicability =
//! {agent: no, human: no, external: yes}. Tests verify that bpmux's
//! refresh path picks up source-of-truth changes that the bridge
//! itself didn't author.
//!
//! Three canonical user-jobs covered here:
//!   1. external-create — issue created via gh CLI / GitHub UI;
//!      bpmux refresh observes the new story
//!   2. external-label — label added externally (e.g., `in progress`);
//!      bpmux refresh reflects the new state per state_from_labels
//!   3. external-close — issue closed externally; bpmux refresh
//!      reflects the closed state via is_closed → "accepted"
//!
//! Same substrate as `test_routes_mock_octocrab_tests`: real
//! GitHubSourceAdapter pointed at httpmock; full decode_story_metadata
//! path exercised. Bridge-subprocess-against-MockServer is deferred
//! (would need TestBpmux harness extension to support custom
//! GH base URL via env-var to OctocrabStoryTracker).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use httpmock::{Method, MockServer};
use serde_json::{Value, json};

use crate::board::adapters::github::GitHubSourceAdapter;
use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::bridge::test_fixtures::issue;
use crate::bridge::tracker::OctocrabStoryTracker;

fn comment(body: &str) -> Value {
    json!({
        "id": 1,
        "node_id": "MDEyOklzc3VlQ29tbWVudDE=",
        "url": "https://example.test/comments/1",
        "html_url": "https://example.test/web/comment/1",
        "body": body,
        "author_association": "OWNER",
        "user": {
            "login": "external-actor",
            "id": 1,
            "node_id": "MDQ6VXNlcjE=",
            "avatar_url": "https://example.test/avatar.png",
            "gravatar_id": "",
            "url": "https://example.test/users/external-actor",
            "html_url": "https://example.test/external-actor",
            "followers_url": "https://example.test/external-actor/followers",
            "following_url": "https://example.test/external-actor/following{/other_user}",
            "gists_url": "https://example.test/external-actor/gists{/gist_id}",
            "starred_url": "https://example.test/external-actor/starred{/owner}{/repo}",
            "subscriptions_url": "https://example.test/external-actor/subscriptions",
            "organizations_url": "https://example.test/external-actor/orgs",
            "repos_url": "https://example.test/external-actor/repos",
            "events_url": "https://example.test/external-actor/events{/privacy}",
            "received_events_url": "https://example.test/external-actor/received_events",
            "type": "User",
            "site_admin": false
        },
        "created_at": "2026-04-26T19:30:00Z"
    })
}

async fn fixture_orchestrator(server: &MockServer) -> Arc<dyn RefreshDriver> {
    let tracker = OctocrabStoryTracker::with_base_url(
        &format!("{}/", server.base_url()),
        "acme".to_string(),
        "bpmux".to_string(),
    )
    .expect("build tracker against mock");
    let adapter = Arc::new(GitHubSourceAdapter::new(tracker));
    let store = Arc::new(BoardStore::new());
    let orch = SyncOrchestrator::new(store, adapter);
    orch.ingest_once().await.expect("seed-empty ingest");
    Arc::new(orch)
}

#[tokio::test]
async fn external_create_via_gh_cli_appears_in_bpmux_after_refresh() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let refresh = fixture_orchestrator(&server).await;
    assert!(
        refresh.story(101).is_none(),
        "fresh adapter has no #101 before external creation"
    );

    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/101");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(101, "[Feature] external-via-gh-cli", "open", vec![]).to_string());
    });

    let result = refresh.refresh_story(101).await.expect("refresh_story");
    assert!(
        result.is_some(),
        "post-refresh, bpmux observes the externally-created story"
    );

    let story = refresh.story(101).expect("story in store");
    assert_eq!(story.number, 101);
    assert_eq!(story.title, "[Feature] external-via-gh-cli");
}

#[tokio::test]
async fn external_label_change_propagates_to_bpmux_state_after_refresh() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let refresh = fixture_orchestrator(&server).await;

    let mut initial_labels_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/202");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(202, "[Feature] external-label-change", "open", vec![]).to_string());
    });

    refresh.refresh_story(202).await.expect("first refresh");
    let initial = refresh.story(202).expect("seeded story");
    assert_eq!(
        initial.state,
        crate::board::domain::StoryState::Icebox,
        "no state-bearing label → Icebox"
    );

    initial_labels_mock.delete();
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/202");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                issue(
                    202,
                    "[Feature] external-label-change",
                    "open",
                    vec!["in progress"],
                )
                .to_string(),
            );
    });

    refresh
        .refresh_story(202)
        .await
        .expect("second refresh after external label change");
    let updated = refresh.story(202).expect("story still present");
    assert_eq!(
        updated.state,
        crate::board::domain::StoryState::InProgress,
        "post-refresh, bpmux state reflects externally-added 'in progress' label"
    );
}

#[tokio::test]
async fn external_comment_appears_via_fetch_free_comments_after_refresh() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let refresh = fixture_orchestrator(&server).await;

    let mut empty_comments_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/404/comments");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let initial = refresh
        .fetch_free_comments(404)
        .await
        .expect("fetch_free_comments empty");
    assert!(initial.is_empty(), "no external comments yet");

    empty_comments_mock.delete();
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/404/comments");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                Value::Array(vec![comment(
                    "external commenter dropped a note via gh CLI",
                )])
                .to_string(),
            );
    });

    let updated = refresh
        .fetch_free_comments(404)
        .await
        .expect("fetch_free_comments after external add");
    assert_eq!(
        updated.len(),
        1,
        "post-refresh, bpmux observes the externally-added comment"
    );
    assert!(
        updated[0].body.contains("external commenter"),
        "comment body round-trips verbatim; got {:?}",
        updated[0].body
    );
}

#[tokio::test]
async fn external_re_open_clears_is_closed_after_refresh() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let refresh = fixture_orchestrator(&server).await;

    let mut closed_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/505");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(505, "[Feature] external-re-open", "closed", vec![]).to_string());
    });

    refresh.refresh_story(505).await.expect("first refresh");
    let initial = refresh.story(505).expect("seeded story");
    assert!(initial.is_closed, "starts closed");

    closed_mock.delete();
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/505");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(505, "[Feature] external-re-open", "open", vec![]).to_string());
    });

    refresh
        .refresh_story(505)
        .await
        .expect("second refresh after external re-open");
    let updated = refresh.story(505).expect("story still present");
    assert!(
        !updated.is_closed,
        "post-refresh, bpmux observes externally-reopened issue"
    );
}

#[tokio::test]
async fn external_close_propagates_to_bpmux_accepted_state_after_refresh() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let refresh = fixture_orchestrator(&server).await;

    let mut open_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/303");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(303, "[Feature] external-close", "open", vec![]).to_string());
    });

    refresh.refresh_story(303).await.expect("first refresh");
    let initial = refresh.story(303).expect("seeded story");
    assert!(
        !initial.is_closed,
        "fresh story is open before external close"
    );

    open_mock.delete();
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/303");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(303, "[Feature] external-close", "closed", vec![]).to_string());
    });

    refresh
        .refresh_story(303)
        .await
        .expect("second refresh after external close");
    let updated = refresh.story(303).expect("story still present");
    assert!(
        updated.is_closed,
        "post-refresh, bpmux observes externally-closed issue (renderer maps is_closed → accepted)"
    );
    assert_eq!(
        updated.state,
        crate::board::domain::StoryState::Accepted,
        "is_closed forces Accepted state per state_from_labels"
    );
}
