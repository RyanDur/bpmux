use rmcp::model::Tool;

pub fn save_bookmark_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "current_task": {
                "type": "string",
                "description": "What you are currently working on"
            },
            "decisions": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Key decisions made this session"
            },
            "open_questions": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Unresolved threads or questions"
            },
            "next_step": {
                "type": "string",
                "description": "What you expect to do next"
            },
            "notes": {
                "type": "string",
                "description": "Free-text notes"
            }
        }
    });

    Tool::new(
        "save_bookmark",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
