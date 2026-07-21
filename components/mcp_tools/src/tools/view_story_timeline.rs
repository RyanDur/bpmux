use rmcp::model::Tool;

pub fn view_story_timeline_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Issue number whose gate-transition timeline to fetch"
            }
        },
        "required": ["id"]
    });
    Tool::new(
        "view_story_timeline",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
