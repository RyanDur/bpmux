use rmcp::model::Tool;

pub fn view_board_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "state": {
                "type": "string",
                "enum": ["icebox", "ready", "in_progress", "desk_check", "delivered", "accepted", "retired"],
                "description": "Scope to a single column. Omit for all columns."
            },
            "iteration": {
                "type": "integer",
                "description": "Scope to stories whose parent-iteration issue number equals this. Omit for all iterations."
            },
            "row_limit": {
                "type": "integer",
                "description": "Cap rows rendered per column; the column count still reflects the full total. Omit for no cap."
            }
        }
    });
    Tool::new(
        "view_board",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
