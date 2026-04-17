use rmcp::model::Tool;

pub fn suggest_leave_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "thread": {
                "type": "string",
                "description": "Thread name the target is in (passed to start_thread)."
            },
            "target_role": {
                "type": "string",
                "description": "Role of the participant you're suggesting step out (e.g. 'dev', 'pm')."
            },
            "reason": {
                "type": "string",
                "description": "Required context — why you're suggesting the target step out (e.g. 'focus on #218'). Soft signal, not a command; target always decides. Empty/whitespace rejected."
            }
        },
        "required": ["thread", "target_role", "reason"]
    });
    Tool::new(
        "suggest_leave",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
