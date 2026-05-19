use rmcp::model::Tool;

pub fn view_verdicts_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "story_number": {
                "type": "integer",
                "description": "Issue number whose verdicts to read."
            }
        },
        "required": ["story_number"]
    });
    Tool::new(
        "view_verdicts",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
