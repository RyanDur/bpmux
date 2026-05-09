use rmcp::model::Tool;

pub fn create_iteration_marker_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "theme_name": {
                "type": "string",
                "description": "Optional constellation theme name (e.g. 'Orion', 'Lyra'). Omit to auto-pick the next unused name from the constellation pool."
            }
        },
        "required": []
    });
    Tool::new(
        "create_iteration_marker",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
