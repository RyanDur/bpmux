use rmcp::model::Tool;

pub fn view_comments_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number whose comment audit-trail to fetch"
            }
        },
        "required": ["number"]
    });
    Tool::new(
        "view_comments",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
