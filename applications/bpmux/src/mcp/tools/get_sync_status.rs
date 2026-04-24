use rmcp::model::Tool;

pub fn get_sync_status_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });

    Tool::new(
        "get_sync_status",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
