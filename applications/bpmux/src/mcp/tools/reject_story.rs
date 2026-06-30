use rmcp::model::Tool;

pub fn reject_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Issue number to reject"
            },
            "reason": {
                "type": "string",
                "description": "Rejection reason — added as a comment on the issue"
            }
        },
        "required": ["id"]
    });
    Tool::new(
        "reject_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
