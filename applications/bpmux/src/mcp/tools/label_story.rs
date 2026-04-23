use rmcp::model::Tool;

// #392: `operation` added to close the ADD-only bypass. Without a
// REMOVE path in MCP, label removal required `gh issue edit
// --remove-label`, which bypassed the bridge's BOARD_INVALIDATE
// broadcast and left peer renderers on a stale board. With this
// extension, both directions go through the bridge and fire the
// invalidate marker on success; no call-out to raw gh stays needed
// for label mutation. Default = "add" preserves backward-compat
// with existing callers that omit `operation`.
pub fn label_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number to label"
            },
            "label": {
                "type": "string",
                "description": "Label to add or remove (e.g., 'ready', 'blocked', 'icebox', 'deferred')"
            },
            "operation": {
                "type": "string",
                "enum": ["add", "remove"],
                "description": "Whether to add or remove the label. Defaults to 'add' for backward-compat with pre-#392 callers. Remove is idempotent: removing a label not attached to the story succeeds silently."
            }
        },
        "required": ["number", "label"]
    });
    Tool::new(
        "label_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
