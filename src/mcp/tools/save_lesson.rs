use rmcp::model::Tool;

pub fn save_lesson_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "topic": {
                "type": "string",
                "description": "Topic name (e.g. 'role boundaries')"
            },
            "lesson": {
                "type": "string",
                "description": "What you learned from this experience"
            },
            "evidence": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Message timestamps that support this lesson"
            }
        },
        "required": ["topic", "lesson"]
    });

    Tool::new(
        "save_lesson",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
