use rmcp::model::Tool;

pub fn simulate_type_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "text": {
                "type": "string",
                "description": "Multi-char text to insert into the current input buffer. Same observable effect as Ryan typing each character."
            }
        },
        "required": ["text"]
    });
    Tool::new(
        "simulate_type",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
