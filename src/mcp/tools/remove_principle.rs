use rmcp::model::Tool;

pub fn remove_principle_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "value_name": {
                "type": "string",
                "description": "The value the principle lives under."
            },
            "principle": {
                "type": "string",
                "description": "The principle text to remove."
            }
        },
        "required": ["value_name", "principle"]
    });
    Tool::new(
        "remove_principle",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
