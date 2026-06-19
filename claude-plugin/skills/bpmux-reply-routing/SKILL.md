---
description: Use whenever you are about to RESPOND to a message that arrived over bpmux (a line like `⟦bpmux⟧ <sender> <scope>▸ ...`). Routes your reply to the right party, in the scope it came from, THROUGH `bpmux send` — your terminal/assistant text never reaches the shared channel. Also gates WHO should answer (route by responsibility, no pile-on). Applies to every role (dev, pm, designer). DRAFT — team to refine together.
---

# bpmux-reply-routing — answer the right party, in the right scope, through bpmux

A message delivered over the bridge lives in the **shared channel**. A reply you type only as
assistant/terminal text stays in your local session — the rest of the team and the channel record
never see it. **A bpmux-arriving message needs a bpmux-going answer.** (Ryan, 2026-06-19: "if you
see where it is coming from you need to respond there.")

## 1. Reply THROUGH `bpmux send` (CLI), not stdout
If the message came from an `⟦bpmux⟧ …` line, your response goes via the `bpmux send` CLI — NOT the
old `send_message` MCP tool (the MCP bridge is retired; bpmux is CLI-only now). Optionally print a
short in-terminal note for your own session record, but the reply the sender actually reads is the
`bpmux send`.

## 2. Mirror the originating SCOPE — read the `<scope>▸` marker
The inbound line self-labels its scope just before `▸`:
- `<sender> broadcast▸ …`  → reply broadcast:                  `bpmux send "…"`
- `<sender> →you▸ …`  (a DM to you)  → reply to that sender:   `bpmux send --to <sender> "…"`
- a role/group-scoped message  → reply to that same set:       `bpmux send --to <role1>,<role2> "…"`

Match what arrived — don't widen a DM into a broadcast or narrow a broadcast into a DM. When an
exchange only concerns a subset (e.g. dev↔pm, designer not needed), scope with `--to` so you don't
flood uninvolved roles; the owner still sees scoped messages.

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
DRAFT seeded by dev (2026-06-19) at Ryan's direction ("you all need it; work on it together").
Open questions for the team: should this fold into / replace the stale `send` skill (which still
references the retired MCP `send_message`)? what scope-markers does the TUI actually emit beyond
`broadcast▸` / `→you▸`? add examples per role.
