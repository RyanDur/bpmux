pub(super) fn board_read_tools() -> Vec<rmcp::model::Tool> {
    vec![
        crate::mcp::tools::get_story_tool(
            "Fetch full detail for a single story by issue number. Returns JSON `{number, title, body, state, labels[]}`. Any role can call. USE THIS INSTEAD OF `gh issue view <N>` — raw gh calls bypass the shared chat so teammates can't observe the read (#403 closes that bypass). Required read before pickup-gate paraphrasing a story.",
        ),
        crate::mcp::tools::view_comments_tool(
            "Fetch the full comment audit-trail for a story by issue number. Returns JSON array `[{id, author, body, created_at}, ...]` in chronological order. Any role can call. USE THIS INSTEAD OF `gh issue view <N> --json comments` — surfaces rejection-reason + filing-discussion + retro-scenarios for rework-pickup, desk-check context, and audit-trail re-audit without reaching outside bpmux.",
        ),
        crate::mcp::tools::dev_in_flight_tool(
            "Query whether dev currently has a story in the in-progress state. Returns JSON `{dev_in_flight: bool}` — a minimal yes/no read with no story identifiers so consumers (pm skills, interrupt-discipline checks) can't accidentally leak what dev is working on. Derived from server-side labels (in progress / started / finished) so the answer is authoritative even across bridge restarts. Any role can call.",
        ),
        crate::mcp::tools::list_ready_tool(
            "Enumerate the full ready queue. Returns JSON array of `{number, title, labels[]}` for every story labeled `ready`. Any role can call. USE THIS INSTEAD OF `gh issue list --label ready` — view_backlog surfaces counts + a sample, list_ready returns the complete queue for full pickup-gate visibility.",
        ),
        crate::mcp::tools::list_epic_children_tool(
            "List all stories whose `parent_epic` field equals the given Epic issue number. Returns JSON `{epic_id, children: [{number, title, state, story_type, is_closed}, ...]}` sorted by issue number. Any role can call. Use this for Epic-progress-grounding, count-divergence diagnosis, and ratchet-time enumeration. Errors: `epic_id not found` if the issue doesn't exist; `not an Epic` if the story_type isn't Epic.",
        ),
        crate::mcp::tools::view_board_tool(
            "View the project board — all stories grouped by state (icebox, ready, in progress, delivered, ready for accept, needs fixes, rejected, accepted) with counts. SCOPE it to stay token-light on a large board: `state` (one column), `iteration` (parent-iteration issue number), and/or `row_limit` (cap rows per column; the column count still shows the full total). Omit all three for the full board. Same data as the TUI board view.",
        ),
        crate::mcp::tools::view_velocity_tool(
            "View team delivery metrics computed off the canonical store. Returns JSON with `velocity` (accepted Features in the most recent complete iteration — Tracker semantics, window 1), the per-iteration `iterations` series, `lead_time_median_hours` (first-ready to accept), `acceptance_rejection_rate`, `sub_section_timings` (median hours per board position), and `rework` (re-entry counts per story). Nulls speak for thin data. Objective data — no self-report; call when the team asks about velocity, not proactively.",
        ),
    ]
}
