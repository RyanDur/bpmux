use rmcp::model::Tool;

pub fn close_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Issue number to close"
            },
            "reason": {
                "type": "string",
                "description": "Closure reason — added as comment on the issue before close. Required: audit trail for superseded / duplicate / won't-fix closures."
            },
            "disposition": {
                "type": "string",
                "enum": ["retire", "accept", "wontfix"],
                "description": "Explicit closure disposition — read-canonical, NOT inferred from the reason wording. 'retire' or 'wontfix' lands the story Retired (closed && !accepted) — use for superseded / duplicate / won't-fix. 'accept' closes it as shipped. Defaults to 'accept' when omitted; pass 'retire' explicitly so a supersede is never mis-filed as accepted regardless of how the reason is phrased."
            }
        },
        "required": ["id", "reason"]
    });

    Tool::new(
        "close_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
