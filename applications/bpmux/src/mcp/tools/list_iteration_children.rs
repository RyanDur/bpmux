use rmcp::model::Tool;

pub fn list_iteration_children_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "iteration_number": {
                "type": "integer",
                "description": "Issue number of the Iteration story whose member-stories to list."
            }
        },
        "required": ["iteration_number"]
    });
    Tool::new(
        "list_iteration_children",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
