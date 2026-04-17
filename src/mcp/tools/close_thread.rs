use rmcp::model::Tool;

pub fn close_thread_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Thread name to close (the human-readable name passed to start_thread)."
            }
        },
        "required": ["name"]
    });
    Tool::new(
        "close_thread",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
