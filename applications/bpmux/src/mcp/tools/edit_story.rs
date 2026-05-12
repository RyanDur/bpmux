use rmcp::model::Tool;

pub fn edit_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to edit"
            },
            "body": {
                "type": "string",
                "description": "New full markdown body — replaces the existing body in entirety. Optional: omit to leave body unchanged. Cascade: GitHub issue updated → canonical store refreshed → board-invalidate broadcast."
            },
            "title": {
                "type": "string",
                "description": "New title — replaces the existing title (#1279). Optional: omit to leave title unchanged. Use for in-flight title corrections (typos, template leaks) without losing issue number / comments / state-machine continuity. Cascade: GitHub issue updated → canonical store refreshed → board-invalidate broadcast."
            }
        },
        "required": ["number"]
    });

    Tool::new(
        "edit_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
