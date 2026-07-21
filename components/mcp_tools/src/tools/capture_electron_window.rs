use rmcp::model::Tool;

pub fn capture_electron_window_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    });
    Tool::new(
        "capture_electron_window",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
