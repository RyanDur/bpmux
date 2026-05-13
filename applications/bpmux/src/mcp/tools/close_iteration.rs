use rmcp::model::Tool;

pub fn close_iteration_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "iteration_number": {
                "type": "integer",
                "description": "Issue number of the Iteration story (story_type=Iteration) to close. Closes the GH iteration-issue + promotes member-stories (queried via iteration_id) to ready via the canonical mutation chain. Preserves the ready-empty guard and IterationClose state-delta broadcast."
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
