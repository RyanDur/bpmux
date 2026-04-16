use rmcp::model::Tool;

pub fn reject_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to reject"
            },
            "reason": {
                "type": "string",
                "description": "Rejection reason — added as a comment on the issue"
            }
        },
        "required": ["number"]
    });
    Tool::new(
        "reject_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
