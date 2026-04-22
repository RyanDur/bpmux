use rmcp::model::Tool;

pub fn add_value_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "The value name (e.g. 'trust', 'stewardship')."
            },
            "description": {
                "type": "string",
                "description": "Short description of what this value captures."
            }
        },
        "required": ["name", "description"]
    });
    Tool::new(
        "add_value",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
