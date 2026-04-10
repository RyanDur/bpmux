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
                "description": "Your status: 'available', 'busy', or 'help'. Add a colon and description for context: 'busy: building #123'"
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

pub fn save_lesson_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "topic": {
                "type": "string",
                "description": "Topic name (e.g. 'role boundaries')"
            },
            "lesson": {
                "type": "string",
                "description": "What you learned from this experience"
            },
            "evidence": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Message timestamps that support this lesson"
            }
        },
        "required": ["topic", "lesson"]
    });

    Tool::new(
        "save_lesson",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn list_lessons_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "topic": {
                "type": "string",
                "description": "Filter by topic (substring match). Omit to list all topics."
            },
            "limit": {
                "type": "integer",
                "description": "Return only the top N lessons ranked by reinforcement. Use for lightweight startup orientation."
            }
        }
    });

    Tool::new(
        "list_lessons",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn list_peer_lessons_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "peer": {
                "type": "string",
                "description": "Peer name to query lessons from. Omit to list available peers."
            }
        }
    });

    Tool::new(
        "list_peer_lessons",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn simulate_click_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "x": { "type": "integer", "description": "Column position (0-based)" },
            "y": { "type": "integer", "description": "Row position (0-based)" }
        },
        "required": ["x", "y"]
    });
    Tool::new(
        "simulate_click",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn simulate_key_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "key": {
                "type": "string",
                "description": "Key to press: single character, or 'enter', 'escape', 'tab', 'backspace', 'up', 'down', 'left', 'right'"
            }
        },
        "required": ["key"]
    });
    Tool::new(
        "simulate_key",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn capture_screen_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });

    Tool::new(
        "capture_screen",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn search_log_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search keyword or phrase"
            },
            "sender": {
                "type": "string",
                "description": "Filter by sender name (e.g. 'ryan')"
            }
        },
        "required": ["query"]
    });

    Tool::new(
        "search_log",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn classifier_misfire_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "nudge_hash": {
                "type": "string",
                "description": "Short identifier for the nudge that misfired. Can be a hash of the nudge body, a timestamp fragment, or any stable reference the reporter chooses."
            },
            "reason": {
                "type": "string",
                "description": "OPTIONAL free-text reason describing why this fire was wrong. The feedback path does NOT require a reason — per Naur's 'state observation, not conclusion' framing, simply marking the fire as wrong is valuable training data on its own. Provide a reason only when you have a clear one."
            }
        },
        "required": ["nudge_hash"]
    });

    Tool::new(
        "classifier_misfire",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
