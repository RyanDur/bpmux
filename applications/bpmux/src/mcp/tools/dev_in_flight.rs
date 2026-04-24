use rmcp::model::Tool;

pub fn dev_in_flight_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });
    Tool::new(
        "dev_in_flight",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
