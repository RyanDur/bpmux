use rmcp::model::Tool;

pub fn user_fn_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Name of the user-fn (matches `<name>` in `bpmux-<role>/.bpmux/functions/<name>.md`)."
            },
            "args": {
                "type": "object",
                "description": "Optional dict of substitution values for {{arg}} slots in the definition body.",
                "additionalProperties": true
            }
        },
        "required": ["name"]
    });

    Tool::new(
        "user_fn",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}

pub fn list_user_fns_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {}
    });

    Tool::new(
        "list_user_fns",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
