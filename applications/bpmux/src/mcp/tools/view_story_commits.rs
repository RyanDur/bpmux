use rmcp::model::Tool;

pub fn view_story_commits_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Story issue number whose landing commits to fetch"
            }
        },
        "required": ["number"]
    });
    Tool::new(
        "view_story_commits",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
