use rmcp::model::Tool;

pub fn view_tokens_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });
    Tool::new(
        "view_tokens",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
