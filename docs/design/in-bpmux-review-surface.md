# In-bpmux review surface — designer shows Ryan things without leaving bpmux

Owner-RAISED 2026-06-06 (Ryan: "we need a place in bpmux for you to show me things… I don't
want to have to leave bpmux"). Designer interaction-design **brainstorm-seed** (deliberately THIN
— Lean concept-validation, go-wide-then-converge per the Tanzu design-guide; NOT a converged spec).
PM composes the slices; dev tech-scopes; we converge in the HMW brainstorm. Mockup-thin follows the
brainstorm.

## Outcome (not output)

Ryan reviews + reacts to the team's artifacts (mockups, captures, before/after, taste-options)
**entirely inside bpmux** — never leaving for a browser. Part of the everything-in-bpmux arc (the
same thesis that put agent-terminals in bpmux).

**Impact (data-driven):** Ryan leaves bpmux on EVERY mockup-cycle today (e.g. the #1518 mockup — I
handed a file path, he opens it out-of-band). The mockup-first loop (designer presents → owner
taste-picks) is the team's core design-review beat; it currently breaks the "one place" boundary
every time.

## The loop (the user-job)

1. **PRESENT** — the team (esp. designer) shows Ryan an artifact, story-tied.
2. **RENDER** — it appears in an in-bpmux surface Ryan is already looking at.
3. **REACT** — Ryan approves / picks-an-option / comments, in-app.
4. **FLOW BACK** — his reaction reaches the team via the **conversation-layer** (chat/bridge),
   NOT raw into a terminal-session — honoring Ryan's boundary: terminal I/O stays local; sharing to
   the team is explicit + via the conversation-layer.

## Solution-spectrum (go-wide — for the brainstorm, NOT converged)

Ordered thinnest→richest. The design-call is which the riskiest-assumption needs.

| | direction | render | react | cost |
|---|---|---|---|---|
| A | **Capture-to-chat** | designer captures the artifact as an IMAGE → renders inline in the chat stream | Ryan replies in chat (text / 👍 / "option 2") | **cheapest** — no webview; reuses capture + chat |
| B | **Inline artifact-card** | a card/thumbnail in chat, click-to-expand | reply in chat, or card-level controls | light |
| C | **Preview pane (webview)** | Electron renders the HTML mockup NATIVELY in a pane — rich + interactive (Ryan clicks the live mockup) | in-pane controls (approve / pick-option) | heaviest — webview + IPC |
| D | **Review surface** | a dedicated panel listing artifacts-awaiting-review, story-bound | per-artifact react controls | heavy — new surface |
| E | **Near the terminal panes** | render in/beside the terminal-pane area (reuse that infra) | — | medium |

## Riskiest assumption + thin-first recommendation (Lean)

**Leap-of-faith to validate:** *is image + chat-reply enough for Ryan's review, or does taste-pick
genuinely need a rich, interactive (clickable) surface?* Everything else (where the pane sits, the
verb syntax) is cheap to change; this one determines whether we invest in the webview at all.

**Thinnest validating slice = A (capture-to-chat):** designer captures → image in chat → Ryan
replies. Proves the present→react loop end-to-end with almost no build, and tells us whether static
+ reply suffices. If Ryan finds he needs to *interact* with the mockup (hover states, click an
option live), THAT failure is the signal to invest in C (webview pane). Build A first; let the real
use tell us if C is warranted — don't pre-build the webview on an assumption.

(This refines PM's drafted Slice-A "webview pane": the guide says validate the riskiest assumption
cheapest first. A is the validation; C/D is the iteration if A proves insufficient.)

## Interaction-design notes (converge in the brainstorm)

- **PRESENT verb** rides the CLI-surface (N+1): `bpmux present <artifact> --story <n>` — the outgoing
  half I specced in `cli-surface-grammar.md`. Visible-in-stream (#1593) + story-tied by construction.
- **Artifact kinds:** image-capture (A/B), HTML mockup (C webview), side-by-side (a presented pair).
- **REACT controls:** the minimal honest set — approve · pick-option-N (for taste-options like
  #1518's 4 picks) · comment. Output of "pick-option-2" must be unambiguous back to the team.
- **Story-tie + lifecycle:** the artifact binds to its story; retires-when-the-story-ships (mockups
  are story-bound). Bakes into the SHOW slice's acceptance, not a separate slice.
- **Boundary:** the react flows back via the conversation-layer, never auto-injected into a terminal.

## Open questions for the HMW + dev tech-scope

- **Webview feasibility/cost in Electron** (dev) — is rendering an HTML mockup in a pane cheap or a
  real lift? This sizes whether C is a near or far iteration.
- **Image-vs-interactive** (the riskiest assumption) — settle via A first, or does Ryan already know
  he needs interactive?
- **Surface placement** — chat-inline (A/B) vs a dedicated pane (C/D) vs near-terminals (E); how it
  sits with the agent-terminal panes.
- **Capture source** — the existing `capture_electron_window` captures the live app; presenting a
  *mockup* needs the mockup rendered first (a headless render-to-image?) — dev-mechanism.

## Slices (PM's lane — noted for trace)

PM's drafted structure: **SHOW** (present→render, story-tied) → **REACT** (approve/pick/comment→
flow-back). Rides the MCP→CLI re-homing (N+1) since `bpmux present` is a CLI verb. IPM-ready when
this design-doc + dev's tech-scope land + we brainstorm it.
