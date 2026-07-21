use rmcp::model::Tool;

pub fn get_briefing_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "role": {
                "type": "string",
                "description": "Role whose briefing to read (e.g. 'dev', 'pm', 'designer'). Omit to read caller's own."
            }
        }
    });
    Tool::new(
        "get_briefing",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
