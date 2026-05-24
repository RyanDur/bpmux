//! #724: shared httpmock helpers for fixture-seed tests.
//!
//! Tests should use these to construct GitHub API JSON payloads that
//! octocrab's deserializer accepts. Per Ryan's 2026-04-30 directive,
//! ONLY octocrab gets mocked — these helpers wrap the GitHub HTTP
//! boundary so the real `GitHubSourceAdapter` + canonical store +
//! state-decode + RPC dispatch are exercised end-to-end.
//!
//! ## The Label gotcha
//!
//! `octocrab`'s `Label` deserializer needs the FULL shape (id,
//! node_id, url, name, description, color, default). A skinny
//! `[{"name": "ready"}]` causes serde to silently reject the entire
//! issue → BoardStore stays empty with no error surface. `label_json`
//! emits the full shape from a single `name` argument.
#![allow(dead_code)]

use httpmock::{HttpMockRequest, HttpMockResponse, Method, MockServer};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Build a full GitHub label JSON object suitable for inclusion in an
/// issue's `labels` array. Stable id/url so identical names produce
/// identical JSON across calls (handy for comparison assertions).
pub fn label_json(name: &str) -> Value {
    let id = name.bytes().fold(0u64, |acc, b| {
        acc.wrapping_mul(131).wrapping_add(u64::from(b))
    });
    serde_json::json!({
        "id": id,
        "node_id": format!("MDU6TGFiZWw{id}"),
        "url": format!("https://example.test/labels/{name}"),
        "name": name,
        "description": null,
        "color": "ededed",
        "default": false
    })
}

/// Register the standard mock-gh endpoints for a single-story
/// mutation test: GET issue list + GET single issue + POST labels + POST
/// comments + DELETE labels (regex match for any label name). Each
/// returns the minimum-viable shape octocrab's deserializer accepts.
///
/// Caller is responsible for the OUTER GET /repos/.../issues mock —
/// only `number` is shared per call, but the issue list often varies
/// per test (different labels/seeds). This helper registers per-issue
/// endpoints (issue detail + mutations) only.
pub fn mock_story_mutations(server: &MockServer, number: u64, issue_response: &Value) {
    let state = Arc::new(Mutex::new(issue_state_from_json(issue_response)));
    mock_story_mutations_with_state(server, number, state);
}

pub fn mock_story_full(
    server: &MockServer,
    number: u64,
    issue_response: &Value,
) -> Arc<Mutex<MockIssueState>> {
    let state = Arc::new(Mutex::new(issue_state_from_json(issue_response)));
    let list_state = Arc::clone(&state);
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.respond_with(move |_req: &HttpMockRequest| {
            let snap = list_state.lock().unwrap().clone();
            json_response(Value::Array(vec![snap.to_issue_json(number)]))
        });
    });
    mock_story_mutations_with_state(server, number, Arc::clone(&state));
    state
}

fn issue_state_from_json(issue_response: &Value) -> MockIssueState {
    MockIssueState {
        title: issue_response
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("untitled")
            .to_string(),
        body: issue_response
            .get("body")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        labels: labels_from_issue(issue_response),
        is_closed: issue_response
            .get("state")
            .and_then(|v| v.as_str())
            .map(|s| s == "closed")
            .unwrap_or(false),
    }
}

