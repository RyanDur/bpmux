use rmcp::model::Tool;

/// #391: the deliver-gate from the 2026-04-22 retro's 4-gate
/// externalization model requires dev to articulate what they
/// DROVE end-to-end at delivery time, not just "tests pass."
/// Required `verified_observation` param forces the dev to write
/// the falsifier-scenario walk result into the delivery record —
/// becomes the diff-point designer checks at desk-check
/// (claim-vs-reality; mismatch → rework). Missing/empty → MCP
/// validation rejection.
pub fn deliver_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to mark as delivered"
            },
            "verified_observation": {
                "type": "string",
                "description": "REQUIRED: Articulate what you DROVE end-to-end in the running app, not 'tests pass'. Describe the falsifier scenario you exercised and what you observed (e.g., 'scrolled up mid-chat, another role posted, my viewport did not drag to bottom — live-verified via Playwright + designer capture'). Becomes the diff-point at desk-check. If you cannot articulate an observation, don't deliver — exercise the user-story path first."
            }
        },
        "required": ["number", "verified_observation"]
    });
    Tool::new(
        "deliver_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
