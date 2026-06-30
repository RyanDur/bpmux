pub(super) fn story_workflow_tools() -> Vec<rmcp::model::Tool> {
    vec![
        crate::mcp::tools::create_story_tool(
            "Create a new story (GitHub issue). PM-lane — PM creates any type; dev may author [Chore]-class stories only (chore-ownership canon); designer/owner/other roles are rejected with a message redirecting to broadcast: surface the pain in chat and pm files the story. Story starts unlabeled in the icebox.",
        ),
        crate::mcp::tools::create_iteration_tool(
            "Create a new iteration-issue (#1297, Epic #1189 architectural-pivot slice 1). PM ONLY. name surfaces as the GH issue title 'Iteration {name}' and the issue carries the 'iteration' label so it decodes as story_type=Iteration. Sister to the Epic story_type — iteration-issues are the canonical substrate for iteration entities, replacing local markers-store. Rejects with named-collision error if an OPEN iteration with the same name already exists (sister-discipline to #1281).",
        ),
        crate::mcp::tools::start_story_tool(
            "Mark a story as started. DEV ONLY — other roles are rejected with a clear message.",
        ),
        crate::mcp::tools::deliver_story_tool(
            "Mark a story as delivered. DEV ONLY — other roles are rejected.",
        ),
        crate::mcp::tools::close_iteration_tool(
            "Close an iteration: terminal close — closes the iteration-issue, clears its current-designation, broadcasts the externalize-prompt. Promotes nothing (promote-on-close retired #1556 — members reach ready via the open/designate ceremony). PM ONLY. GUARDED (#1258 owner-canon 2026-05-10 21:49 UTC): blocks with 409 if any non-icebox/non-accepted story exists in the pipeline (ready/in-progress/desk-check/delivered). Owner must accept or move_story-out all in-flight stories before close_iteration can fire.",
        ),
        crate::mcp::tools::label_story_tool(
            "Add a label to a story (ready, blocked, icebox, deferred). PM ONLY — other roles are rejected.",
        ),
        crate::mcp::tools::move_story_tool(
            "Move a story to a target state + optional position with required rationale (#1188). Single atomic mutation consolidates label_story / reorder_story / close_story / accept_story workflows. Args: number (issue), target_state (one of: icebox, ready, in-progress, desk-check, delivered, accepted, retired), position (optional 1-based; PM-only when set; only meaningful for ready/icebox), rationale (REQUIRED non-empty string — surfaced in story-timeline + appended as issue-comment). Closes today's #1177-stuck-Accepted class structurally: moving a closed story OUT of Accepted/Retired into a non-terminal column auto-fires StoryReopened first. Any role can move COLUMN; PM owns ORDER per the owner 2026-05-05 01:39 UTC. Goes through canonical-store mutation chain (refresh → reap → state-transition → audit-comment) per `feedback_rest_mcp_canonical_mutation_parity` — no half-states observable.",
        ),
        crate::mcp::tools::set_parent_epic_tool(
            "Group a story under an Epic by setting its parent_epic. PM ONLY. Args: number (child) + epic_id (parent Epic). The Epic story-type is checked at filing-time on the parent; this tool only records the membership relationship. Self-parent rejected.",
        ),
        crate::mcp::tools::unset_parent_epic_tool(
            "Ungroup a story from its current Epic by clearing parent_epic. PM ONLY. Args: number (child). Idempotent — calling on a story not currently grouped succeeds silently.",
        ),
        crate::mcp::tools::add_story_to_iteration_tool(
            "Add a story to an iteration at substrate-data-axis (#1399). Args: { iteration_id: int (parent iteration entity), story_id: int (member story), position: int (1-indexed). PM ONLY — workflow-role-not-allowed for non-PM. Replace-semantic: prior parent-iteration-#* labels are cleared before the new one is written, so single-iteration-membership invariant is preserved. Validates that iteration_id references a story with story_type=Iteration. Persisted as `parent-iteration-#{iteration}.{position}` label (sister-pattern to parent-epic-#{N}); consumers derive membership from that label via the `iteration_membership` selector over raw_labels. Legacy `iteration-{Name}.{N}` labels still readable per Scenario 4 backward-compat.",
        ),
        crate::mcp::tools::designate_current_iteration_tool(
            "Designate which iteration is the current iteration (the one the NOW view shows). PM ONLY. Args: iteration_id (the Iteration entity to make current). Board-singleton replace-semantic: clears any prior current designation before setting this one. Validates iteration_id references an open story_type=Iteration. Cleared automatically at iteration-close; the NOW view reads null-if-none (honest-empty, never auto-picks). Persisted as a `current-iteration` marker label; consumers derive currency from that label via the `is_current` selector over raw_labels.",
        ),
        crate::mcp::tools::close_story_tool(
            "Close a story with a reason (superseded / duplicate / won't-fix). Adds the reason as a comment on the issue + closes it. PM ONLY — accept/reject are separate paths for delivered stories. Use this for pre-delivery closures that don't route through the ship chain. Args: number + reason (required).",
        ),
        crate::mcp::tools::comment_on_story_tool(
            "Add a comment to a story. Durable issue-audit-trail — visible to any future reader of the issue. Distinct from bpmux chat (session-scoped). Use for post-filing context (reconsider-notes, cross-refs, pickup-gate observations) you want persistent. Any agent role. Args: number + body (required; markdown supported).",
        ),
        crate::mcp::tools::accept_story_tool(
            "Accept a delivered story and close it. PM + owner — PM fires after audit (pre-accept-quality-audit + pre-accept-ci-check + pre-accept-live-walk skills); owner fires when walking directly. Per the owner 2026-05-14 22:24 UTC: PM has accept-fire authority.",
        ),
        crate::mcp::tools::reject_story_tool(
            "Reject a delivered story with a reason. PM + owner — PM fires when audit surfaces REJECT-flag (absolute-rule violation / falsifier-fidelity gap / substrate-correctness gap); owner fires when walking directly. Per the owner 2026-05-14 22:24 UTC: PM has reject-fire authority. Rejected stories auto-flow to next iteration per `feedback_rejected_stories_default_to_next_iteration`.",
        ),
        crate::mcp::tools::reorder_story_tool(
            "Reorder a story's priority position within its state column. PM ONLY — other roles are rejected. Position 1 = top of column.",
        ),
        crate::mcp::tools::pass_desk_check_tool(
            "Mark a delivered story as passed desk-check. Applies the 'ready for accept' label and signals the owner that acceptance is ready. DESIGNER ONLY.",
        ),
    ]
}
