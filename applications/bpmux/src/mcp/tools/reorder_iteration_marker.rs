use rmcp::model::Tool;

pub fn reorder_iteration_marker_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "marker_id": {
                "type": "integer",
                "description": "ID of the iteration marker to reposition (from list_iteration_markers)."
            },
            "position": {
                "type": "integer",
                "description": "1-indexed position in the icebox_unified column. Stories above the marker promote on close_iteration; below stay queued for later iterations."
            }
        },
        "required": ["marker_id", "position"]
    });
    Tool::new(
        "reorder_iteration_marker",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
