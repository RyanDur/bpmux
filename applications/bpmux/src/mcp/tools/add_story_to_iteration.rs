use rmcp::model::Tool;

pub fn add_story_to_iteration_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "iteration_id": {
                "type": "integer",
                "description": "Iteration entity issue number (the parent iteration this story is being added to)."
            },
            "story_id": {
                "type": "integer",
                "description": "Member story issue number (the story being added to the iteration)."
            },
            "position": {
                "type": "integer",
                "description": "Position within the iteration (1-indexed)."
            }
        },
        "required": ["iteration_id", "story_id", "position"]
    });
    Tool::new(
        "add_story_to_iteration",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
