# Agent CLI-surface — `bpmux <verb>` grammar + output-UX (MCP→CLI re-homing)

Designer interaction-design for **N+1 (channels-independence completion, Epic #1657 slice-4)**:
every MCP tool the agent uses → a `bpmux <verb>` shell-command, so the agent acts on bpmux
through its **terminal** (visible-by-construction, off MCP). This is the OUTGOING half of the
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
   on every call. Role+auth come from the spawn-injected config / env (`BPMUX_ROLE`, the server
   endpoint) — the same identity the per-role `.mcp.json --name` carries today.

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
| `move_story` / `reorder_story` / `label_story` | `bpmux move <n> ...` / `bpmux reorder <n> ...` / `bpmux label <n> ...` |
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

## Open design-questions / taste-calls (for the shaping session with PM + Ryan)

- **Output format** — is the compact column-text enough for agents to parse reliably, or do read-verbs
  also need a `--json` mode (machine-stable) with the text as the default human/agent view? (Lean: text
  default that's parseable; `--json` only if an agent-consumer needs it.)
- **Verb naming** — `bpmux ready` vs `bpmux board --state ready`; `bpmux pass` vs `bpmux desk-check pass`.
  Lean flat for the high-frequency verbs (send/ready/story/pass), namespaced for the rare noun-clusters.
- **Identity source** — env (`BPMUX_ROLE`) vs the spawn-injected config; how `bpmux` finds the server.
  (Pairs with the identity-assignment fork in designer-ux-needs.md.)
- **Migration shape** — all-verbs-at-once vs the #1593 thin-first (actions VISIBLE first via the CLI,
  then the full gate-migration). #1593 is the thin-first seed; the full re-homing is the fuller vertical.
- **`set_status`** — stays hook-owned (not an agent verb), confirm.

PM: this is the N+1 design-readiness input — let's shape the MCP→CLI stories off this (thin-first #1593
visibility-slice → the gate-migration slice → the full verb-set), sized at IPM with dev's mechanism-input.

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
- **Role-guard survival (Ryan 2026-06-06 — load-bearing):** the per-verb ROLE-RESTRICTIONS the MCP
  tools enforce today MUST carry onto the `bpmux <verb>` commands — `pass`/`fail`=DESIGNER-only ·
  `create`/`edit`/`move`/`reorder`/`label`/`iteration *`=PM-only · `verdict`=DEV+DESIGNER ·
  `set_status`=hooks-only (not an agent verb). The CLI conveys the caller's ambient role
  (`BPMUX_ROLE` / spawn-config) to the server; the server rejects a role-mismatched verb with the
  same `workflow-role-not-allowed` error → stderr + exit-`2`. This is SERVER-side enforcement (not a
  PreToolUse hook) — distinct from the gate-survival map above, and equally non-droppable: moving off
  MCP must NOT drop the role-gating.
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
