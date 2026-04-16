use rmcp::model::Tool;

pub fn get_bookmark_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "role": {
                "type": "string",
                "description": "Role to read bookmark for. Omit for your own."
            }
        }
    });

    Tool::new(
        "get_bookmark",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
