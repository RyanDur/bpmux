use rmcp::model::Tool;

pub fn set_parent_epic_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number of the child story"
            },
            "epic_number": {
                "type": "integer",
                "description": "Issue number of the Epic that owns the child"
            }
        },
        "required": ["number", "epic_number"]
    });
    Tool::new(
        "set_parent_epic",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
