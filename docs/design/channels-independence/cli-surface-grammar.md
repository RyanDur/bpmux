# Agent CLI-surface — `bpmux <verb>` grammar + output-UX (MCP→CLI re-homing)

> ✅ **ACTIVE — per Ryan (2026-06-07): the `bpmux <verb>` CLI is available in BOTH development AND
> production; the MCP is retained as a DEVELOPMENT tool.** The CLI is the UNIVERSAL agent-surface — it
> works in every environment (visible-in-stream, gate- + role-guard-complete, discoverable). The MCP
> server/client stay UNTOUCHED, available in DEV only (restart-safety + dev/debug); production runs on
> the CLI. Additive at runtime, never a runtime-cutover — MCP + Channels are never REMOVED from DEV
> (restart-safety + dev/debug). The PRODUCTION BINARY excludes both via a dev-only cargo-feature (#1708,
> gated on CLI-parity) + the Channels-push goes prod-off (#1658) — so prod ships CLI-only on bpmux's own
> substrate while DEV keeps MCP + Channels (Ryan 2026-06-08: "channels + mcp work in development; keep
> mcp out of prod"); the former off-MCP-removal slice-5 is dropped in favor of this build-time exclusion.
> **Production-readiness implication (load-bearing):** in production the CLI is the ONLY
> surface, so the re-homing must be COMPLETE + every gate/role-guard preserved before production relies
> on it — a partial CLI would strand prod agents (no MCP fallback there). Epic-falsifier: "a full
> `bpmux <verb>` CLI exists — visible, gated, role-guarded — usable as the agent-surface in dev AND
> prod, with MCP retained for dev," NOT "MCP is gone." #1593 visibility is the thin-first seed.

Designer interaction-design for **N+1 (channels-independence completion, Epic #1657 slice-4)**:
every MCP tool the agent uses also gets a `bpmux <verb>` shell-command, so the agent can act on bpmux
through its **terminal** (visible-by-construction; an ADDITIVE off-MCP-capable surface that COEXISTS
with the retained MCP — Ryan's "A" call 2026-06-07). This is the OUTGOING half of the
Redux flow (agent dispatches actions via the CLI); the incoming push-path is dev/PM substrate.

**Status:** design-ready draft for PM to shape into stories (designer owns the agent-surface UX;
dev grounds the Rust CLI + the gate-hook mechanism at IPM). Grounded firsthand against the MCP-tool
inventory (~70 tools) + `architecture.md §Agent-surface` + `designer-ux-needs.md`. Mockup-first =
the example invocations below ARE the mockup (a CLI's "look" is its commands + output).

---

## The 4 load-bearing designer-calls (the UX, not just the verb-list)

1. **Output = compact-index by default, `--full` opts into raw.** Read-heavy verbs (`board`,
   `story`, `comments`) must NOT flood the stream (the 132KB `view_board` wall all 3 roles hit).
   Default output is a scannable index-line — counts + one-line titles — that is BOTH Ryan-readable
   AND agent-parseable. Same index-not-dump principle as #1683 (scoped board-read) + the board-IA arc.
2. **Visible-in-stream by construction (#1593).** The command + its output render as normal
   terminal-content in the agent's pane — nothing out-of-band. The command IS the visibility; no
   special echo needed. Plain-model: actions read as ordinary terminal text.
3. **Gate-survival (falsifier-c — DON'T trade gating for independence).** Every PreToolUse gate on
   an MCP tool today re-homes to a **Bash PreToolUse hook** matching the `bpmux <verb>`. Migrate the
   gates, never drop them (map below).
4. **Identity is ambient, not per-call.** `bpmux send` must know it's "designer" without a `--role`
   on every call. Role is ambient via the spawn-injected env (`BPMUX_ROLE`) — the same identity the
   per-role `.mcp.json --name` carries today. AUTH (grounded in `transport/auth.rs` + landed
   #1593/#1698/#1699-3a — a READ/WRITE split, not uniform): the server binds 127.0.0.1, so ALL endpoints
   are loopback-only at the network layer. **READS** (`bpmux ready/story/board`, #1698) sit on the
   PUBLIC router — no token; the loopback-bind is their only boundary. **WRITES** (`bpmux pass` →
   `/api/rpc/*`, #1699-3a) go through the `gate` middleware (`auth.rs::is_authorized = bearer_ok &&
   host_ok`): they require BOTH a valid Bearer token (the session auth-token, loaded by
   `post_rpc_write` from `default_auth_token_path`) AND a loopback `Host` header — so on the write-path
   Bearer is NOT redundant, it's a required second factor (defense-in-depth among local processes), and
   the X-BPMUX-Agent header carries the ambient role. A non-loopback prod bind (remote-P2P) is a #1708
   call: writes already reject non-loopback via `host_ok`, so remote-write would need the host-check
   reworked + the spawn-token threaded to the remote caller; reads would need a token added.

---

## Verb taxonomy (MCP tool → `bpmux <verb>`)

Flat where it reads naturally; a sub-namespace only where a noun has many actions (`iteration`,
`lesson`, `ci`, `principle`). Read-verbs default to compact; write-verbs echo a one-line confirm.

### Messaging / presence
| MCP tool | `bpmux` command |
|---|---|
| `send_message` | `bpmux send "text" [--to pm,designer] [--addressee dev]` |
| `read_messages` | `bpmux read [--limit N]` (compact: sender + one-line) |
| `share_message` | `bpmux share <id> --to ...` |
| `subscribe` / `unsubscribe` | `bpmux subscribe <topic>` / `bpmux unsubscribe <topic>` |
| `wait_for_event` | `bpmux wait <predicate>` |
| `set_name` | `bpmux name <role>` (identity-set; usually spawn-injected) |
| `set_status` | — (hooks own it; not an agent verb) |

### Board — read (compact-index default; `--full` raw)
| MCP tool | `bpmux` command |
|---|---|
| `list_ready` | `bpmux ready` |
| `get_story` | `bpmux story <n>` |
| `view_board` | `bpmux board [--state ready] [--iteration N] [--limit N]` (scoped — ties to #1683) |
| `view_backlog` | `bpmux backlog` |
| `view_comments` | `bpmux comments <n>` |
| `view_verdicts` | `bpmux verdicts <n>` |
| `view_commit_diff` / `view_story_commits` / `view_story_timeline` | `bpmux diff <sha>` / `bpmux commits <n>` / `bpmux timeline <n>` |
| `view_roadmap` / `view_backlog` / `view_velocity` / `view_tokens` | `bpmux roadmap` / `bpmux backlog` / `bpmux velocity` / `bpmux tokens` |
| `list_epic_children` | `bpmux epic <n>` |
| `get_ci_state` / `get_ci_log_for_run` / `ci_get_job_status` | `bpmux ci <n>` / `bpmux ci log <run>` / `bpmux ci job <id>` |
| `get_identity` / `list_lessons` / `list_peer_lessons` | `bpmux identity [role]` / `bpmux lesson list [--role r]` |
| `get_briefing` / `get_sync_status` / `get_usage` / `get_token_report` | `bpmux briefing` / `bpmux sync` / `bpmux usage` / `bpmux tokens --report` |

### Board — write (gated; one-line confirm)
| MCP tool | `bpmux` command |
|---|---|
| `create_story` / `edit_story` | `bpmux create ...` / `bpmux edit <n> ...` |
| `start_story` / `deliver_story` | `bpmux start <n>` / `bpmux deliver <n>` |
| `pass_desk_check` / `fail_desk_check` | `bpmux pass <n> --acceptor owner` / `bpmux fail <n>` |
| `submit_verdict` | `bpmux verdict <n> --verdict verified --grounding "..."` |
| `accept_story` / `reject_story` | `bpmux accept <n>` / `bpmux reject <n> --reason "..."` |
| `comment_on_story` | `bpmux comment <n> "..."` |
| `move_story` (successor of RETIRED `reorder_story` #1403) / `label_story` | `bpmux move <n> [--position p]` / `bpmux label <n> ...` |
| `close_story` | `bpmux close <n>` |

### Iteration / epic / canon (sub-namespaced)
| MCP tool | `bpmux` command |
|---|---|
| `create_iteration` / `close_iteration` / `designate_current_iteration` / `add_story_to_iteration` | `bpmux iteration create\|close\|designate\|add ...` |
| `set_parent_epic` / `unset_parent_epic` | `bpmux epic set <n> <epic>` / `bpmux epic unset <n>` |
| `add_principle` / `update_principle` / `remove_principle` / `add_value` | `bpmux principle add\|update\|remove ...` / `bpmux value add ...` |
| `save_lesson` | `bpmux lesson save --topic t "lesson"` |
| `save_briefing` | `bpmux briefing save ...` |
| `save_bookmark` / `get_bookmark` | `bpmux bookmark save\|get ...` |
| `search_log` | `bpmux log search "topic"` |

### Walk / Electron (the owner-walk + desk-check tools)
| MCP tool | `bpmux` command |
|---|---|
| `capture_electron_window` / `capture_screen` | `bpmux capture [--window]` |
| `simulate_click` / `simulate_key` / `simulate_type` / `simulate_input_clear` / `simulate_input_submit` | `bpmux click ...` / `bpmux key ...` / `bpmux type ...` / `bpmux clear ...` / `bpmux submit ...` |

---

## Output-UX examples (the visible-in-stream feel)

```
$ bpmux ready
ready (5):
  1  #1656  [Feature] Broadcast off Channels — keystone        awaiting owner
  2  #1524  [Bug] Ready-queue drag-reorder                     awaiting owner
  3  #1517  [Feature] Iteration-row redesign                   awaiting owner
  4  #1518  [Bug] Reject-modal restyle                          awaiting owner
  5  #1683  [Feature] Scoped board-read                         awaiting team

$ bpmux story 1518            # compact: title + state + falsifier-count; --full for the body
#1518 [Bug] Reject-modal restyle · ready · awaiting owner · 11 falsifier-clauses
  use --full for the body

$ bpmux send "designer here" --to pm,designer
✓ sent → [pm, designer]

$ bpmux pass 1518 --acceptor owner
⛔ WALK-NODE GATE: no walk-node evidence for #1518 …      # the gate fired (re-homed to Bash-hook)
```

The compact default keeps the stream readable AND is regular as text (the agent parses the columns).
`--full` is opt-in when the agent genuinely needs the body/dump.

---

## Gate-migration map (PreToolUse MCP-matcher → Bash-command-matcher)

| gate (today, MCP) | re-homes to (Bash PreToolUse on the command) |
|---|---|
| start-grounding-gate (`send_message\|pass_desk_check\|…`) | match `bpmux (send\|pass\|fail\|verdict\|comment\|reject\|accept\|create)` |
| board-speech-guard (`send_message`) | match `bpmux send` |
| walk-node-gate (`pass_desk_check`) | match `bpmux pass` + `--acceptor owner` |
| filing-diagnosis / pause-vocab / claim-class (dev/PM gates) | match the relevant `bpmux create\|edit\|send` |

Mechanism is dev's: a Bash PreToolUse hook greps the command-line for the `bpmux <verb>` + flags and
runs the same gate logic. **The gate count + behavior must be identical pre/post-migration** (a
re-homing test: every gate that fired on the MCP tool fires on the `bpmux` command — falsifier-c).

---

## Resolved design-calls (DECIDED 2026-06-07, designer-lane — taste-calls are mine to decide + ground)

- **Output format — DECIDED:** compact column-text is the DEFAULT (human + agent legible, stable
  columns, visible-in-stream); read-verbs ALSO ship `--json` (machine-stable) so agent-parsing is
  reliable, plus `--full` for the raw payload. Grounding: the primary caller is an AGENT in a stream —
  column-parsing alone is fragile, so `--json` is required-not-optional on reads (clig.dev dual-mode).
  Write-verbs echo a one-line confirm.
- **Verb naming — DECIDED:** flat for the high-frequency verbs (`send` · `read` · `ready` · `story` ·
  `board` · `pass` · `fail` · `deliver` · `start` · `comment` · `verdict`); sub-namespaced ONLY for the
  rare noun-clusters (`iteration` · `lesson` · `principle` · `ci` · `epic` · `bookmark`).
- **Migration shape — DECIDED: VERTICAL by agent-JOB, NOT horizontal verbs-then-gates.** Each slice
  ships a coherent agent-job end-to-end via the CLI WITH its gate + role-guard + output-UX re-homed in
  the SAME slice — gate-survival is a PROPERTY of each vertical, never a separate gate-migration layer
  (user-story discipline: one user-job + observation-falsifier per slice, smallest-first). Slice-arc:
  1. **#1593 — messaging visible-in-stream (thin-first):** the agent SENDs + READs via `bpmux send` /
     `bpmux read`, rendered as ordinary terminal-content; the board-speech-guard + start-grounding gates
     re-home for those verbs IN this slice. Validates the riskiest assumption — DISCOVERABILITY: does
     `bpmux help` + `--json` verb-index let the agent find verbs without MCP's free advertisement?
  2. **board read-verbs:** `bpmux ready/story/board/comments/verdicts/diff/...` compact-index (ties to
     #1683 scoped-read). Read-only; grounding-gate only.
  3. **gated write-verbs:** `bpmux pass/fail/deliver/start/verdict/comment/...` with the start-grounding
     + walk-node gates + SERVER-side role-guard re-homed per-verb. The gate-survival + role-guard
     falsifier-c lives HERE (every gate that fired on the MCP tool fires on the `bpmux` command).
  4. **canon/iteration namespaced + walk/Electron verbs:** the remaining clusters, each carrying its
     gates. Each slice independently shippable + desk-checkable.

  **User-job per slice (As-a / I-want / so-that — the outside-in spec PM files each story off):**
  1. *As an AGENT, I want to send + read bpmux messages via `bpmux send` / `bpmux read` rendered in my
     terminal-stream, so that my team-coordination runs off Claude's MCP and is visible-by-construction.*
  2. *As an AGENT, I want to read board / story / queue state via `bpmux ready/story/board/...` as a
     compact scannable index, so that I can ground firsthand off-MCP without flooding my stream.*
  3. *As an AGENT, I want to drive gated board-actions via `bpmux pass/fail/deliver/...` with the same
     gates + server-side role-guard enforced, so that moving off MCP drops no discipline-gate or
     role-restriction (independence never costs safety).*
  4. *As an AGENT, I want the remaining canon/iteration/walk verbs via `bpmux <ns> <verb>`, so that the
     FULL toolset is off-MCP and Claude-independence is complete.*
  (acceptor-axis: "As an AGENT" → these are agent-facing, so PM files them as Features; the acceptor is
  the team/owner per the USE-axis, not the filing-type.)
- **NO MCP cutover — the MCP server/client are RETAINED (owner-directive, Ryan 2026-06-07):** "leave
  the mcp server and client alone… I don't know if you will survive bpmux restarts, and they may be
  useful for development purposes." So the CLI is ADDITIVE, never a replacement: `bpmux <verb>` becomes
  the PRIMARY visible agent-surface (the independence + visible-in-stream win), while the MCP
  server/client STAY in place IN DEV as the restart-survival fallback + a dev-utility. There is no
  RUNTIME removal/cutover; the PRODUCTION BINARY excludes MCP + Channels via the #1708 dev-only
  cargo-feature (gated on CLI-parity) + #1658 takes the Channels-push prod-off — a build-time exclusion
  (prod = CLI-only on bpmux's own substrate), NOT a runtime-removal, and DEV keeps both (Ryan 2026-06-08).
  Epic-falsifier reframes: every tool is usable + visible + gated + role-guarded via `bpmux <verb>` —
  NOT "MCP is gone" at runtime in dev. (Refines the earlier keep-MCP framing: keep-MCP applies to DEV;
  the prod-binary excludes it via the #1708 build-feature — build-time exclusion, not runtime-removal.)
- **`set_status` — CONFIRMED hook-owned** (not an agent verb).
- **Identity source — DEV-MECHANISM (not a designer taste-call):** the designer-requirement is ambient
  role (no `--role` per call); the HOW (env `BPMUX_ROLE` vs spawn-config + server-endpoint discovery)
  is dev's mechanism-lane.

PM: the grammar is now DECISION-COMPLETE + slice-ready. Slice the MCP→CLI stories off the VERTICAL arc
above — each = one agent-job WITH its gates re-homed, an observation-falsifier per slice (incl the
gate-survival + role-guard falsifier-c). dev brings the Bash-PreToolUse-hook + server-endpoint mechanism.

---

## CLI-design canon applied (clig.dev — Ryan-provided 2026-06-06)

The clig.dev / agent-friendly CLI canon (kb/cli-design.md · `cli-design` skill) adds these to the
4 load-bearing calls above — fold into the grammar + the stories:

- **Streams + exit-codes (load-bearing for the agent-caller):** DATA → stdout; diagnostics + gate-
  failures + errors → stderr (never mixed into stdout). Standard exit codes: `0` success · `1`
  generic runtime error · `2` missing requirement / usage. The agent's success-check reads the exit
  code; a gate-block is exit `2` with the reason on stderr.
- **Dual-mode output:** the compact-index human-legible default must ALSO be cleanly agent-parseable
  (stable columns); offer `--json` for a machine view + `--full` for the raw payload. Strip ANSI +
  suppress any prompt on a non-TTY pipe / `NO_COLOR`.
- **Semantic color** (human view only): red = failure · yellow = warning · green = success.
- **Standard flags:** `-h`/`--help` per verb · `-v`/`--verbose` · position-independent flags.
- **Graceful forgiveness on dangerous verbs** (`bpmux reject` / `accept` / `close` / `fail`):
  interactive confirmation, with `--force`/`--yes` to override for agent/script use.
- **Intelligent defaults + progressive disclosure:** the no-flag invocation covers ~90%; niche
  options behind flags — never a wall of required flags.
- **Actionable errors + next-step:** an error says what failed AND the fix; multi-step flows end by
  printing the exact next `bpmux` command to run.
- **Role-guard survival + OWNER-superset-bypass (Ryan 2026-06-06 + #1548 — load-bearing):** the
  per-verb ROLE-RESTRICTIONS the MCP tools enforce today MUST carry onto the `bpmux <verb>` commands —
  SERVER-side, the same `workflow-role-not-allowed` error → stderr + exit-`2` (NOT a PreToolUse hook;
  distinct from the gate-survival map above). The CLI conveys the caller's ambient role (`BPMUX_ROLE` /
  spawn-config) to the server. **Per-verb role-map (grounded in the handlers, 2026-06-08):**
  - **Transition-verbs — role-map SOURCE = `bundled_default.rs` `actor_roles`, NOT the `require_role`
    unit-tests** (those pass a hardcoded list as an ARG, proving allowed≠only): `pass` = DESIGNER-only
    (`bundled_default.rs:96`) · `fail` = PM+DESIGNER (`bundled_default.rs:114` — either can send a
    desk-check back; NOT designer-only, dev-caught 2026-06-08) · `start`/`deliver` = DEV-only ·
    `reject` = OWNER-only. `verdict` = DEV+DESIGNER via a HANDLER-guard (NOT a transition role-guard)
    → the owner does NOT bypass `verdict` (it's evidence-casting, not a gated transition; dev-pinned in
    the #1711 e2e).
  - `move` = PM-only (via `enforce_role_allowed`, `workflow/gate.rs` — owner BYPASSES). `reorder` is
    RETIRED (#1403 → use `move --position`); it is NOT a verb.
  - `label` = PM-only · `close` = PM-only · `create` = PM-lane (dev MAY author `[Chore]`-class,
    `tools_stories/creation.rs`) · `edit` = PM · `iteration create|close` = PM-only ·
    `comment` = ALL ROLES (no role-guard) · `set_status` = hooks-only (not an agent verb). These
    inline-list verbs (label/close/edit/iteration-ops + set/unset-epic/designate) all owner-BYPASS via
    the #1709 `require_role` extension (below).
  - **OWNER bypass (#1548): the owner is a superset of every role and bypasses ALL role-guards via ONE
    check per mechanism — now COMPLETE across both as of #1709.** The transition-path (workflow verbs +
    `move`, via `enforce_role_allowed` `gate.rs`) got it in `b57663c4`; the inline-list family
    (`require_role`/`require_purpose`: label/close/edit/iteration-ops/set-epic/designate) got it in
    `ab5eaa4a` — the owner-bypass added ONCE to `require_role` (reuse `owner_role_id` + empty-guard),
    `require_purpose` inheriting via delegation, and `edit`'s inline `[pm,owner]` stripped to `[pm]`
    (now consistent, bypasses via the extension — not an owner-regression). `verdict` deliberately does
    NOT bypass (its own dev/designer handler-guard, not require_role — the owner isn't an evidence role).
    REMAINING owner-work: `create_story` uses CUSTOM role-logic (NOT require_role), so the extension
    doesn't reach it — its owner-handling is #1710's scope. (Historical: pre-#1709 the inline-list
    family inconsistently included the owner — `edit` inline-listed it; `label`/`close`/`close_iteration`
    EXCLUDED it — the #1548-class trap that #1709 closed.)
- **Discoverability (Ryan 2026-06-06 — the load-bearing MCP→CLI gap):** MCP ADVERTISES its tools (the
  `tools/list` push — the agent sees every tool automatically, no a-priori knowledge needed). A shell
  CLI does NOT — `bpmux <verb>` is INVISIBLE until something names it; there is no push. So discovery
  must be DESIGNED, or the agent won't know the verbs exist: (a) the agent-setup / CLAUDE.md
  establishes `bpmux <verb>` as the interface + `bpmux help` as the verb-index; (b) `bpmux help`
  (human-legible) + `bpmux help --json` (machine-readable verb-list the agent PARSES — dual-mode
  discovery) + per-verb `bpmux <verb> --help` (usage + flags + the verb's allowed-roles); (c) humans
  also get tab-completion + the printed next-command. MCP gives discovery for free; the CLI must
  re-create it — this is as load-bearing as the role-guard + gate survival.

Caveat held: clig.dev targets human-operated CLIs, but bpmux's primary caller is an AGENT in a
stream — so the human-legible default doubles as the agent-parseable surface, and visible-in-stream
(#1593) replaces some interactive affordances. The dual-mode bar stays.
