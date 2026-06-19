use rmcp::model::Tool;

pub fn move_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to move"
            },
            "target_state": {
                "type": "string",
                "enum": [
                    "icebox",
                    "ready",
                    "in-progress",
                    "desk-check",
                    "delivered",
                    "accepted",
                    "retired"
                ],
                "description": "Destination column / terminal state. 'retired' closes the GH issue with the rationale as reason; 'accepted' transitions and closes; the rest are workflow-state transitions."
            },
            "position": {
                "type": ["integer", "null"],
                "minimum": 1,
                "description": "1-based absolute position within the destination column. Only meaningful when target_state is 'ready' or 'icebox'; PM-only when present (PM organizes those queues per the owner 2026-05-05 01:39 UTC). Null/absent is fine for any non-organizing column."
            },
            "rationale": {
                "type": "string",
                "minLength": 1,
                "description": "REQUIRED. Why this move? Surfaced in the story-timeline + appended as an issue-comment so audit-trail is observable. Blank rejected. Examples: 'rework after desk-check', 'next-iteration promote', 'retire — superseded by #1186'."
            }
        },
        "required": ["number", "target_state", "rationale"]
    });
    Tool::new(
        "move_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
