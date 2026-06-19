pub(super) fn verdict_and_view_tools() -> Vec<rmcp::model::Tool> {
    vec![
        crate::mcp::tools::submit_verdict_tool(
            "Cast a substrate-grounded verdict on a story so accept_story's evidence-check reads structured data instead of regex-over-freetext chat messages (#1407 Option C). DEV + DESIGNER fire when team-verifiable evidence is required. `substrate_grounding` is a REQUIRED non-empty cite of the source-grounded reason the verdict was reached — CONTENT enforced structurally per META-CANON substrate-enforce > discipline.",
        ),
        crate::mcp::tools::view_verdicts_tool(
            "Read verdict-records for a story (#1428 read-axis sister to #1407 submit_verdict + #1429 notify-pm-on-verdict-cast). Returns JSON array of {story_number, from_role, verdict, substrate_grounding, ts}. Pure-read; no side effects. Any role can call — verdict-records are team-canonical evidence. Use pre-accept-fire or mid-ceremony to observe team-converged-evidence-state via direct read instead of chat-broadcast-cite-of-tool-fire.",
        ),
        crate::mcp::tools::fail_desk_check_tool(
            "Mark a delivered story as failed desk-check with a reason. Applies the 'needs fixes' label and signals dev with the reason. DESIGNER ONLY.",
        ),
        crate::mcp::tools::view_tokens_tool(
            "#225: open the token-usage overlay in Ryan's TUI. Call when conversation warrants seeing token spend (e.g., 'how's our token usage?', 'are we over budget?'). The TUI shows the same data as the /tokens slash command; agents use this MCP tool instead of typing.",
        ),
        crate::mcp::tools::view_roadmap_tool(
            "#226: surface the roadmap in Ryan's TUI. Call when conversation warrants reviewing roadmap state (e.g., 'what's on the roadmap?', 'what are we working on next?'). Same content as the /roadmap slash command; agents use this MCP tool instead of typing.",
        ),
        crate::mcp::tools::view_backlog_tool(
            "#286: surface the board/backlog in Ryan's TUI. Call when conversation warrants reviewing the backlog state (e.g., 'how's the icebox?', 'what's in the ready queue?', 'where are we?'). Returns a JSON summary with counts + ready/delivered samples so the agent can answer inline AND opens the project panel for Ryan.",
        ),
        crate::mcp::tools::get_usage_tool(
            "Get feature usage statistics — invocation counts and last-used timestamps for all commands/features used this session. JSON format.",
        ),
        crate::mcp::tools::view_story_timeline_tool(
            "#418: fetch a story's complete gate-transition timeline — filed, pickup, delivered, desk-check PASS/FAIL (with reason), rejected (with reason), accepted — in one call. Each event carries timestamp + actor + payload. Sourced from structured marker comments (#391/#398/#419 gates) + story metadata. Any role can call. Use instead of scrolling chat or chaining view_comments + view_board to reconstruct a story's journey.",
        ),
        crate::mcp::tools::view_story_commits_tool(
            "#411: fetch the authoritative commit-set that landed a story. Returns JSON array of `{sha, short_sha, subject, author, committed_at}` for every commit on main whose subject references `#<N>` with a word boundary. Safer than `git log --grep` (body matches + number-prefix collisions). Any role can call. Use at desk-check to pin down what to code-review, at rework-pickup to find the prior-delivery baseline.",
        ),
        crate::mcp::tools::view_commit_diff_tool(
            "#412: read a commit's line-level diff inside bpmux. Accepts a sha/ref (or any git ref). Returns the output of `git show --format=fuller --patch <sha>` with an optional `max_bytes` cap (default 100000) — oversized diffs are truncated with a trailing marker citing shown/total bytes so the consumer knows to narrow the view. Any role can call. Use at desk-check to review implementation against spec without context-switching to terminal git-show.",
        ),
        crate::mcp::tools::edit_story_tool(
            "Replace a story's full markdown body. PM + Ryan only — other roles are rejected. Cascade: GitHub issue updated → canonical store refreshed via SourceAdapter::update_body → board-invalidate broadcast so renderers + bridges see the new body without restart. USE THIS INSTEAD OF `gh issue edit <N> --body` — raw gh calls bypass the canonical-store cascade so teammates can't observe the edit (#530). Args: number + body (full replacement; not a diff).",
        ),
    ]
}
