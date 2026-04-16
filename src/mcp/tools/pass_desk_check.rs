use rmcp::model::Tool;

pub fn pass_desk_check_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number that passed desk-check"
            }
        },
        "required": ["number"]
    });
    Tool::new(
        "pass_desk_check",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
