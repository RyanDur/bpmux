use rmcp::model::Tool;

pub fn close_iteration_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "iteration_number": {
                "type": "integer",
                "description": "Issue number of the Iteration story (story_type=Iteration) to close. Terminal close: closes the GH iteration-issue and clears its current-designation if set; promotes nothing (promote-on-close retired #1556 — members reach ready via the open/designate ceremony). Preserves the ready-empty guard (#1258) and IterationClose state-delta broadcast."
            }
        },
        "required": ["iteration_number"]
    });
    Tool::new(
        "close_iteration",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
