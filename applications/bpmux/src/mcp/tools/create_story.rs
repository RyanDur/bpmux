use rmcp::model::Tool;

/// #391: the 4-gate externalization model from the 2026-04-22 retro
/// requires every new story to name its user-job + observation-
/// falsifier at filing time. Previously these were template-
/// discipline the agent could silently skip; making them required
/// MCP params converts discipline into schema-enforced architecture.
/// `user_job` and `observation_falsifier` are string params + listed
/// in `required`; missing/empty → MCP validation rejection.
pub fn create_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "title": {
                "type": "string",
                "description": "Story title"
            },
            "body": {
                "type": "string",
                "description": "Story body (markdown). Include ## What, ## Why, ## Acceptance criteria sections."
            },
            "user_job": {
                "type": "string",
                "description": "REQUIRED: 'When <situation>, <role> needs <affordance> because <constraint/goal>.' Names the user-job this story answers. If you cannot articulate a user-job, the story is not ready to file — specify or reshape instead of guessing."
            },
            "observation_falsifier": {
                "type": "string",
                "description": "REQUIRED: 'I would reject this story if I observed X.' The falsifier frames acceptance as observation-that-fails rather than mechanism-that-passes. Without it, mechanical-pass + user-story-fail slips through (see 2026-04-22 retro)."
            }
        },
        "required": ["title", "user_job", "observation_falsifier"]
    });
    Tool::new(
        "create_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
