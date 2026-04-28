use rmcp::model::Tool;

pub fn edit_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number whose body to replace"
            },
            "body": {
                "type": "string",
                "description": "New full markdown body — replaces the existing body in entirety. Cascade: GitHub issue updated → canonical store refreshed → board-invalidate broadcast."
            }
        },
        "required": ["number", "body"]
    });

    Tool::new(
        "edit_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
