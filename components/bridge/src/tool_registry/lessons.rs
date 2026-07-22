pub(super) fn lessons_tools() -> Vec<rmcp::model::Tool> {
    vec![
        mcp_tools::tools::list_lessons_tool(
            "List indexed experiences grouped by topic. Shows what the team has learned.",
        ),
        mcp_tools::tools::save_lesson_tool(
            "Save what you learned from an experience. Your lesson is tagged with your role and visible to the team.",
        ),
        mcp_tools::tools::list_peer_lessons_tool(
            "Read lessons from a connected peer. Returns the peer's lesson index (read-only). Use this to see what another machine has learned. Call save_lesson to internalize anything valuable.",
        ),
    ]
}
