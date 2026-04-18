use rmcp::model::Tool;

pub fn view_backlog_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    });
    Tool::new(
        "view_backlog",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
