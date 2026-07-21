use rmcp::model::Tool;

pub fn set_status_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "status": {
                "type": "string",
                "description": "Retired (#1404): status is hook-managed; this field is ignored."
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
