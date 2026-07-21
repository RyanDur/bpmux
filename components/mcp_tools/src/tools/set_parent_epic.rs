use rmcp::model::Tool;

pub fn set_parent_epic_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Issue number of the child story"
            },
            "epic_id": {
                "type": "integer",
                "description": "Issue number of the Epic that owns the child"
            }
        },
        "required": ["id", "epic_id"]
    });
    Tool::new(
        "set_parent_epic",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
