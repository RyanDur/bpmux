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
            },
            "topic": {
                "type": "string",
                "description": "Topic tag (#191). Scopes the message to a named topic — only visible in that topic's view. Omit for broadcast (visible in all topics)."
            },
            "thread_id": {
                "type": "string",
                "description": "Thread id (#268). Tag the message as belonging to a named thread so participants see it scoped and non-participants see it in broadcast with the thread name. Get the id from start_thread's response. Omit for broadcast."
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
