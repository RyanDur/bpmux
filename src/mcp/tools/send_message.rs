use rmcp::model::Tool;

pub fn send_message_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "text": {
                "type": "string",
                "description": "The message text to send"
            },
            "to": {
                "type": "array",
                "items": { "type": "string" },
                "description": "PRIVACY FILTER — optional list of recipient names (short form, e.g. [\"pm\", \"designer\"]). When set, only the listed roles can SEE the message. Omit for broadcast. Distinct from 'addressee': 'to' controls visibility, 'addressee' controls attention."
            },
            "addressee": {
                "type": "string",
                "description": "ATTENTION SIGNAL (#158) — optional role this message is directed at. Everyone in 'to' still receives the message, but only the addressed role should treat it as their turn to respond. Others should see it in their log without responding. Use a role name (e.g. \"pm\", \"dev\", \"designer\") or \"all\" for explicit broadcast attention. Omit if the message is not directed at any specific role."
            },
            "message_type": {
                "type": "string",
                "enum": ["chat", "insight"],
                "description": "Message type. Use 'insight' for reasoning traces and architectural decisions. Omit for normal chat."
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
