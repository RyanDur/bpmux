use rmcp::model::Tool;

pub fn accept_story_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Issue number to accept"
            },
            "would_file_bug": {
                "type": "boolean",
                "description": "Bug-question test: would you file a bug about this story if you accepted it as-is? If true, accept_story blocks and routes you to reject_story instead (per `feedback_bug_question_test_at_desk_check`)."
            },
            "force": {
                "type": "boolean",
                "description": "Override the bug-question block. Requires bypass_reason; reason is recorded in the gate-comment audit trail."
            },
            "bypass_reason": {
                "type": "string",
                "description": "Required when force=true and would_file_bug=true. Explicit justification recorded in gate-comment per `feedback_external_catch_over_self_catch`."
            }
        },
        "required": ["id", "would_file_bug"]
    });
    Tool::new(
        "accept_story",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
