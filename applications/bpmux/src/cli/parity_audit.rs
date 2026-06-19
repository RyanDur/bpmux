use clap::CommandFactory;
use std::collections::BTreeSet;

const IN_SCOPE: &[(&str, &[&str])] = &[
    ("send_message", &["send"]),
    ("read_messages", &["read"]),
    ("wait_for_event", &["wait-for-event"]),
    ("search_log", &["search"]),
    ("list_lessons", &["lesson", "list"]),
    ("list_peer_lessons", &["lesson", "list-peer"]),
    ("save_lesson", &["lesson", "save"]),
    ("capture_electron_window", &["capture"]),
    ("save_bookmark", &["bookmark", "save"]),
    ("get_bookmark", &["bookmark", "get"]),
    ("save_briefing", &["briefing", "save"]),
    ("get_briefing", &["briefing", "get"]),
    ("get_identity", &["identity"]),
    ("add_value", &["value", "add"]),
    ("add_principle", &["principle", "add"]),
    ("update_principle", &["principle", "update"]),
    ("remove_principle", &["principle", "remove"]),
    ("get_ci_state", &["ci", "state"]),
    ("get_ci_log_for_run", &["ci", "log"]),
    ("ci_get_job_status", &["ci", "jobs"]),
    ("rerun_ci", &["ci", "rerun"]),
    ("get_sync_status", &["sync-status"]),
    ("view_story_commits", &["commits"]),
    ("view_commit_diff", &["diff"]),
    ("view_velocity", &["velocity"]),
    ("start_story", &["start"]),
    ("deliver_story", &["deliver"]),
    ("pass_desk_check", &["pass"]),
    ("fail_desk_check", &["fail"]),
    ("accept_story", &["accept"]),
    ("reject_story", &["reject"]),
    ("close_story", &["close"]),
    ("create_story", &["create"]),
    ("create_iteration", &["iteration", "create"]),
    ("comment_on_story", &["comment"]),
    ("label_story", &["label"]),
    ("move_story", &["move"]),
    ("set_parent_epic", &["epic", "set-parent"]),
    ("unset_parent_epic", &["epic", "unset-parent"]),
    ("add_story_to_iteration", &["iteration", "add"]),
    ("designate_current_iteration", &["iteration", "designate"]),
    ("edit_story", &["edit"]),
    ("view_board", &["board"]),
    ("get_story", &["story"]),
    ("list_ready", &["ready"]),
    ("list_epic_children", &["epic", "children"]),
    ("view_story_timeline", &["timeline"]),
    ("view_comments", &["comments"]),
    ("view_backlog", &["backlog"]),
    ("close_iteration", &["iteration", "close"]),
    ("view_roadmap", &["roadmap"]),
    ("submit_verdict", &["verdict"]),
    ("view_verdicts", &["verdicts"]),
];

const EXCLUDED: &[(&str, &str)] = &[
    (
        "set_name",
        "bridge identity handshake — bootstrap, not an agent capability",
    ),
    (
        "set_status",
        "status is hook-managed; agents never self-report it",
    ),
    (
        "subscribe",
        "registers event-classes to the bridge's MCP push-stream; an off-MCP agent drains events via the in-scope wait-for-event pull",
    ),
    (
        "unsubscribe",
        "the inverse of subscribe — MCP push-stream deregistration; off-MCP receive is the wait-for-event pull",
    ),
    (
        "share_message",
        "coordination relay — reshares a peer's message with provenance; an agent's own workflow uses send_message (in-scope), so excluding the reshare strands nothing",
    ),
    (
        "classifier_misfire",
        "classifier-feedback telemetry, not a product capability",
    ),
    ("get_token_report", "token telemetry"),
    ("get_usage", "usage telemetry"),
    (
        "view_tokens",
        "opens the owner-facing tokens TUI view (a renderer-open, not agent data — cf. get_token_report which returns the JSON)",
    ),
    ("dev_in_flight", "dev WIP-introspection telemetry"),
    (
        "reorder_story",
        "superseded by the move verb's --position (bpmux move <n> <same-state> --position covers reordering); retired per #1403, still advertised as a guiding tombstone",
    ),
    (
        "user_fn",
        "user-extension mechanism, not a fixed agent capability",
    ),
    (
        "list_user_fns",
        "user-extension mechanism, not a fixed agent capability",
    ),
];

fn verb_exists(path: &[&str]) -> bool {
    if path.is_empty() {
        return false;
    }
    let mut command = crate::Cli::command();
    for (index, segment) in path.iter().enumerate() {
        let Some(sub) = command.find_subcommand(segment).cloned() else {
            return false;
        };
        if index + 1 == path.len() {
            return true;
        }
        command = sub;
    }
    false
}

#[test]
fn every_dispatched_mcp_tool_is_either_a_bpmux_verb_or_a_reasoned_exclusion() {
    let in_scope: BTreeSet<&str> = IN_SCOPE.iter().map(|(tool, _)| *tool).collect();
    let excluded: BTreeSet<&str> = EXCLUDED.iter().map(|(tool, _)| *tool).collect();

    let overlap: Vec<&str> = in_scope.intersection(&excluded).copied().collect();
    assert!(
        overlap.is_empty(),
        "tools classified as both in-scope and excluded: {overlap:?}"
    );

    let unaccounted: Vec<&str> = crate::bridge::DISPATCHED_TOOL_NAMES
        .iter()
        .filter(|tool| !in_scope.contains(*tool) && !excluded.contains(*tool))
        .copied()
        .collect();
    assert!(
        unaccounted.is_empty(),
        "MCP tools with neither a bpmux verb nor a reasoned exclusion — a production agent off MCP would strand on these: {unaccounted:?}"
    );

    let known: BTreeSet<&str> = crate::bridge::DISPATCHED_TOOL_NAMES
        .iter()
        .copied()
        .collect();
    let stale: Vec<&str> = in_scope
        .iter()
        .chain(excluded.iter())
        .filter(|tool| !known.contains(*tool))
        .copied()
        .collect();
    assert!(
        stale.is_empty(),
        "the parity map / exclusion list references tools absent from DISPATCHED_TOOL_NAMES (stale after a tool rename/removal): {stale:?}"
    );
}

#[test]
fn every_in_scope_tool_resolves_to_a_real_bpmux_verb_path() {
    let mut missing: Vec<String> = Vec::new();
    for (tool, path) in IN_SCOPE {
        if !verb_exists(path) {
            missing.push(format!("{tool} -> {path:?}"));
        }
    }
    assert!(
        missing.is_empty(),
        "in-scope MCP tools whose mapped bpmux verb-path is absent from the clap command tree: {missing:?}"
    );
}

#[test]
fn every_exclusion_carries_a_non_empty_reason() {
    for (tool, reason) in EXCLUDED {
        assert!(
            !reason.trim().is_empty(),
            "exclusion {tool} must carry a non-empty reason"
        );
    }
}
