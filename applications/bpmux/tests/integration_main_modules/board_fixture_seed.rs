//! #724: read-side fixture-seed test using the httpmock pattern
//! (only octocrab is mocked; real GitHubSourceAdapter + canonical
//! BoardStore + RPC + state-decode all exercised). Supersedes the
//! pre-#724 BPMUX_BOARD_FIXTURE+InMemorySourceAdapter substrate-bypass
//! shape per Ryan's 2026-04-30 directive: only octocrab gets mocked.
//!
//! Verifies that when the mocked GH list response includes a story
//! with body content, the canonical store ingests both metadata + body
//! through the real adapter cascade so get_story surfaces the body
//! without a separate fetch.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::harness;

use std::time::Duration;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use httpmock::{Method, MockServer};
use serde_json::Value;

fn issue_json_with_body(number: u64, title: &str, body: &str) -> Value {
    serde_json::json!({
        "id": number,
        "node_id": "MDU6SXNzdWUx",
        "url": format!("https://example.test/issues/{number}"),
        "repository_url": "https://example.test/repo",
        "labels_url": format!("https://example.test/issues/{number}/labels{{/name}}"),
        "comments_url": format!("https://example.test/issues/{number}/comments"),
        "events_url": format!("https://example.test/issues/{number}/events"),
        "html_url": format!("https://example.test/web/{number}"),
        "number": number,
        "state": "open",
        "state_reason": null,
        "title": title,
        "body": body,
        "user": {
            "login": "actor",
            "id": 1,
            "node_id": "MDQ6VXNlcjE=",
            "avatar_url": "https://example.test/avatar.png",
            "gravatar_id": "",
            "url": "https://example.test/users/actor",
            "html_url": "https://example.test/actor",
            "followers_url": "https://example.test/actor/followers",
            "following_url": "https://example.test/actor/following{/other_user}",
            "gists_url": "https://example.test/actor/gists{/gist_id}",
            "starred_url": "https://example.test/actor/starred{/owner}{/repo}",
            "subscriptions_url": "https://example.test/actor/subscriptions",
            "organizations_url": "https://example.test/actor/orgs",
            "repos_url": "https://example.test/actor/repos",
            "events_url": "https://example.test/actor/events{/privacy}",
            "received_events_url": "https://example.test/actor/received_events",
            "type": "User",
            "site_admin": false
        },
        "labels": [],
        "assignees": [],
        "author_association": "OWNER",
        "comments": 0,
        "created_at": "2026-04-26T19:30:00Z",
        "updated_at": "2026-04-26T19:30:00Z",
        "closed_at": null,
        "milestone": null,
        "pull_request": null,
        "active_lock_reason": null,
        "locked": false
    })
}

#[test]
fn bpmux_surfaces_fixture_story_body_through_real_adapter_via_mocked_octocrab() {
    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());

    let body_with_atp = format!(
        "## Acceptance test path\n\n{}/tests/harness/story_helpers.rs\n",
        env!("CARGO_MANIFEST_DIR")
    );
    let issue = issue_json_with_body(777, "Fixture-seed test", &body_with_atp);

    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![issue.clone()]).to_string());
    });
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/777");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue.to_string());
    });

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;
        tokio::time::sleep(Duration::from_millis(2500)).await;

        let story = dev
            .call_tool("get_story", serde_json::json!({"number": 777}))
            .await
            .expect("get_story should succeed against mocked-GH-seeded story");
        let story_text = story
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| story.to_string());
        assert!(
            story_text.contains("Fixture-seed test"),
            "get_story surfaces fixture title via canonical-store; got:\n{story_text}"
        );
        assert!(
            story_text.contains("body_hydrated"),
            "post-#1165 slice 1: response carries body_hydrated flag for caller; \
             body content surfaces once background poller hydrates (slice 2+); \
             got:\n{story_text}"
        );
    });
}
