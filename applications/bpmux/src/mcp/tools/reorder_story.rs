use rmcp::model::Tool;

pub fn reorder_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to reorder"
            },
            "state": {
                "type": "string",
                "description": "State column to reorder within (e.g., 'ready', 'in_progress', 'delivered', 'icebox')"
            },
            "position": {
                "type": "integer",
                "description": "Target position (1-indexed). 1 = top of column."
            }
        },
        "required": ["number", "state", "position"]
    });
    Tool::new(
        "reorder_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
