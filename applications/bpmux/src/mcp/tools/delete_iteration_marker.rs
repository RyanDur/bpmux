use rmcp::model::Tool;

pub fn delete_iteration_marker_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "marker_id": {
                "type": "integer",
                "description": "Marker id (from list_iteration_markers) to delete directly without firing close_iteration. Errors if the marker's group contains stories (caller must use remove_stories_from_iteration to clear the group first)."
            }
        },
        "required": ["marker_id"]
    });
    Tool::new(
        "delete_iteration_marker",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
