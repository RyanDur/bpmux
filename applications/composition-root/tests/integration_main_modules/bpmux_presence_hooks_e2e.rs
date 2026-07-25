#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

fn plugin_hooks_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../claude-plugin/hooks")
}

fn commands_for(hooks_json: &serde_json::Value, event: &str) -> Vec<String> {
    hooks_json["hooks"][event]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .flat_map(|block| block["hooks"].as_array().cloned().unwrap_or_default())
                .filter_map(|hook| hook["command"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_presence_bundle_routes_dedicated_events_to_bpmux_signal_with_no_script() {
    let hooks_dir = plugin_hooks_dir();
    let raw = std::fs::read_to_string(hooks_dir.join("hooks.json")).expect("read hooks.json");
    let hooks_json: serde_json::Value =
        serde_json::from_str(&raw).expect("hooks.json is valid json");

    let routes = |event: &str, state: &str| {
        commands_for(&hooks_json, event)
            .iter()
            .any(|command| command.contains(&format!("bpmux signal {state}")))
    };
    assert!(
        routes("UserPromptSubmit", "busy"),
        "turn-start routes directly to `bpmux signal busy` (no script indirection); commands: {:?}",
        commands_for(&hooks_json, "UserPromptSubmit")
    );
    assert!(
        routes("Stop", "idle"),
        "definitive turn-end routes to `bpmux signal idle`; commands: {:?}",
        commands_for(&hooks_json, "Stop")
    );
    assert!(
        routes("PermissionRequest", "help"),
        "needs-you routes through the DEDICATED PermissionRequest event (a real permission dialog), the documented genuine-attention signal; commands: {:?}",
        commands_for(&hooks_json, "PermissionRequest")
    );
    assert!(
        hooks_json["hooks"].get("Notification").is_none(),
        "Notification is NOT wired for presence — it also fires on the 60s idle-waiting, which is the exact false-red this design avoids by routing help through the dedicated PermissionRequest event instead"
    );
    assert!(
        !raw.contains("signal-presence"),
        "no signal-presence.sh indirection — hooks call `bpmux signal` directly (S1's gated verb, proven by bpmux_signal_cli_e2e); the discrimination is the platform's dedicated events, not bash"
    );
    assert!(
        !hooks_dir.join("signal-presence.sh").exists(),
        "the superseded signal-presence.sh is removed, not commented (remove-superseded on refactor)"
    );
}

#[test]
fn post_tool_use_pushes_busy_so_a_resolved_help_clears_not_persists() {
    let hooks_dir = plugin_hooks_dir();
    let raw = std::fs::read_to_string(hooks_dir.join("hooks.json")).expect("read hooks.json");
    let hooks_json: serde_json::Value =
        serde_json::from_str(&raw).expect("hooks.json is valid json");

    let post_tool_routes_busy = commands_for(&hooks_json, "PostToolUse")
        .iter()
        .any(|command| command.contains("bpmux signal busy"));
    assert!(
        post_tool_routes_busy,
        "#1747 clear-on-resolve: PostToolUse routes to `bpmux signal busy`. Claude Code has no permission-answered event, but the OBSERVABLE resolve is the agent resuming tool-use — after a grant the gated tool runs and PostToolUse fires, pushing busy so a stale help CLEARS (never persists past its cause, the trustworthy-dot bar) and a long mid-turn tool-wait stays busy (no false-idle). commands: {:?}",
        commands_for(&hooks_json, "PostToolUse")
    );
}
