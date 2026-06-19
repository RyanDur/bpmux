use axum::http::HeaderMap;
use serde_json::Value;

pub const AGENT_HEADER: &str = "X-BPMUX-Agent";

const KNOWN_AGENTS: &[&str] = &["dev", "pm", "designer", "owner"];

pub(super) fn known_tools() -> &'static [&'static str] {
    &[
        "start_story",
        "deliver_story",
        "pass_desk_check",
        "fail_desk_check",
        "accept_story",
        "reject_story",
        "close_story",
        "create_story",
        "create_iteration",
        "create_outcome",
        "close_iteration",
        "comment_on_story",
        "label_story",
        "set_parent_epic",
        "unset_parent_epic",
        "add_story_to_iteration",
        "designate_current_iteration",
        "close_pre_pickup_review",
        "dev_ok_iteration",
        "map_iteration_to_outcome",
        "reorder_story",
        "edit_story",
        "view_board",
        "get_story",
        "list_ready",
        "list_epic_children",
        "dev_in_flight",
        "view_story_timeline",
        "view_comments",
        "view_backlog",
        "submit_verdict",
        "view_verdicts",
        "view_velocity",
        "view_story_commits",
        "view_commit_diff",
    ]
}

pub(super) fn is_read_only_tool(tool: &str) -> bool {
    matches!(
        tool,
        "view_board"
            | "get_story"
            | "list_ready"
            | "list_epic_children"
            | "dev_in_flight"
            | "view_story_timeline"
            | "view_comments"
            | "view_backlog"
            | "view_verdicts"
            | "view_velocity"
            | "view_story_commits"
            | "view_commit_diff"
    )
}

pub fn agent_from_headers(headers: &HeaderMap) -> Result<String, String> {
    let raw = headers
        .get(AGENT_HEADER)
        .ok_or_else(|| format!("{AGENT_HEADER} header required"))?
        .to_str()
        .map_err(|_| format!("{AGENT_HEADER} header is not valid UTF-8"))?
        .trim()
        .to_ascii_lowercase();
    if raw.is_empty() {
        return Err(format!("{AGENT_HEADER} header is empty"));
    }
    if !KNOWN_AGENTS.contains(&raw.as_str()) {
        return Err(format!(
            "{AGENT_HEADER} '{raw}' is not a known role; expected one of {KNOWN_AGENTS:?}"
        ));
    }
    Ok(raw)
}

pub(super) fn args_from_value(body: Value) -> Result<serde_json::Map<String, Value>, String> {
    match body {
        Value::Object(map) => Ok(map),
        other => Err(format!(
            "RPC body must be a JSON object; got {}",
            match other {
                Value::Null => "null",
                Value::Bool(_) => "bool",
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Array(_) => "array",
                Value::Object(_) => unreachable!(),
            }
        )),
    }
}
