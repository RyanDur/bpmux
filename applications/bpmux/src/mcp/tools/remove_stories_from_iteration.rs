use rmcp::model::Tool;

pub fn remove_stories_from_iteration_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "story_ids": {
                "type": "array",
                "items": { "type": "integer" },
                "description": "Story numbers to remove from their current iteration. Each story moves to the unscheduled bucket (immediately below the bottom-most marker). Stories not in any iteration are no-ops."
            }
        },
        "required": ["story_ids"]
    });
    Tool::new(
        "remove_stories_from_iteration",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
