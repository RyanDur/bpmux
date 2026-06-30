use rmcp::model::Tool;

pub fn pass_desk_check_tool(description: &'static str) -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "description": "Issue number that passed desk-check"
            },
            "acceptor": {
                "type": "string",
                "enum": ["team", "owner", "dev"],
                "description": "Acceptor-class for the story at desk-check terminal. \"team\" routes to PM-aggregated team-verdict acceptance; \"owner\" routes to owner-USE-axis acceptance; \"dev\" routes to dev-acceptance for [Chore]-class (chore-ownership-triad — chores have no business-value so dev accepts after team-feedback, per owner-canon 2026-05-21 22:59 UTC). Per owner-canon 2026-05-20 03:08:51 UTC + `feedback_designer_voices_acceptor_class_at_pass_desk_check` — illegal-states-unrepresentable at acceptor-axis per Wlaschin Designing-with-Types. Handler atomically applies `awaiting team` / `awaiting owner` / `awaiting dev` label BEFORE gate transition."
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
        "required": ["id", "acceptor", "would_file_bug"]
    });
    Tool::new(
        "pass_desk_check",
        description,
        schema.as_object().cloned().unwrap_or_default(),
    )
}
