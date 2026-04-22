use rmcp::model::Tool;

pub fn simulate_click_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "x": { "type": "integer", "description": "Column position (0-based)" },
            "y": { "type": "integer", "description": "Row position (0-based)" }
        },
        "required": ["x", "y"]
    });
    Tool::new(
        "simulate_click",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
