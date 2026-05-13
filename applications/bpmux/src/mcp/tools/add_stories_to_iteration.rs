use rmcp::model::Tool;

pub fn add_stories_to_iteration_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "story_ids": {
                "type": "array",
                "items": { "type": "integer" },
                "description": "List of story numbers to add to the iteration."
            },
            "iteration_number": {
                "type": "integer",
                "description": "Issue number of the Iteration story (story_type=Iteration). Each story gets the iteration_id label applied via the canonical mutation chain."
            }
        },
        "required": ["story_ids", "iteration_number"]
    });
    Tool::new(
        "add_stories_to_iteration",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
