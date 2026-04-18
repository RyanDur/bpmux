use rmcp::model::Tool;

pub fn close_iteration_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    });
    Tool::new(
        "close_iteration",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
