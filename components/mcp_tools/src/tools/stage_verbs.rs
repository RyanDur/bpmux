use rmcp::model::Tool;

pub fn pass_stage_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Issue number whose current middle stage passed its judgment"
            },
            "verified_observation": {
                "type": "string",
                "description": "What you verified at this stage in the running app — becomes the next stage's diff-point"
            }
        },
        "required": ["id", "verified_observation"]
    });
    Tool::new(
        "pass_stage",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn fail_stage_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Issue number whose current middle stage failed its judgment"
            },
            "reason": {
                "type": "string",
                "description": "What failed, cited concretely (min 16 chars) — travels with the story as its rework context"
            }
        },
        "required": ["id", "reason"]
    });
    Tool::new(
        "fail_stage",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
