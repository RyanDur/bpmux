use rmcp::model::Tool;

// #403: read-side bypass closure. Pre-#403 agents read story bodies
// via raw `gh issue view <N>`; the bridge couldn't observe the call,
// so pickup-gate paraphrases referenced story content that other
// roles never saw come across the wire. Now goes through MCP so
// every read is visible in the shared chat like every write.
pub fn get_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to fetch"
            }
        },
        "required": ["number"]
    });
    Tool::new(
        "get_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
