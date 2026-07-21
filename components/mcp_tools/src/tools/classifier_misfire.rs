use rmcp::model::Tool;

pub fn classifier_misfire_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "nudge_hash": {
                "type": "string",
                "description": "Short identifier for the nudge that misfired. Can be a hash of the nudge body, a timestamp fragment, or any stable reference the reporter chooses."
            },
            "reason": {
                "type": "string",
                "description": "OPTIONAL free-text reason describing why this fire was wrong. The feedback path does NOT require a reason — per Naur's 'state observation, not conclusion' framing, simply marking the fire as wrong is valuable training data on its own. Provide a reason only when you have a clear one."
            }
        },
        "required": ["nudge_hash"]
    });

    Tool::new(
        "classifier_misfire",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
