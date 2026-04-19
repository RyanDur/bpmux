use rmcp::model::Tool;

pub fn get_ci_state_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });

    Tool::new(
        "get_ci_state",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
