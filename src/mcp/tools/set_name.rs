use rmcp::model::Tool;

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
