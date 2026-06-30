use rmcp::model::Tool;

pub fn get_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Issue number to fetch"
            }
        },
        "required": ["id"]
    });
    Tool::new(
        "get_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
