use rmcp::model::Tool;

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
