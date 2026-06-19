use serde_json::{Value, json};

pub(crate) fn author() -> Value {
    json!({
        "login": "owner",
        "id": 1,
        "node_id": "MDQ6VXNlcjE=",
        "avatar_url": "https://example.test/avatar.png",
        "gravatar_id": "",
        "url": "https://example.test/users/owner",
        "html_url": "https://example.test/owner",
        "followers_url": "https://example.test/owner/followers",
        "following_url": "https://example.test/owner/following{/other_user}",
        "gists_url": "https://example.test/owner/gists{/gist_id}",
        "starred_url": "https://example.test/owner/starred{/owner}{/repo}",
        "subscriptions_url": "https://example.test/owner/subscriptions",
        "organizations_url": "https://example.test/owner/orgs",
        "repos_url": "https://example.test/owner/repos",
        "events_url": "https://example.test/owner/events{/privacy}",
        "received_events_url": "https://example.test/owner/received_events",
        "type": "User",
        "site_admin": false
    })
}

pub(crate) fn issue(number: u64, title: &str, state: &str, labels: Vec<&str>) -> Value {
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
