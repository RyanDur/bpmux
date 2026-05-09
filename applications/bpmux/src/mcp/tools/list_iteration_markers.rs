use rmcp::model::Tool;

pub fn list_iteration_markers_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    });
    Tool::new(
        "list_iteration_markers",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
