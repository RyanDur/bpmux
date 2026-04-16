use rmcp::model::Tool;

pub fn share_message_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "scope": {
                "type": "string",
                "description": "Target scope: \"all\" to broadcast, or a comma-separated role list (e.g. \"dev,pm\"). Wider than the source scope by convention — /share exists to widen visibility."
            },
            "text": {
                "type": "string",
                "description": "Optional fresh text. When present: Form 2 (fresh share) — send `[Shared by <you>] <text>` to `scope`. When absent: Form 1 (re-share) — propagate your most recent visible chat message to `scope` with provenance marker `[Shared by <you> from <source> thread]`."
            }
        },
        "required": ["scope"]
    });

    Tool::new(
        "share_message",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
