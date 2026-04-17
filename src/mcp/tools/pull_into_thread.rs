use rmcp::model::Tool;

pub fn pull_into_thread_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "thread": {
                "type": "string",
                "description": "Thread name you're inviting the target into (passed to start_thread)."
            },
            "target_role": {
                "type": "string",
                "description": "Role you're inviting (e.g. 'designer', 'dev')."
            },
            "reason": {
                "type": "string",
                "description": "Required context — why their perspective is needed (e.g. 'need designer UX lens on naming'). Soft invitation; target decides whether to join."
            }
        },
        "required": ["thread", "target_role", "reason"]
    });
    Tool::new(
        "pull_into_thread",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
