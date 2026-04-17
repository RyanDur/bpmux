use rmcp::model::Tool;

pub fn join_thread_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Name of the thread to join (received via a pull_into_thread invitation)."
            }
        },
        "required": ["name"]
    });
    Tool::new(
        "join_thread",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
