use rmcp::model::Tool;

pub fn send_message_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "text": {
                "type": "string",
                "description": "The message text to send"
            }
        },
        "required": ["text"]
    });

    Tool::new(
        "send_message",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn read_messages_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });

    Tool::new(
        "read_messages",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
