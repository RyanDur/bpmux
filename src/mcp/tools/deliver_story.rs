use rmcp::model::Tool;

pub fn deliver_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to mark as delivered"
            }
        },
        "required": ["number"]
    });
    Tool::new(
        "deliver_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
