use rmcp::model::Tool;

pub fn view_velocity_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });
    Tool::new(
        "view_velocity",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
