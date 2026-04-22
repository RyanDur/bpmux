use rmcp::model::Tool;

pub fn get_token_report_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });

    Tool::new(
        "get_token_report",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
