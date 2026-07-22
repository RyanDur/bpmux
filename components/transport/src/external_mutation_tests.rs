#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use httpmock::{Method, MockServer};
use serde_json::{Value, json};

use bridge::test_fixtures::issue;
use integrations::github::GitHubSourceAdapter;
use integrations::github::tracker::OctocrabStoryTracker;
use sync::SyncOrchestrator;

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

async fn fixture_orchestrator(server: &MockServer) -> (Arc<SyncOrchestrator>, store::Select) {
    let tracker = OctocrabStoryTracker::with_base_url(
        &format!("{}/", server.base_url()),
        "acme".to_string(),
        "bpmux".to_string(),
    )
    .expect("build tracker against mock");
    let adapter = Arc::new(GitHubSourceAdapter::new(tracker));
    let (orch, select) = store_test_support::build_seeded_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        vec![],
        vec![],
        vec![],
    );
    orch.refresh_all().await.expect("seed-empty ingest");
    (orch, select)
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

    let (refresh, select) = fixture_orchestrator(&server).await;
    assert!(
        select
            .select(|state| product_management::story::slice::get(
                &state.stories,
                product_management::story::StoryId::new(101)
            ))
            .is_none(),
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

    let story = select
        .select(|state| {
            product_management::story::slice::get(
                &state.stories,
                product_management::story::StoryId::new(101),
            )
        })
        .expect("story in store");
    assert_eq!(story.id, product_management::story::StoryId::new(101));
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

    let (refresh, select) = fixture_orchestrator(&server).await;

    let mut initial_labels_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/202");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(202, "[Feature] external-label-change", "open", vec![]).to_string());
    });

    refresh.refresh_story(202).await.expect("first refresh");
    let initial = select
        .select(|state| {
            product_management::story::slice::get(
                &state.stories,
                product_management::story::StoryId::new(202),
            )
        })
        .expect("seeded story");
    assert_eq!(
        initial.state,
        product_management::story::Work::NotStarted,
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
    let updated = select
        .select(|state| {
            product_management::story::slice::get(
                &state.stories,
                product_management::story::StoryId::new(202),
            )
        })
        .expect("story still present");
    assert_eq!(
        updated.state,
        product_management::story::Work::InProgress,
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

    let (refresh, _select) = fixture_orchestrator(&server).await;

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
async fn external_re_open_returns_the_story_to_the_label_derived_state_after_refresh() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let (refresh, select) = fixture_orchestrator(&server).await;

    let mut closed_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/505");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(505, "[Feature] external-re-open", "closed", vec![]).to_string());
    });

    refresh.refresh_story(505).await.expect("first refresh");
    let initial = select
        .select(|state| {
            product_management::story::slice::get(
                &state.stories,
                product_management::story::StoryId::new(505),
            )
        })
        .expect("seeded story");
    assert_eq!(
        initial.state,
        product_management::story::Work::Accepted,
        "closed with no labels forces the terminal Accepted state at decode"
    );

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
    let updated = select
        .select(|state| {
            product_management::story::slice::get(
                &state.stories,
                product_management::story::StoryId::new(505),
            )
        })
        .expect("story still present");
    assert_eq!(
        updated.state,
        product_management::story::Work::NotStarted,
        "post-refresh, an externally-reopened issue with no labels returns to icebox"
    );
}

#[tokio::test]
async fn external_label_remove_clears_state_back_to_icebox_after_refresh() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let (refresh, select) = fixture_orchestrator(&server).await;

    let mut labeled_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/606");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                issue(
                    606,
                    "[Feature] external-label-remove",
                    "open",
                    vec!["in progress"],
                )
                .to_string(),
            );
    });

    refresh.refresh_story(606).await.expect("first refresh");
    let initial = select
        .select(|state| {
            product_management::story::slice::get(
                &state.stories,
                product_management::story::StoryId::new(606),
            )
        })
        .expect("seeded story");
    assert_eq!(
        initial.state,
        product_management::story::Work::InProgress,
        "fixture seeds in-progress state"
    );

    labeled_mock.delete();
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/606");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(606, "[Feature] external-label-remove", "open", vec![]).to_string());
    });

    refresh
        .refresh_story(606)
        .await
        .expect("second refresh after external label remove");
    let updated = select
        .select(|state| {
            product_management::story::slice::get(
                &state.stories,
                product_management::story::StoryId::new(606),
            )
        })
        .expect("story still present");
    assert_eq!(
        updated.state,
        product_management::story::Work::NotStarted,
        "post-refresh, bpmux state clears back to Icebox after externally-removed label"
    );
}

#[tokio::test]
async fn external_body_change_propagates_via_fetch_body_after_refresh() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let (refresh, _select) = fixture_orchestrator(&server).await;

    let mut original_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/707");
        then.status(200)
            .header("content-type", "application/json")
            .body({
                let mut v = issue(707, "[Feature] external-body-change", "open", vec![]);
                v["body"] = Value::String("ORIGINAL_BODY_MARKER".to_string());
                v.to_string()
            });
    });

    let initial = refresh.fetch_body(707).await.expect("first fetch_body");
    assert!(
        initial.contains("ORIGINAL_BODY_MARKER"),
        "first fetch returns original body verbatim; got {initial:?}"
    );

    original_mock.delete();
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/707");
        then.status(200)
            .header("content-type", "application/json")
            .body({
                let mut v = issue(707, "[Feature] external-body-change", "open", vec![]);
                v["body"] = Value::String("EDITED_BODY_MARKER".to_string());
                v.to_string()
            });
    });

    let updated = refresh.fetch_body(707).await.expect("second fetch_body");
    assert!(
        updated.contains("EDITED_BODY_MARKER"),
        "post-edit fetch returns the externally-edited body; got {updated:?}"
    );
    assert!(
        !updated.contains("ORIGINAL_BODY_MARKER"),
        "edited body fully replaces the original; got {updated:?}"
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

    let (refresh, select) = fixture_orchestrator(&server).await;

    let mut open_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/303");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue(303, "[Feature] external-close", "open", vec![]).to_string());
    });

    refresh.refresh_story(303).await.expect("first refresh");
    let initial = select
        .select(|state| {
            product_management::story::slice::get(
                &state.stories,
                product_management::story::StoryId::new(303),
            )
        })
        .expect("seeded story");
    assert!(
        !initial.is_concluded(),
        "fresh story is not concluded before external close"
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
    let updated = select
        .select(|state| {
            product_management::story::slice::get(
                &state.stories,
                product_management::story::StoryId::new(303),
            )
        })
        .expect("story still present");
    assert_eq!(
        updated.state,
        product_management::story::Work::Accepted,
        "post-refresh, an externally-closed issue forces Accepted state per state_from_labels"
    );
}
