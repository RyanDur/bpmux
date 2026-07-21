use rmcp::model::Tool;

pub fn list_ready_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });
    Tool::new(
        "list_ready",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
