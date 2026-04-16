use rmcp::model::Tool;

pub fn get_usage_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });
    Tool::new(
        "get_usage",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
