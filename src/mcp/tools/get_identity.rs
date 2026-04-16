use rmcp::model::Tool;

pub fn get_identity_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "role": {
                "type": "string",
                "description": "Role to query identity for (e.g. 'dev', 'pm', 'designer'). Omit to get your own."
            }
        }
    });

    Tool::new(
        "get_identity",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
