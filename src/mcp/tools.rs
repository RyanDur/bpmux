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
                "description": "Optional list of recipient names (short form, e.g. [\"pm\", \"designer\"]). Omit for broadcast."
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

pub fn set_name_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "The display name to use in the session"
            }
        },
        "required": ["name"]
    });

    Tool::new(
        "set_name",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn set_status_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "status": {
                "type": "string",
                "enum": ["available", "busy", "help"],
                "description": "Your current status: available (green), busy (yellow), or help (red, needs attention)"
            }
        },
        "required": ["status"]
    });

    Tool::new(
        "set_status",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn read_messages_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "limit": {
                "type": "integer",
                "description": "Maximum number of messages to return (default 20)"
            }
        }
    });

    Tool::new(
        "read_messages",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC: send_message tool description tells agents not to self-prefix.
    #[test]
    fn send_message_description_warns_against_name_prefix() {
        let desc =
            "Send a message. Do not prefix your name — the session adds the sender automatically.";
        let tool = send_message_tool(desc);
        let tool_desc = tool.description.unwrap_or_default();
        assert!(
            tool_desc.contains("Do not prefix your name"),
            "send_message description should warn against self-prefixing, got: {tool_desc}"
        );
    }
}
