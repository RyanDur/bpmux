use rmcp::model::Tool;

pub fn create_iteration_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "REQUIRED: short identifier for the iteration (e.g., 'Aquila', 'Pegasus', 'Cassiopeia'). Surfaces as the GH issue title 'Iteration {name}' and is the duplicate-rejection key — an OPEN iteration with the same name is rejected with a named-collision error at create-time (sister-discipline to #1281)."
            },
            "position_after": {
                "type": "integer",
                "description": "OPTIONAL: GH-issue-number of the story (or iteration) the new marker should appear AFTER in the icebox positional order. Omit to insert at the top of the icebox (next-iteration anchor). Stories above the marker become implicit-members per Litetracker positional-membership semantic.",
                "minimum": 1
            }
        },
        "required": ["name"]
    });
    Tool::new(
        "create_iteration",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
