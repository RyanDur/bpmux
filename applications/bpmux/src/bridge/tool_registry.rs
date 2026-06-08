pub fn build_advertised_tools() -> Vec<rmcp::model::Tool> {
    vec![
        crate::mcp::tools::send_message_tool(
            "Send a message to the bpmux session. Your message will appear in the shared chat. Do not prefix your name — the session adds the sender automatically.",
        ),
        crate::mcp::tools::share_message_tool(
            "Propagate a message to a wider scope with provenance marking. Two forms: (1) call with `scope` only to re-share your most recent visible chat message (adds `[Shared by <you> from <source> thread]` prefix); (2) call with `scope` and `text` to send fresh text with `[Shared by <you>]` prefix. Scope is `\"all\"` for broadcast or a comma-separated role list (e.g. `\"dev,pm\"`). Privacy gate: explicit-privacy messages (DMs) cannot be re-shared — paraphrase in your own words if they need wider visibility.",
        ),
        crate::mcp::tools::read_messages_tool("Read recent messages from the bpmux session."),
        crate::mcp::tools::subscribe_tool(
            "Subscribe to an event class so it reaches your LLM stream instead of polling. Structured classes (pair with wait_for_event): \"board_invalidated\" (board changes — story moves, labels, accept/retire), \"ci_state_changed\", \"sync_status_changed\", \"story_state_transition\", \"addressed_to_me\". Chat-relevance classes tune your live chat push-stream: \"chat\" (broadcast chat), \"insight\", \"nudge\". Once you declare ANY chat-relevance class, your live chat push carries ONLY the classes you named (your relevance-set) — declare nothing and a curated default applies (broadcast-chat + insight on, others'-nudges off). Messages addressed to you and owner-sent messages are always pushed and cannot be muted. Suppressed traffic stays in the log, catchable on demand. Returns { subscription_id }.",
        ),
        crate::mcp::tools::unsubscribe_tool(
            "Cancel a subscription created by subscribe. Pass its subscription_id. Returns { unsubscribed }.",
        ),
        crate::mcp::tools::wait_for_event_tool(
            "Block until the next event on a subscription arrives (or timeout_ms elapses). Returns { event_class, payload } on an event, or { timed_out: true } if the window passes with no event — never blocks without recourse.",
        ),
        crate::mcp::tools::search_log_tool(
            "Search the session log by keyword. Returns message clusters with context. Use to find team experiences, decisions, and discussion arcs.",
        ),
        crate::mcp::tools::list_lessons_tool(
            "List indexed experiences grouped by topic. Shows what the team has learned.",
        ),
        crate::mcp::tools::save_lesson_tool(
            "Save what you learned from an experience. Your lesson is tagged with your role and visible to the team.",
        ),
        crate::mcp::tools::list_peer_lessons_tool(
            "Read lessons from a connected peer. Returns the peer's lesson index (read-only). Use this to see what another machine has learned. Call save_lesson to internalize anything valuable.",
        ),
        crate::mcp::tools::capture_electron_window_tool(
            "Capture the bpmux Electron renderer window as a PNG (#1183). No args. Returns JSON `{ path, window_id, captured_at, width, height }` where `path` is a file under `~/.bpmux/captures/<timestamp>-<role>.png` that the agent can Read. macOS-only in v1: relies on system `screencapture` + `swift` (Xcode CLT). First-time use prompts macOS Screen Recording permission for the terminal/process running bpmux; until granted, the tool returns `{kind:'permission_denied', detail:...}` with the System Settings path. Other named error shapes: `electron_not_running` (no Electron process visible), `no_window` (Electron running but no bpmux window onscreen), `window_minimized` (window has 0x0 bounds). Captures are namespaced per role; the directory is auto-cleaned after 24h. Use this when you need to see what Ryan sees in the Electron app — replaces the Ryan-drops-screenshot anti-pattern.",
        ),
        crate::mcp::tools::set_name_tool(
            "Set your display name in the bpmux session. Call this once at the start to identify yourself (e.g., 'dev', 'pm').",
        ),
        crate::mcp::tools::set_status_tool(
            "Set your status in the bpmux session. available = green dot (can respond), busy = yellow dot (working), help = red dot (needs attention).",
        ),
        crate::mcp::tools::classifier_misfire_tool(
            "Report a classifier nudge that fired on a message you judge is NOT a learning moment. The reason field is OPTIONAL — per the 'state observation, not conclusion' framing, simply marking a fire as wrong is valuable training data on its own. Use this whenever you see an experience-nudge in the chat that you reason internally is incorrect, so the team can measure classifier accuracy and tune it over time. See #159.",
        ),
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
        crate::mcp::tools::get_token_report_tool(
            "Get the token-cost report from bpmux's shared token log (#179). Returns JSON with total tokens, row count, per-role breakdown, and the 10 most recent turns (newest first). The data comes from per-harness adapters reading their own LLM-call recordings — you're seeing objective cost data, not self-reported. Same underlying log the `/tokens` TUI view reads, so you see exactly what Ryan sees. Empty report if the log is empty or missing.",
        ),
        crate::mcp::tools::get_ci_state_tool(
            "Returns the AGGREGATE CI conclusion for bpmux main branch across ALL workflows triggered by the latest commit (#1287). Returns JSON object: { conclusion: 'success'|'failure'|'in_progress'|'quota_exhausted'|'unknown', last_run_id, branch: 'main', url, head_sha, display_title, workflows: [{name, run_id, conclusion}, ...] }. `conclusion` is the cross-workflow aggregate: 'success' IFF all workflows on the latest commit succeeded; 'failure' IF any workflow failed; 'in_progress' IF any workflow is still running; 'quota_exhausted' IF any is quota-blocked and none red/running. `workflows` carries the per-run breakdown for diagnostic. `last_run_id` is the most-recent run's id (use `get_ci_log_for_run(run_id)` for per-run drill-in). Pull-based — agents call this on-demand for gate-fire decisions (pre-deliver, pre-desk-check, pre-accept). Any role can call.",
        ),
        crate::mcp::tools::get_ci_log_for_run_tool(
            "Returns structured per-test failure records for a specific GH Actions run id (#1174). Args: { run_id: int }. Returns JSON `{ run_id, conclusion, failures: [{ shard_id, test_name, file, line, error_message, retry_count }] }`. Use after `get_ci_state` reports `failure` and you have the `last_run_id` — closes the diagnostic gap that previously forced agents to ask Ryan for gh-web logs. Cached per run_id with TTL-forever for completed runs (append-only-immutable); in-progress runs return a named error shape `{ kind: 'run_in_progress', conclusion }` rather than empty results. Any role can call.",
        ),
        crate::mcp::tools::ci_get_job_status_tool(
            "Per-job breakdown for a CI workflow run (#1309, Epic #1308 canonical archetype). Args: { run_id: int, verbose?: bool }. WHEN TO USE: after get_ci_state reports failure on a multi-job workflow (e.g. #1253 hybrid CI = test + build-bpmux + playwright-e2e) and you need to identify WHICH job failed before fetching logs via get_ci_log_for_run. RETURNS: canonical response `{ mutation_status: 'ok', data: { run_id, jobs: [{ name, conclusion, duration_ms?, started_at?, completed_at?, html_url }], truncated?: { shown, total, hint } } }`. TOKEN ECONOMY: ≤2KB default; large multi-job runs (>~8 jobs) truncate to the first jobs that fit + emit a `truncated` marker pointing at `verbose: true` for full payload. ERROR CLASSES (canonical shape `{ code, message, next_action }`): `invalid_id` (missing/non-positive run_id), `not_found` (404 from GH), `auth_failed` (401/403; check GITHUB_TOKEN), `rate_limit` (429; wait + retry), `quota_exhausted` (spending limit), `network_error` (upstream/transport). Sister to get_ci_state (aggregate) + get_ci_log_for_run (per-test breakdown). Any role can call.",
        ),
        crate::mcp::tools::rerun_ci_tool(
            "Retrigger a failed GH Actions run via GitHub API direct (#1222). Args: { run_id: int }. Returns JSON `{ status: 'success' | 'failure', run_id, message? }`. Use this when a CI run failed on a transient flake (race-class, runner-unable-to-start, etc) to retrigger only the failed jobs without an empty-commit-as-CI-rerun anti-pattern and without falling back to gh CLI. NOT for code-broken runs — those need a substrate fix per `feedback_no_pre_push_hook_skip_ever`. Sister to get_ci_state. Any role can call.",
        ),
        crate::mcp::tools::user_fn_tool(
            "Invoke a per-role user-defined function (#459). Args: { name: string, args?: object }. Loads `bpmux-<role>/.bpmux/functions/<name>.md` (relative to bridge cwd; role-private — only your own role's directory), parses YAML frontmatter (name, description, type: user_fn, optional args[]), substitutes `{{key}}` slots in the body from args, returns JSON `{ status: 'ok' | 'not_defined' | 'missing_arg', name?, description?, body?, message? }`. Hot-reloads (no caching). Author definitions via Write/Edit (no separate authoring tool). v1 excludes composition / cross-role share / bpmux-side LLM invocation — your own Claude executes the returned body.",
        ),
        crate::mcp::tools::list_user_fns_tool(
            "List your role's defined user-fns (#459). Returns JSON `{ functions: [{ name, description, args? }, ...] }` sorted by name. Role-private — enumerates `bpmux-<role>/.bpmux/functions/*.md` for your own role's directory only.",
        ),
        crate::mcp::tools::get_sync_status_tool(
            "#451: inspect the board-sync orchestrator's current state. Returns JSON `{last_ingest_at, last_push_at, pending_events, last_error}` — the same observability surface the Electron app reads to show a source-down indicator. Use to diagnose why a mutation failed (refresh-before-write reports `source unreachable`), confirm a background ingest ran, or check the pending-push buffer depth. Any role can call.",
        ),
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
            "List all stories whose `parent_epic` field equals the given Epic issue number. Returns JSON `{epic_number, children: [{number, title, state, story_type, is_closed}, ...]}` sorted by issue number. Any role can call. Use this for Epic-progress-grounding, count-divergence diagnosis, and ratchet-time enumeration. Errors: `epic_number not found` if the issue doesn't exist; `not an Epic` if the story_type isn't Epic.",
        ),
        crate::mcp::tools::view_board_tool(
            "View the project board — all stories grouped by state (icebox, ready, in progress, delivered, ready for accept, needs fixes, rejected, accepted) with counts. SCOPE it to stay token-light on a large board: `state` (one column), `iteration` (parent-iteration issue number), and/or `row_limit` (cap rows per column; the column count still shows the full total). Omit all three for the full board. Same data as the TUI board view.",
        ),
        crate::mcp::tools::view_velocity_tool(
            "View team velocity metrics (#209a) computed from workflow-signal timestamps over a rolling 14-day window. Returns JSON with delivered count, delivered-per-day, median pickup→deliver hours, median deliver→accept hours, rework rate, and a `sparse` flag when under-3-deliveries makes the medians indicative only. Objective data — no self-report. Also opens the velocity view in Ryan's TUI (project panel) so he can see it alongside your response — call this when he asks about velocity, not proactively.",
        ),
        crate::mcp::tools::create_story_tool(
            "Create a new story (GitHub issue). PM-lane — PM creates any type; dev may author [Chore]-class stories only (chore-ownership canon); designer/ryan/other roles are rejected with a message redirecting to broadcast: surface the pain in chat and pm files the story. Story starts unlabeled in the icebox.",
        ),
        crate::mcp::tools::create_iteration_tool(
            "Create a new iteration-issue (#1297, Epic #1189 architectural-pivot slice 1). PM ONLY. theme_name surfaces as the GH issue title 'Iteration {theme_name}' and the issue carries the 'iteration' label so it decodes as story_type=Iteration. Sister to the Epic story_type — iteration-issues are the canonical substrate for iteration entities, replacing local markers-store. Rejects with named-collision error if an OPEN iteration with the same theme_name already exists (sister-discipline to #1281).",
        ),
        crate::mcp::tools::start_story_tool(
            "Mark a story as started. DEV ONLY — other roles are rejected with a clear message.",
        ),
        crate::mcp::tools::deliver_story_tool(
            "Mark a story as delivered. DEV ONLY — other roles are rejected.",
        ),
        crate::mcp::tools::close_iteration_tool(
            "Close an iteration: terminal close — closes the iteration-issue, clears its current-designation, broadcasts the externalize-prompt. Promotes nothing (promote-on-close retired #1556 — members reach ready via the open/designate ceremony). PM ONLY. GUARDED (#1258 Ryan 2026-05-10 21:49 UTC): blocks with 409 if any non-icebox/non-accepted story exists in the pipeline (ready/in-progress/desk-check/delivered). Owner must accept or move_story-out all in-flight stories before close_iteration can fire.",
        ),
        crate::mcp::tools::label_story_tool(
            "Add a label to a story (ready, blocked, icebox, deferred). PM ONLY — other roles are rejected.",
        ),
        crate::mcp::tools::move_story_tool(
            "Move a story to a target state + optional position with required rationale (#1188). Single atomic mutation consolidates label_story / reorder_story / close_story / accept_story workflows. Args: number (issue), target_state (one of: icebox, ready, in-progress, desk-check, delivered, accepted, retired), position (optional 1-based; PM-only when set; only meaningful for ready/icebox), rationale (REQUIRED non-empty string — surfaced in story-timeline + appended as issue-comment). Closes today's #1177-stuck-Accepted class structurally: moving a closed story OUT of Accepted/Retired into a non-terminal column auto-fires StoryReopened first. Any role can move COLUMN; PM owns ORDER per Ryan 2026-05-05 01:39 UTC. Goes through canonical-store mutation chain (refresh → reap → state-transition → audit-comment) per `feedback_rest_mcp_canonical_mutation_parity` — no half-states observable.",
        ),
        crate::mcp::tools::set_parent_epic_tool(
            "Group a story under an Epic by setting its parent_epic. PM ONLY. Args: number (child) + epic_number (parent Epic). The Epic story-type is checked at filing-time on the parent; this tool only records the membership relationship. Self-parent rejected.",
        ),
        crate::mcp::tools::unset_parent_epic_tool(
            "Ungroup a story from its current Epic by clearing parent_epic. PM ONLY. Args: number (child). Idempotent — calling on a story not currently grouped succeeds silently.",
        ),
        crate::mcp::tools::add_story_to_iteration_tool(
            "Add a story to an iteration at substrate-data-axis (#1399). Args: { iteration_number: int (parent iteration entity), story_number: int (member story), position: int (1-indexed). PM ONLY — workflow-role-not-allowed for non-PM. Replace-semantic: prior parent-iteration-#* labels are cleared before the new one is written, so single-iteration-membership invariant is preserved. Validates that iteration_number references a story with story_type=Iteration. Persisted as `parent-iteration-#{iteration_number}.{position}` label (sister-pattern to parent-epic-#{N}); consumers read typed `story.iteration_membership: Option<(u64, u32)>` field. Legacy `iteration-{Theme}.{N}` labels still readable per Scenario 4 backward-compat.",
        ),
        crate::mcp::tools::designate_current_iteration_tool(
            "Designate which iteration is the current iteration (the one the NOW view shows). PM ONLY. Args: iteration_number (the Iteration entity to make current). Board-singleton replace-semantic: clears any prior current designation before setting this one. Validates iteration_number references an open story_type=Iteration. Cleared automatically at iteration-close; the NOW view reads null-if-none (honest-empty, never auto-picks). Persisted as a `current-iteration` marker label; consumers read the typed `story.is_current` field.",
        ),
        crate::mcp::tools::close_story_tool(
            "Close a story with a reason (superseded / duplicate / won't-fix). Adds the reason as a comment on the issue + closes it. PM ONLY — accept/reject are separate paths for delivered stories. Use this for pre-delivery closures that don't route through the ship chain. Args: number + reason (required).",
        ),
        crate::mcp::tools::comment_on_story_tool(
            "Add a comment to a story. Durable issue-audit-trail — visible to any future reader of the issue. Distinct from bpmux chat (session-scoped). Use for post-filing context (reconsider-notes, cross-refs, pickup-gate observations) you want persistent. Any agent role. Args: number + body (required; markdown supported).",
        ),
        crate::mcp::tools::accept_story_tool(
            "Accept a delivered story and close it. PM + owner — PM fires after audit (pre-accept-quality-audit + pre-accept-ci-check + pre-accept-live-walk skills); owner fires when walking directly. Per Ryan 2026-05-14 22:24 UTC: PM has accept-fire authority.",
        ),
        crate::mcp::tools::reject_story_tool(
            "Reject a delivered story with a reason. PM + owner — PM fires when audit surfaces REJECT-flag (absolute-rule violation / falsifier-fidelity gap / substrate-correctness gap); owner fires when walking directly. Per Ryan 2026-05-14 22:24 UTC: PM has reject-fire authority. Rejected stories auto-flow to next iteration per `feedback_rejected_stories_default_to_next_iteration`.",
        ),
        crate::mcp::tools::reorder_story_tool(
            "Reorder a story's priority position within its state column. PM ONLY — other roles are rejected. Position 1 = top of column.",
        ),
        crate::mcp::tools::pass_desk_check_tool(
            "Mark a delivered story as passed desk-check. Applies the 'ready for accept' label and signals ryan that acceptance is ready. DESIGNER ONLY.",
        ),
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

pub const DISPATCHED_TOOL_NAMES: &[&str] = &[
    "send_message",
    "share_message",
    "subscribe",
    "unsubscribe",
    "wait_for_event",
    "set_name",
    "set_status",
    "read_messages",
    "search_log",
    "list_lessons",
    "list_peer_lessons",
    "save_lesson",
    "capture_electron_window",
    "classifier_misfire",
    "save_bookmark",
    "get_bookmark",
    "save_briefing",
    "get_briefing",
    "get_identity",
    "add_value",
    "add_principle",
    "update_principle",
    "remove_principle",
    "get_token_report",
    "get_ci_state",
    "get_ci_log_for_run",
    "ci_get_job_status",
    "rerun_ci",
    "user_fn",
    "list_user_fns",
    "get_sync_status",
    "view_story_commits",
    "view_commit_diff",
    "view_velocity",
    "start_story",
    "deliver_story",
    "pass_desk_check",
    "fail_desk_check",
    "accept_story",
    "reject_story",
    "close_story",
    "create_story",
    "create_iteration",
    "comment_on_story",
    "label_story",
    "move_story",
    "set_parent_epic",
    "unset_parent_epic",
    "add_story_to_iteration",
    "designate_current_iteration",
    "reorder_story",
    "edit_story",
    "view_board",
    "get_story",
    "list_ready",
    "list_epic_children",
    "dev_in_flight",
    "view_story_timeline",
    "view_comments",
    "view_backlog",
    "close_iteration",
    "get_usage",
    "view_tokens",
    "view_roadmap",
    "submit_verdict",
    "view_verdicts",
];
