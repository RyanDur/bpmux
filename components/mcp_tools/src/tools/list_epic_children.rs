use rmcp::model::Tool;

pub fn list_epic_children_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "epic_id": {
                "type": "integer",
                "description": "Issue number of the Epic story whose children to list."
            }
        },
        "required": ["epic_id"]
    });
    Tool::new(
        "list_epic_children",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
