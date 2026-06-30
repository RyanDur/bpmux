use rmcp::model::Tool;

pub fn unset_parent_epic_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Issue number of the child story to ungroup from its current Epic"
            }
        },
        "required": ["id"]
    });
    Tool::new(
        "unset_parent_epic",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
