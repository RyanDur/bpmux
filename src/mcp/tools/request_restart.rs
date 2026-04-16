use rmcp::model::Tool;

pub fn request_restart_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });
    Tool::new(
        "request_restart",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
