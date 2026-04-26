//! Cross-bridge mutation-propagation E2E tests (#476 Phase 1 +
//! #482 substrate). Two bridges spawned via the new
//! `TestBridge::connect_with_mock_gh` harness share a single
//! `httpmock::MockServer` as their GitHub source-of-truth; this
//! exercises the F-coh-1..7 falsifier class — bridge A's view of
//! the world should converge on bridge B's via the shared GH state.
//!
//! Per the 3-class framing — cross-bridge mutation-propagation
//! applicability = mostly external-mutation-class-with-multi-bridge
//! observers. Per-bridge background ingest poll (1s in test mode
//! per `connect_with_mock_gh`) drives convergence.
//!
//! First spec: external state mutated mid-session → both bridges
//! observe via their next poll. Subsequent specs (deferred) extend
//! to bridge-A-mutation → bridge-B-observation roundtrips.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use httpmock::{Method, MockServer};
use serde_json::Value;
use std::time::Duration;

#[test]
fn external_state_change_propagates_to_two_bridges_via_shared_mock_gh() {
    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());

    let mut empty_list_mock = server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let bridge_a = TestBridge::connect_with_mock_gh(&bpmux, "pm", &base_url).await;
        let bridge_b = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let initial_a = bridge_a
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board on bridge A succeeds");
        let initial_a_text = initial_a
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| initial_a.to_string());
        assert!(
            !initial_a_text.contains("#999"),
            "bridge A starts with empty board (no #999); got:\n{initial_a_text}"
        );

        empty_list_mock.delete();
        let issue_999 = serde_json::json!({
            "id": 999,
            "node_id": "MDU6SXNzdWUx",
            "url": "https://example.test/issues/999",
            "repository_url": "https://example.test/repo",
            "labels_url": "https://example.test/issues/999/labels{/name}",
            "comments_url": "https://example.test/issues/999/comments",
            "events_url": "https://example.test/issues/999/events",
            "html_url": "https://example.test/web/999",
            "number": 999,
            "state": "open",
            "state_reason": null,
            "title": "[Feature] cross-bridge-shared-state-marker",
            "body": null,
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
        });
        server.mock(|when, then| {
            when.method(Method::GET).path("/repos/acme/bpmux/issues");
            then.status(200)
                .header("content-type", "application/json")
                .body(Value::Array(vec![issue_999.clone()]).to_string());
        });

        tokio::time::sleep(Duration::from_millis(2500)).await;

        let after_a = bridge_a
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board on bridge A succeeds post-poll");
        let after_a_text = after_a
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| after_a.to_string());
        assert!(
            after_a_text.contains("#999")
                && after_a_text.contains("cross-bridge-shared-state-marker"),
            "bridge A poll picked up the externally-added story; got:\n{after_a_text}"
        );

        let after_b = bridge_b
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board on bridge B succeeds post-poll");
        let after_b_text = after_b
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| after_b.to_string());
        assert!(
            after_b_text.contains("#999")
                && after_b_text.contains("cross-bridge-shared-state-marker"),
            "bridge B poll picked up the same externally-added story (cross-bridge convergence); got:\n{after_b_text}"
        );

        bridge_a.close().await;
        bridge_b.close().await;
    });
}
