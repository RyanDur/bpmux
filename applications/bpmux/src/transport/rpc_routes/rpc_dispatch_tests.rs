#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::validation::{
    AGENT_HEADER, agent_from_headers, args_from_value, is_read_only_tool, known_tools,
};
use axum::http::{HeaderMap, HeaderValue};

fn header_map(values: &[(&'static str, &'static str)]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (k, v) in values {
        headers.insert(*k, HeaderValue::from_str(v).expect("test header"));
    }
    headers
}

#[test]
fn agent_from_headers_accepts_known_role_lowercase() {
    let h = header_map(&[(AGENT_HEADER, "dev")]);
    assert_eq!(agent_from_headers(&h).unwrap(), "dev");
}

#[test]
fn agent_from_headers_canonicalizes_case_and_whitespace() {
    let h = header_map(&[(AGENT_HEADER, "  PM  ")]);
    assert_eq!(agent_from_headers(&h).unwrap(), "pm");
}

#[test]
fn agent_from_headers_rejects_missing_header() {
    let err = agent_from_headers(&HeaderMap::new()).unwrap_err();
    assert!(
        err.contains("required"),
        "expected 'required' in {err}, got {err:?}"
    );
}

#[test]
fn agent_from_headers_rejects_unknown_role() {
    let h = header_map(&[(AGENT_HEADER, "intern")]);
    let err = agent_from_headers(&h).unwrap_err();
    assert!(
        err.contains("not a known role"),
        "expected 'not a known role' in {err}, got {err:?}"
    );
}

#[test]
fn agent_from_headers_rejects_empty_value() {
    let h = header_map(&[(AGENT_HEADER, "   ")]);
    let err = agent_from_headers(&h).unwrap_err();
    assert!(
        err.contains("empty"),
        "expected 'empty' in {err}, got {err:?}"
    );
}

#[test]
fn args_from_value_accepts_object() {
    let body = serde_json::json!({"a": 1, "b": "two"});
    let args = args_from_value(body).unwrap();
    assert_eq!(args.len(), 2);
}

#[test]
fn args_from_value_rejects_non_object() {
    let body = serde_json::json!([1, 2]);
    let err = args_from_value(body).unwrap_err();
    assert!(err.contains("must be a JSON object"));
}

#[test]
fn known_tools_covers_full_state_machine_and_critical_reads() {
    let tools = known_tools();
    for required in [
        "start_story",
        "deliver_story",
        "pass_desk_check",
        "fail_desk_check",
        "accept_story",
        "reject_story",
        "close_story",
        "view_board",
        "get_story",
        "list_ready",
    ] {
        assert!(
            tools.contains(&required),
            "{required} must be in known_tools (load-bearing for canonical-store cutover)"
        );
    }
}

#[test]
fn is_read_only_tool_distinguishes_mutations_from_queries() {
    assert!(is_read_only_tool("view_board"));
    assert!(is_read_only_tool("get_story"));
    assert!(!is_read_only_tool("start_story"));
    assert!(!is_read_only_tool("deliver_story"));
}
