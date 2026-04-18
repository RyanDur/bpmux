use rmcp::model::Tool;

pub fn save_briefing_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "content": {
                "type": "string",
                "description": "Full briefing content (markdown). Replaces any existing briefing. Max 1MB; UTF-8 only."
            }
        },
        "required": ["content"]
    });
    Tool::new(
        "save_briefing",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
