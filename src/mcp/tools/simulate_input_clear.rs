use rmcp::model::Tool;

pub fn simulate_input_clear_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });
    Tool::new(
        "simulate_input_clear",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
