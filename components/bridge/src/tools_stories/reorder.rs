pub fn reorder_story_retired_error() -> String {
    serde_json::json!({
        "kind": "tool-retired",
        "retired_axis": "reorder-tool",
        "successor": "move_story",
        "message": "reorder_story is retired per #1403. Use move_story(id, target_state=<same column>, position=<N>, rationale=<why>) — position-axis is uniform with state-transition substrate via #1392 universal-move. Internal position-mutation algorithm preserved; only the dedicated reorder_story tool retires."
    })
    .to_string()
}

pub async fn handle_reorder_story(
    _args: &serde_json::Map<String, serde_json::Value>,
    _agent_name: &str,
    _refresh: Option<&std::sync::Arc<sync::SyncOrchestrator>>,
) -> Result<String, String> {
    Err(reorder_story_retired_error())
}
