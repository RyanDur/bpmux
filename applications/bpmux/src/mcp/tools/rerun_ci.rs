use rmcp::model::Tool;

pub fn rerun_ci_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "run_id": {
                "type": "integer",
                "description": "GH Actions run id (typically the `last_run_id` from get_ci_state on a failed run that you want to retrigger because the failure is a transient flake)."
            }
        },
        "required": ["run_id"]
    });

    Tool::new(
        "rerun_ci",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
