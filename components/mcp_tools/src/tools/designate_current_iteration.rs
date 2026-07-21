use rmcp::model::Tool;

pub fn designate_current_iteration_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "iteration_id": {
                "type": "integer",
                "description": "Iteration entity issue number to designate as the current iteration (the one the NOW view shows). Replaces any prior current designation; closing the iteration clears it."
            }
        },
        "required": ["iteration_id"]
    });
    Tool::new(
        "designate_current_iteration",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
