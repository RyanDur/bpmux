use rmcp::model::Tool;

pub fn add_stories_to_iteration_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "story_ids": {
                "type": "array",
                "items": { "type": "integer" },
                "description": "Ordered list of story numbers to fold above the iteration marker. story_ids[0] lands at the top of the iteration group; story_ids[N-1] lands just above the marker."
            },
            "marker_id": {
                "type": "integer",
                "description": "Existing iteration-marker id (from list_iteration_markers). Wins when both marker_id + theme_name are provided."
            },
            "theme_name": {
                "type": "string",
                "description": "Optional constellation name; used to auto-create a marker (appended to icebox_unified bottom) when marker_id is omitted."
            }
        },
        "required": ["story_ids"]
    });
    Tool::new(
        "add_stories_to_iteration",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
