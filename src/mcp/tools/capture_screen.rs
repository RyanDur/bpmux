use rmcp::model::Tool;

pub fn capture_screen_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });

    Tool::new(
        "capture_screen",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
