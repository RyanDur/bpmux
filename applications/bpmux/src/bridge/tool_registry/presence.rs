pub(super) fn presence_tools() -> Vec<rmcp::model::Tool> {
    vec![
        crate::mcp::tools::capture_electron_window_tool(
            "Capture the bpmux Electron renderer window as a PNG (#1183). No args. Returns JSON `{ path, window_id, captured_at, width, height }` where `path` is a file under `~/.bpmux/captures/<timestamp>-<role>.png` that the agent can Read. macOS-only in v1: relies on system `screencapture` + `swift` (Xcode CLT). First-time use prompts macOS Screen Recording permission for the terminal/process running bpmux; until granted, the tool returns `{kind:'permission_denied', detail:...}` with the System Settings path. Other named error shapes: `electron_not_running` (no Electron process visible), `no_window` (Electron running but no bpmux window onscreen), `window_minimized` (window has 0x0 bounds). Captures are namespaced per role; the directory is auto-cleaned after 24h. Use this when you need to see what the owner sees in the Electron app — replaces the owner-drops-screenshot anti-pattern.",
        ),
        crate::mcp::tools::set_name_tool(
            "Set your display name in the bpmux session. Call this once at the start to identify yourself (e.g., 'dev', 'pm').",
        ),
        crate::mcp::tools::set_status_tool(
            "Set your status in the bpmux session. available = green dot (can respond), busy = yellow dot (working), help = red dot (needs attention).",
        ),
        crate::mcp::tools::classifier_misfire_tool(
            "Report a classifier nudge that fired on a message you judge is NOT a learning moment. The reason field is OPTIONAL — per the 'state observation, not conclusion' framing, simply marking a fire as wrong is valuable training data on its own. Use this whenever you see an experience-nudge in the chat that you reason internally is incorrect, so the team can measure classifier accuracy and tune it over time. See #159.",
        ),
    ]
}
