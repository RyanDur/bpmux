use rmcp::model::Tool;

pub fn create_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "title": {
                "type": "string",
                "description": "Story title"
            },
            "body": {
                "type": "string",
                "description": "Story body (markdown). Include ## What, ## Why, ## Acceptance criteria sections."
            }
        },
        "required": ["title"]
    });
    Tool::new(
        "create_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
