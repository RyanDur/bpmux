use rmcp::model::Tool;

pub fn ci_get_job_status_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "run_id": {
                "type": "integer",
                "description": "GH Actions workflow run id (use get_ci_state.last_run_id to find a candidate)."
            },
            "verbose": {
                "type": "boolean",
                "description": "When true, bypasses the ≤2KB default token-economy budget and returns the full per-job record list regardless of size. Default false."
            }
        },
        "required": ["run_id"]
    });

    Tool::new(
        "ci_get_job_status",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
