use rmcp::model::Tool;

pub fn view_commit_diff_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "sha": {
                "type": "string",
                "description": "Commit SHA (full or short) or any valid git ref"
            },
            "max_bytes": {
                "type": "integer",
                "description": "Optional cap on returned diff size. Defaults to 100_000. Oversized diffs are truncated with a trailing marker citing shown/total bytes."
            }
        },
        "required": ["sha"]
    });
    Tool::new(
        "view_commit_diff",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
