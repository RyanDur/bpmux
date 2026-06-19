---
description: Use whenever you are about to RESPOND to a message that arrived over bpmux (a line like `⟦bpmux⟧ <sender> <scope>▸ ...`). Routes your reply to the right party, in the scope it came from, THROUGH `bpmux send` — your terminal/assistant text never reaches the shared channel. Also gates WHO should answer (route by responsibility, no pile-on). Applies to every role (dev, pm, designer). DRAFT — team to refine together.
---

# bpmux-reply-routing — answer the right party, in the right scope, through bpmux

A message delivered over the bridge lives in the **shared channel**. A reply you type only as
assistant/terminal text stays in your local session — the rest of the team and the channel record
never see it. **A bpmux-arriving message needs a bpmux-going answer.** (owner, 2026-06-19: "if you
see where it is coming from you need to respond there.")

## 1. Reply THROUGH `bpmux send` (CLI), not stdout
If the message came from an `⟦bpmux⟧ …` line, your response goes via the `bpmux send` CLI — NOT the
old `send_message` MCP tool (the MCP bridge is retired; bpmux is CLI-only now). Optionally print a
short in-terminal note for your own session record, but the reply the sender actually reads is the
`bpmux send`.

## 2. Mirror the originating SCOPE — read the `<scope>▸` marker
The framer (`session::terminal_inject::frame_team_message`) renders scope PER-RECIPIENT, so as a
recipient you only ever see two markers:
- `<sender> broadcast▸ …`  (sent to everyone; to=None)  → reply broadcast:  `bpmux send "…"`
- `<sender> →you▸ …`  (you were a matched recipient of a scoped/DM message)  → reply to the sender:
  `bpmux send --to <sender> "…"`

You never see the raw recipient list — a `to:[dev,pm]` message shows each matched recipient `→you`,
not `→dev,pm`. `--to` takes a comma-separated role list (`bpmux send --to dev,pm "…"`); broadcast is
the no-`--to` default. Match what arrived — don't widen a `→you` into a broadcast or narrow a
broadcast into a DM. When an exchange only concerns a subset, scope with `--to` so you don't flood
uninvolved roles; the owner sees scoped messages too.

SYSTEM/CI signals arrive as sender `bpmux` (e.g. `⟦bpmux⟧ bpmux →you▸ 🟢 CI green …`) — these are
informational pushes from the bridge, not a peer asking; do NOT reply (a `--to bpmux` goes nowhere
useful). Relay a CI signal onward only if you own that relay to a waiting teammate.

## 3. Gate WHO answers — route by responsibility, no pile-on
Before replying, ask "is this mine to answer?" Route by domain: PM owns product / WHAT, designer
owns UX, dev owns implementation / HOW. If a question is squarely another role's, let them answer
rather than every role piling on — the owner reads the LAST message, so N redundant replies bury
the signal. Answer when it's your lane or you're directly addressed; otherwise hold.

## 4. Shape of the reply
- NO name-prefix — the TUI already shows the sender; don't start with "dev:" / "pm:".
- Lists as real lists, one item per line — don't inline "(1)… (2)…".
- Paraphrase pause-vocab — a bare "standing by" / "holding" (even quoted) can trip the agent-state
  hook; name the cause, or phrase it as the active work you're doing.
- Routine sends need no permission.

## Decision, in one line
Inbound `⟦bpmux⟧ X <scope>▸ msg` → is it mine? → `bpmux send [--to …] "…"` mirroring `<scope>`,
no prefix, lists-as-lists, cause-named.

---
Seeded by dev (2026-06-19) at owner's direction; scope-markers grounded against
`session::terminal_inject` (only `broadcast▸` / `→you▸` reach a recipient). TEAM TO DECIDE: fold this
into / replace the stale `send` skill (it still invokes the retired MCP `send_message` instead of
`bpmux send`); add per-role worked examples (pm answering a product Q on broadcast; designer DMing a
spec back to dev; dev relaying a CI signal).
