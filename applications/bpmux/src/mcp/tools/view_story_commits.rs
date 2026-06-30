use rmcp::model::Tool;

pub fn view_story_commits_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Story issue number whose landing commits to fetch"
            }
        },
        "required": ["id"]
    });
    Tool::new(
        "view_story_commits",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
