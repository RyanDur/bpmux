# Event-Message Vocabulary Audit (#1245)

bpmux fires broadcast/addressed messages on workflow state-transitions
(start_story / deliver_story / pass_desk_check / fail_desk_check /
accept_story / reject_story / move_story / close_iteration). Per Ryan
2026-05-09 18:14 UTC, the pre-#1245 templates embedded **imperative
prompts** ("pick the next story from the ready queue", "fix and
re-deliver", "present to ryan", "check ready count", etc.) inside
state-changed-info — those prompts mislead recipients during
ceremony-pause and other non-pickup contexts.

Per `feedback_pause_vocabulary_requires_named_cause` extended at the
event-emission layer: events should NAME the context they emit in,
not assume the recipient's next-action.

## Audit enumeration

All `build_*_signal` functions in `applications/bpmux/src/bridge/tools_stories.rs`:

| Function | Pre-#1245 text | Class | Post-#1245 text |
|---|---|---|---|
| `build_start_signal` | `#N started. ready queue shrunk — check if restocking needed` | actionable | `#N started.` |
| `build_delivery_signals` (designer) | `#N (title) delivered — desk-check needed` | actionable | `#N (title) delivered.` |
| `build_delivery_signals` (pm) | `#N (title) delivered. queue update: check ready count` | actionable | `#N (title) delivered.` |
| `build_pass_desk_check_signal` | `#N ready for acceptance. passed designer desk-check.` | informational | (kept) |
| `build_pass_desk_check_pm_present_signal` | `#N passed designer desk-check. present to ryan.` | actionable | `#N passed designer desk-check.` |
| `build_fail_desk_check_signal` | `#N failed desk-check: {reason} — needs fixes.` | actionable | `#N failed desk-check: {reason}.` |
| `build_accept_signal` | `#N accepted by {who}. pick the next story from the ready queue` | actionable | `#N accepted by {who}.` |
| `build_reject_signal` | `#N rejected: {reason}. fix and re-deliver` | actionable | `#N rejected: {reason}.` |
| `build_iteration_close_signal` | `[iteration-close] iteration closing — flush unsaved findings now …` | actionable-by-design | (kept; flush IS the user-job) |

## Classification rule

- **Informational**: states the new workflow position. Recipient's
  context determines next-action. (`X started`, `X delivered`,
  `X passed desk-check`, `X failed desk-check: reason`, `X accepted by
  Y`, `X rejected: reason`.)
- **Actionable-by-design**: rare class where the event's user-job IS
  to prompt action (today: `iteration-close` flush-prompt). These
  stay imperative because the prompt is the substrate's purpose.
- **Actionable-by-default (retired)**: imperative phrasing tacked
  onto state-info ("pick the next story", "present to ryan",
  "check ready count"). These were the bug class. Removed.

## Why this matters

- Ceremony-mode pause (per `feedback_no_work_during_acceptance_walk`):
  dev should NOT pick up new stories during owner-walks. But
  pre-#1245 `build_accept_signal` instructs "pick the next story" on
  every accept — bypasses ceremony-pause discipline.
- Multi-role coordination: peers reading a "pick the next story"
  prompt addressed to dev may misinterpret as universal next-action.
- Discipline-by-substrate over discipline-by-reminder: removing the
  imperatives lets each role's context (ceremony-mode flag in the
  future, or just lived-state today) determine action; the message
  just carries fact.

## Acceptance test

`applications/bpmux/src/board/event_messages_audit_tests.rs` — 7 unit
tests assert each `build_*_signal` template:
1. Contains required state-changed information (story number, role,
   reason where applicable)
2. Does NOT contain any phrase from `IMPERATIVE_VOCABULARY` (regex-
   class scan: `pick the next story`, `fix and re-deliver`,
   `present to ryan`, `check ready count`, `check restocking`,
   `needs fixes`, `you should`, `please …`, etc.)

## Out of scope

- Ceremony-mode flag substrate (sister-Feature; this Chore drops the
  imperative-prompts entirely so a flag is not strictly needed —
  state-info messages are context-neutral).
- Per-role customization of event-message content.
- WS state-delta / state-snapshot frame vocabulary (those are
  structured payloads, not human-readable prompts).

## Sister substrate

- `feedback_pause_vocabulary_requires_named_cause` — vocabulary-catch
  hook on `send_message` enforces named-cause when paused. This
  audit is the analogous discipline at the event-emission layer.
- `feedback_state_transitions_distinct_dont_conflate` — events
  should narrate state changes, not collapse into next-action prompts.
