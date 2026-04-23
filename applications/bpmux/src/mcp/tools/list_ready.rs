use rmcp::model::Tool;

// #403: read-side bypass closure (paired with get_story). Returns
// the full ready queue, not a sample — agents pickup-gate-paraphrase
// from this list rather than `gh issue list --label ready`.
// `view_backlog` surfaces counts + up-to-5 samples; when the queue
// runs longer, the sample truncation forced agents to gh.
pub fn list_ready_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });
    Tool::new(
        "list_ready",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
