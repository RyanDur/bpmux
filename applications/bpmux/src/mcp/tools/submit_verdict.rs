use rmcp::model::Tool;

pub fn submit_verdict_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "story_number": {
                "type": "integer",
                "description": "Issue number this verdict is cast on."
            },
            "verdict": {
                "type": "string",
                "description": "Verdict outcome (e.g. 'verified', 'rejected'). Non-empty."
            },
            "substrate_grounding": {
                "type": "string",
                "description": "Required CONTENT cite — the source-grounded reason the verdict was reached (e.g. 'workflow/gate.rs L85 check_ci_green + acceptance.rs L320 handler-call'). Enforced as required arg per #1407 Option C canon (substrate-enforce > discipline). Non-empty after trim."
            }
        },
        "required": ["story_number", "verdict", "substrate_grounding"]
    });
    Tool::new(
        "submit_verdict",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
