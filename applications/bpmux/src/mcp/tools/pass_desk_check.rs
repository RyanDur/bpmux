use rmcp::model::Tool;

pub fn pass_desk_check_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "number": {
                "type": "integer",
                "description": "Issue number that passed desk-check"
            },
            "would_file_bug": {
                "type": "boolean",
                "description": "Bug-question test: would you file a bug about this story if you passed it as-is? If true, pass_desk_check blocks and routes you to fail_desk_check instead (per `feedback_bug_question_test_at_desk_check`)."
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
        "required": ["number", "would_file_bug"]
    });
    Tool::new(
        "pass_desk_check",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
