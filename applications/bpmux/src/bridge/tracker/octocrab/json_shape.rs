pub(super) fn gh_state_label(state: &octocrab::models::IssueState) -> &'static str {
    match state {
        octocrab::models::IssueState::Open => "open",
        octocrab::models::IssueState::Closed => "closed",
        _ => "unknown",
    }
}

pub(super) fn issue_to_board_json(issue: &octocrab::models::issues::Issue) -> serde_json::Value {
    use crate::product_management::scheduling::ProjectedState;
    use crate::product_management::story::Work;
    let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
    let is_closed = matches!(issue.state, octocrab::models::IssueState::Closed);
    let state = match crate::integrations::github::mapper::state_from_labels(&labels, is_closed) {
        Work::NotStarted => ProjectedState::Icebox.as_str(),
        other => other.as_str(),
    };
    let gh_state = gh_state_label(&issue.state);
    let parent_epic = crate::integrations::github::mapper::parent_epic_from_labels(&labels);
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
