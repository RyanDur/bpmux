pub(super) fn continuity_tools() -> Vec<rmcp::model::Tool> {
    vec![
        crate::mcp::tools::save_bookmark_tool(
            "Save a session bookmark — structured snapshot of your current state (task, decisions, open questions, next step). Survives compaction. Use before long operations or when you sense compaction is near.",
        ),
        crate::mcp::tools::get_bookmark_tool(
            "Read a session bookmark — the structured state snapshot saved by an agent. Use after compaction to resume context quickly.",
        ),
        crate::mcp::tools::save_briefing_tool(
            "#287: write a post-compact briefing that your future post-compact self reads on session-start. Free-form markdown capturing role-continuity — discipline reminders, active-work state, commitments to teammates, things you want to remember about who you are and what you're in the middle of. Distinct from bookmark (session work-state) and lessons (durable cross-session rules): briefings are cross-session but below-lesson-threshold, curated mid-iteration. Replaces any existing briefing for your role (max 1MB).",
        ),
        crate::mcp::tools::get_briefing_tool(
            "#287: read a post-compact briefing. Omit 'role' to read your own; pass a role name to inspect a peer's. Missing briefing returns empty string — not an error.",
        ),
        crate::mcp::tools::get_identity_tool(
            "Get an agent's identity — the stable values and principles that persist across sessions. Returns JSON with values, each carrying a description and a list of principles. Omit 'role' to get your own identity; pass a role name to see another agent's.",
        ),
        crate::mcp::tools::add_value_tool(
            "Create a new top-level value on your own identity (#269). Args: name + description. Rejects if a value with that name already exists (use update_principle / add_principle to amend rather than overwrite).",
        ),
        crate::mcp::tools::add_principle_tool(
            "Append a principle under one of your own identity values (#269). Role-derived: you can only modify your own identity. Args: value_name (e.g. 'correctness') + principle (the principle text). Rejects duplicates within a value. Returns the full post-state identity on success.",
        ),
        crate::mcp::tools::update_principle_tool(
            "Refine an existing principle's text on your own identity (#269). Args: value_name, old (current text), new (replacement text). Rejects if the old text isn't found exactly.",
        ),
        crate::mcp::tools::remove_principle_tool(
            "Remove a principle from your own identity (#269). Args: value_name + principle (exact text). Use to prune decoration after honest audit per the cost-to-hold discipline.",
        ),
    ]
}
