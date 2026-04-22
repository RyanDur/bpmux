use rmcp::model::Tool;

pub fn fail_desk_check_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number that failed desk-check"
            },
            "reason": {
                "type": "string",
                "description": "What failed — surfaced to dev as the workflow signal"
            }
        },
        "required": ["number", "reason"]
    });
    Tool::new(
        "fail_desk_check",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
