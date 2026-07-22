pub(super) fn observability_tools() -> Vec<rmcp::model::Tool> {
    vec![
        mcp_tools::tools::get_token_report_tool(
            "Get the token-cost report from bpmux's shared token log (#179). Returns JSON with total tokens, row count, per-role breakdown, and the 10 most recent turns (newest first). The data comes from per-harness adapters reading their own LLM-call recordings — you're seeing objective cost data, not self-reported. Same underlying log the `/tokens` TUI view reads, so you see exactly what the owner sees. Empty report if the log is empty or missing.",
        ),
        mcp_tools::tools::get_ci_state_tool(
            "Returns the AGGREGATE CI conclusion for bpmux main branch across ALL workflows triggered by the latest commit (#1287). Returns JSON object: { conclusion: 'success'|'failure'|'in_progress'|'quota_exhausted'|'unknown', last_run_id, branch: 'main', url, head_sha, display_title, workflows: [{name, run_id, conclusion}, ...] }. `conclusion` is the cross-workflow aggregate: 'success' IFF all workflows on the latest commit succeeded; 'failure' IF any workflow failed; 'in_progress' IF any workflow is still running; 'quota_exhausted' IF any is quota-blocked and none red/running. `workflows` carries the per-run breakdown for diagnostic. `last_run_id` is the most-recent run's id (use `get_ci_log_for_run(run_id)` for per-run drill-in). Pull-based — agents call this on-demand for gate-fire decisions (pre-deliver, pre-desk-check, pre-accept). Any role can call.",
        ),
        mcp_tools::tools::get_ci_log_for_run_tool(
            "Returns structured per-test failure records for a specific GH Actions run id (#1174). Args: { run_id: int }. Returns JSON `{ run_id, conclusion, failures: [{ shard_id, test_name, file, line, error_message, retry_count }] }`. Use after `get_ci_state` reports `failure` and you have the `last_run_id` — closes the diagnostic gap that previously forced agents to ask owner for gh-web logs. Cached per run_id with TTL-forever for completed runs (append-only-immutable); in-progress runs return a named error shape `{ kind: 'run_in_progress', conclusion }` rather than empty results. Any role can call.",
        ),
        mcp_tools::tools::ci_get_job_status_tool(
            "Per-job breakdown for a CI workflow run (#1309, Epic #1308 canonical archetype). Args: { run_id: int, verbose?: bool }. WHEN TO USE: after get_ci_state reports failure on a multi-job workflow (e.g. #1253 hybrid CI = test + build-bpmux + playwright-e2e) and you need to identify WHICH job failed before fetching logs via get_ci_log_for_run. RETURNS: canonical response `{ mutation_status: 'ok', data: { run_id, jobs: [{ name, conclusion, duration_ms?, started_at?, completed_at?, html_url }], truncated?: { shown, total, hint } } }`. TOKEN ECONOMY: ≤2KB default; large multi-job runs (>~8 jobs) truncate to the first jobs that fit + emit a `truncated` marker pointing at `verbose: true` for full payload. ERROR CLASSES (canonical shape `{ code, message, next_action }`): `invalid_id` (missing/non-positive run_id), `not_found` (404 from GH), `auth_failed` (401/403; check GITHUB_TOKEN), `rate_limit` (429; wait + retry), `quota_exhausted` (spending limit), `network_error` (upstream/transport). Sister to get_ci_state (aggregate) + get_ci_log_for_run (per-test breakdown). Any role can call.",
        ),
        mcp_tools::tools::rerun_ci_tool(
            "Retrigger a failed GH Actions run via GitHub API direct (#1222). Args: { run_id: int }. Returns JSON `{ status: 'success' | 'failure', run_id, message? }`. Use this when a CI run failed on a transient flake (race-class, runner-unable-to-start, etc) to retrigger only the failed jobs without an empty-commit-as-CI-rerun anti-pattern and without falling back to gh CLI. NOT for code-broken runs — those need a substrate fix per `feedback_no_pre_push_hook_skip_ever`. Sister to get_ci_state. Any role can call.",
        ),
        mcp_tools::tools::user_fn_tool(
            "Invoke a per-role user-defined function (#459). Args: { name: string, args?: object }. Loads `bpmux-<role>/.bpmux/functions/<name>.md` (relative to bridge cwd; role-private — only your own role's directory), parses YAML frontmatter (name, description, type: user_fn, optional args[]), substitutes `{{key}}` slots in the body from args, returns JSON `{ status: 'ok' | 'not_defined' | 'missing_arg', name?, description?, body?, message? }`. Hot-reloads (no caching). Author definitions via Write/Edit (no separate authoring tool). v1 excludes composition / cross-role share / bpmux-side LLM invocation — your own Claude executes the returned body.",
        ),
        mcp_tools::tools::list_user_fns_tool(
            "List your role's defined user-fns (#459). Returns JSON `{ functions: [{ name, description, args? }, ...] }` sorted by name. Role-private — enumerates `bpmux-<role>/.bpmux/functions/*.md` for your own role's directory only.",
        ),
        mcp_tools::tools::get_sync_status_tool(
            "#451: inspect the board-sync orchestrator's current state. Returns JSON `{last_ingest_at, last_push_at, pending_events, last_error}` — the same observability surface the Electron app reads to show a source-down indicator. Use to diagnose why a mutation failed (refresh-before-write reports `source unreachable`), confirm a background ingest ran, or check the pending-push buffer depth. Any role can call.",
        ),
    ]
}
