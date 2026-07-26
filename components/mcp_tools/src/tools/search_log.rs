use rmcp::model::Tool;

pub fn search_log_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search keyword or phrase"
            },
            "sender": {
                "type": "string",
                "description": "Filter by sender name (e.g. 'owner')"
            },
            "addressee": {
                "type": "string",
                "description": "Only messages addressed to this role or user id (broadcasts excluded)"
            }
        },
        "required": ["query"]
    });

    Tool::new(
        "search_log",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
