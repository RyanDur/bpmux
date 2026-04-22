use rmcp::model::Tool;

pub fn update_principle_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "value_name": {
                "type": "string",
                "description": "The value containing the principle."
            },
            "old": {
                "type": "string",
                "description": "The current principle text to replace."
            },
            "new": {
                "type": "string",
                "description": "The refined principle text."
            }
        },
        "required": ["value_name", "old", "new"]
    });
    Tool::new(
        "update_principle",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
