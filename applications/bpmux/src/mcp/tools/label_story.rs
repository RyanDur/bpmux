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
                "description": "Label to add or remove (e.g., 'ready', 'blocked', 'icebox', 'deferred')"
            },
            "operation": {
                "type": "string",
                "enum": ["add", "remove"],
                "description": "Whether to add or remove the label. Defaults to 'add' for backward-compat with pre-#392 callers. Remove is idempotent: removing a label not attached to the story succeeds silently."
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
