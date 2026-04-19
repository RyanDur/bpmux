use rmcp::model::Tool;

pub fn comment_on_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to comment on"
            },
            "body": {
                "type": "string",
                "description": "Comment body — markdown supported. Durable audit-trail (visible to any future reader of the issue)."
            }
        },
        "required": ["number", "body"]
    });

    Tool::new(
        "comment_on_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
