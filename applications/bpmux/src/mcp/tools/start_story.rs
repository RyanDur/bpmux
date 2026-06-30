use rmcp::model::Tool;

pub fn start_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Issue number to mark as started"
            },
            "pickup_paraphrase": {
                "type": "string",
                "description": "(REQUIRED, 2026-04-23 retro) Dev's restatement of the user-job as understood at pickup time. This is the team-visible signal that pickup has fired AND that dev's interpretation matches filing intent. Posted as an issue comment for the audit trail (mirrors deliver_story's verified_observation pattern)."
            },
            "falsifier_reframing": {
                "type": "string",
                "description": "(REQUIRED, 2026-04-23 retro) Dev's interpretation of the observation-falsifier. Either restates the filing-time falsifier verbatim (if accepted as-is) OR declares a sharpening (if dev sees a tighter falsifier at pickup). Forces dev to engage with the rejection criteria before writing code."
            }
        },
        "required": ["id", "pickup_paraphrase", "falsifier_reframing"]
    });
    Tool::new(
        "start_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
