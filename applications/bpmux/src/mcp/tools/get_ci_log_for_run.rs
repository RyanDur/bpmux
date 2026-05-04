use rmcp::model::Tool;

pub fn get_ci_log_for_run_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "run_id": {
                "type": "integer",
                "description": "GH Actions run id (the `last_run_id` from get_ci_state on a failed run; or any historical run id known to be failed)."
            }
        },
        "required": ["run_id"]
    });

    Tool::new(
        "get_ci_log_for_run",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
