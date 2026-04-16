use rmcp::model::Tool;

pub fn list_peer_lessons_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "peer": {
                "type": "string",
                "description": "Peer name to query lessons from. Omit to list available peers."
            }
        }
    });

    Tool::new(
        "list_peer_lessons",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
