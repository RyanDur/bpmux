use rmcp::model::Tool;

pub fn subscribe_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "event_class": {
                "type": "string",
                "description": "The event class to subscribe to (e.g. \"board_invalidated\")."
            }
        },
        "required": ["event_class"]
    });
    Tool::new(
        "subscribe",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn unsubscribe_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "subscription_id": {
                "type": "string",
                "description": "The subscription_id returned by subscribe."
            }
        },
        "required": ["subscription_id"]
    });
    Tool::new(
        "unsubscribe",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn wait_for_event_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "subscription_id": {
                "type": "string",
                "description": "The subscription_id to block on (from subscribe)."
            },
            "timeout_ms": {
                "type": "integer",
                "description": "Max milliseconds to block before returning {\"timed_out\": true} (default 30000)."
            }
        },
        "required": ["subscription_id"]
    });
    Tool::new(
        "wait_for_event",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
