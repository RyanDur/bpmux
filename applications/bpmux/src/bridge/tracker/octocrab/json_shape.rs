//! GH-issue JSON-shape helpers used by sister `octocrab` sub-modules
//! to serialise an `octocrab::models::issues::Issue` into the
//! board-store-friendly envelope that
//! `board::adapters::github::decode_story_metadata` consumes. The
//! decoder reads `gh_state` + `labels` and derives canonical
//! `Story.state` via the single classifier `state_from_labels` per
//! Ryan's 2026-04-27 boundary-principle ruling. These helpers are
//! the boundary OUT — they serialise raw GH facts; no state
//! classification happens here.

pub(super) fn gh_state_label(state: &octocrab::models::IssueState) -> &'static str {
    match state {
        octocrab::models::IssueState::Open => "open",
        octocrab::models::IssueState::Closed => "closed",
        _ => "unknown",
    }
}

pub(super) fn issue_to_board_json(issue: &octocrab::models::issues::Issue) -> serde_json::Value {
    let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
    let is_closed = matches!(issue.state, octocrab::models::IssueState::Closed);
    let state = crate::board::adapters::github::state_from_labels(&labels, is_closed).as_str();
    let gh_state = gh_state_label(&issue.state);
    let parent_epic = crate::board::adapters::github::parent_epic_from_labels(&labels);
    serde_json::json!({
        "number": issue.number,
        "title": issue.title,
        "state": state,
        "gh_state": gh_state,
        "labels": labels,
        "parent_epic": parent_epic,
        "assignees": issue.assignees.iter().map(|a| a.login.clone()).collect::<Vec<_>>(),
        "created_at": issue.created_at.to_rfc3339(),
        "body": issue.body.clone().unwrap_or_default(),
    })
}
