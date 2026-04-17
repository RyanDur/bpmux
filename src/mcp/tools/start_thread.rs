use rmcp::model::Tool;

pub fn start_thread_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Human-readable thread name (e.g. 'cheese', 'retro-notes')."
            },
            "participants": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Roles invited to the thread (e.g. ['ryan', 'designer']). Participants see the thread scoped; non-participants see thread messages in broadcast with the name tagged."
            }
        },
        "required": ["name", "participants"]
    });
    Tool::new(
        "start_thread",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
