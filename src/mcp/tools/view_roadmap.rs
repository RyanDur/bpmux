use rmcp::model::Tool;

pub fn view_roadmap_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });
    Tool::new(
        "view_roadmap",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
