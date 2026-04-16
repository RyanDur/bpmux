use rmcp::model::Tool;

pub fn label_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to label"
            },
            "label": {
                "type": "string",
                "description": "Label to add (e.g., 'ready', 'blocked', 'icebox', 'deferred')"
            }
        },
        "required": ["number", "label"]
    });
    Tool::new(
        "label_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
