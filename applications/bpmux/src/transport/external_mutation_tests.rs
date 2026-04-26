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
use serde_json::Value;

use crate::board::adapters::github::GitHubSourceAdapter;
use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::bridge::test_fixtures::issue;
use crate::bridge::tracker::OctocrabStoryTracker;

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
