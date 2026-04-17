use rmcp::model::Tool;

pub fn add_principle_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "value_name": {
                "type": "string",
                "description": "The value to append under (e.g. 'correctness', 'honesty')."
            },
            "principle": {
                "type": "string",
                "description": "The principle text to add."
            }
        },
        "required": ["value_name", "principle"]
    });
    Tool::new(
        "add_principle",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
