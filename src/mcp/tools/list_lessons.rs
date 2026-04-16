use rmcp::model::Tool;

pub fn list_lessons_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "topic": {
                "type": "string",
                "description": "Filter by topic (substring match). Omit to list all topics."
            },
            "limit": {
                "type": "integer",
                "description": "Return only the top N lessons ranked by reinforcement. Use for lightweight startup orientation."
            }
        }
    });

    Tool::new(
        "list_lessons",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