pub fn mock_story_mutations_with_state(
    server: &MockServer,
    number: u64,
    state: Arc<Mutex<MockIssueState>>,
) {
    let issue_path = format!("/repos/acme/bpmux/issues/{number}");
    let labels_path = format!("/repos/acme/bpmux/issues/{number}/labels");
    let comments_path = format!("/repos/acme/bpmux/issues/{number}/comments");
    let labels_delete_re = format!(r"^/repos/acme/bpmux/issues/{number}/labels/.+$");

    let get_state = Arc::clone(&state);
    let issue_path_get = issue_path.clone();
    server.mock(|when, then| {
        when.method(Method::GET).path(issue_path_get);
        then.respond_with(move |_req: &HttpMockRequest| {
            let snap = get_state.lock().unwrap().clone();
            json_response(snap.to_issue_json(number))
        });
    });
    let patch_state = Arc::clone(&state);
    server.mock(|when, then| {
        when.method(Method::PATCH).path(issue_path);
        then.respond_with(move |req: &HttpMockRequest| {
            let mut s = patch_state.lock().unwrap();
            if let Some(patch) = parse_json_body(req) {
                if let Some(state_str) = patch.get("state").and_then(|v| v.as_str()) {
                    s.is_closed = state_str == "closed";
                }
                if let Some(labels_arr) = patch.get("labels").and_then(|v| v.as_array()) {
                    let new_labels: Vec<String> = labels_arr
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    s.labels = new_labels;
                }
            }
            let snap = s.clone();
            drop(s);
            json_response(snap.to_issue_json(number))
        });
    });
    let post_labels_state = Arc::clone(&state);
    server.mock(|when, then| {
        when.method(Method::POST).path(labels_path);
        then.respond_with(move |req: &HttpMockRequest| {
            let mut s = post_labels_state.lock().unwrap();
            if let Some(patch) = parse_json_body(req)
                && let Some(labels_arr) = patch.get("labels").and_then(|v| v.as_array())
            {
                for v in labels_arr {
                    if let Some(name) = v.as_str()
                        && !s.labels.iter().any(|l| l == name)
                    {
                        s.labels.push(name.to_string());
                    }
                }
            }
            let labels_json: Vec<Value> = s.labels.iter().map(|n| label_json(n)).collect();
            json_response(Value::Array(labels_json))
        });
    });
    server.mock(|when, then| {
        when.method(Method::POST).path(comments_path);
        then.status(201)
            .header("content-type", "application/json")
            .body(comment_json("ok").to_string());
    });
    let delete_labels_state = Arc::clone(&state);
    server.mock(|when, then| {
        when.method(Method::DELETE)
            .path_matches(regex::Regex::new(&labels_delete_re).unwrap());
        then.respond_with(move |req: &HttpMockRequest| {
            let mut s = delete_labels_state.lock().unwrap();
            let removed_label = req.uri().path().rsplit('/').next().map(String::from);
            if let Some(name) = removed_label {
                s.labels.retain(|l| l != &name);
            }
            let labels_json: Vec<Value> = s.labels.iter().map(|n| label_json(n)).collect();
            json_response(Value::Array(labels_json))
        });
    });
}

#[derive(Clone)]
pub struct MockIssueState {
    pub title: String,
    pub body: Option<String>,
    pub labels: Vec<String>,
    pub is_closed: bool,
}

impl MockIssueState {
    pub fn to_issue_json(&self, number: u64) -> Value {
        let labels: Vec<&str> = self.labels.iter().map(String::as_str).collect();
        let mut issue = issue_json(number, &self.title, &labels, self.body.as_deref());
        if let Some(obj) = issue.as_object_mut() {
            obj.insert(
                "state".to_string(),
                Value::String(if self.is_closed { "closed" } else { "open" }.to_string()),
            );
        }
        issue
    }
}

fn labels_from_issue(issue: &Value) -> Vec<String> {
    issue
        .get("labels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|l| l.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_json_body(req: &HttpMockRequest) -> Option<Value> {
    let body = req.body();
    serde_json::from_slice::<Value>(body.as_ref()).ok()
}

fn json_response(value: Value) -> HttpMockResponse {
    HttpMockResponse::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(value.to_string())
        .build()
}

/// Build a GitHub comment JSON shape suitable as the response body for
/// POST /repos/{owner}/{repo}/issues/{N}/comments. The skinny `{"id":1}`
/// shape causes octocrab's Comment deserializer to error silently —
/// surface as drain-push failure that's hard to debug. This helper
/// emits the minimum fields octocrab's Comment expects.
pub fn comment_json(body: &str) -> Value {
    serde_json::json!({
        "id": 1,
        "node_id": "MDEyOklzc3VlQ29tbWVudDE=",
        "url": "https://example.test/comments/1",
        "html_url": "https://example.test/web/comment/1",
        "body": body,
        "author_association": "OWNER",
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
        "created_at": "2026-04-30T00:00:00Z"
    })
}

/// Build a GitHub issue JSON object with the named labels + body.
/// Pass `None` for body to emit a `null` body field.
pub fn issue_json(number: u64, title: &str, labels: &[&str], body: Option<&str>) -> Value {
    let labels_json: Vec<Value> = labels.iter().map(|n| label_json(n)).collect();
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
        "labels": labels_json,
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
