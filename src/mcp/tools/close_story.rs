use rmcp::model::Tool;

pub fn close_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to close"
            },
            "reason": {
                "type": "string",
                "description": "Closure reason — added as comment on the issue before close. Required: audit trail for superseded / duplicate / won't-fix closures."
            }
        },
        "required": ["number", "reason"]
    });

    Tool::new(
        "close_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
