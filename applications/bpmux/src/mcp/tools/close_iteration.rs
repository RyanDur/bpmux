use rmcp::model::Tool;

pub fn close_iteration_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "carry_over": {
                "type": "boolean",
                "description": "When true, unfinished ready-stories carry over to the next iteration instead of dropping back to icebox-top. Default false (drop-back)."
            }
        },
        "required": []
    });
    Tool::new(
        "close_iteration",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
