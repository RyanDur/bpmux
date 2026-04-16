use rmcp::model::Tool;

pub fn accept_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to accept"
            }
        },
        "required": ["number"]
    });
    Tool::new(
        "accept_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
