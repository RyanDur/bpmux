# phase-1b Electron specs

Designer specs for polish + readability on top of the phase-1a MVP.
Scope = things Ryan + agents need for sustained daily use, now that
the MVP flow works.

## specs in this directory

- `01-chat-readability.md` — visual differentiation + grouping + a11y
  substrate for the chat surface, grounded in primary-source research
- `02-transitions-animations.md` — motion substrate + high-signal
  transitions + micro-interactions + close-direction
- `03-blocker-visual.md` — block-state visual indicator in the board
  DELIVERED column; extends state-aware badge substrate with a fourth
  BadgeKind. Authored under the retro-committed 3-field spec template
  (user-job + observation-falsifier + team-shape-independence).
- `06-drill-in-reshape.md` — drill-in visual-hierarchy reshape
  (retro 2026-04-24). Hero state-badge + hide-empty sections + filter
  lifecycle-labels + chip-rendering-fix. Unblocks #366/#367/#371
  rework class. Addresses Ryan's "designer did not look at these"
  observation.
- `05-team-roster-substrate.md` — #402 team-roster substrate. Sidebar
  membership authority switches from observed-messages to
  bridge-connected-agents. Closes the "keep losing groups and directs"
  class (Ryan 2026-04-23) + paired with #393 AC1 / #386 / #399 as the
  substrate-level membership source.
- `04-channel-scope-visual.md` — channel-scope visual treatment.
  Channel selection shifts emphasis (0.82 dim on non-primary messages,
  unified with addressed-to-other tier) but never hides. Nothing-
  hidden-from-owner principle made explicit in the stream-rendering
  substrate.

## phase-1b scope (at-a-glance-scan + accessibility)

Phase-1a established the mechanism (chat, board, drill-in, accept/
reject, CI glyph). Phase-1b raises the at-a-glance scan quality so
Ryan can sustain daily use without cognitive fatigue:

- **chat readability**: avatar-anchored message rows, consecutive-
  sender grouping, persistent timestamp per message, stronger
  addressee + scope signals, a11y substrate fixes
- **accessibility floor**: WCAG AA contrast lift (fg-dim), minimum
  font-size 12px, Inter font loaded or dropped, shape/text fallback
  for color-only state signals, HTML5 spec compliance (metrics dl/div)

## explicit phase-1c deferrals

Intentionally NOT in phase-1b, either because the primitive isn't
justified yet OR research says the cognitive load isn't worth it:

- threading / reply trees
- lane-separation of message classes (chat/insight/activity lanes in
  separate columns) — start with better filter, promote to lanes if
  density still pains users
- compact / comfortable density toggle
- typing indicators
- read-receipts
- reactions + pins + unfurls (filed as #340, #342, #343 separately)
- search UI spans story bodies + comments (#344)
- live comment refresh via WS push (#345)

## design grounding

See designer KB:
- `bpmux-designer/kb/chat-readability-multi-agent-team.md` — derived
  principles for bpmux AI+human team chat
- `bpmux-designer/kb/references/reference_chat_ui_research.md` —
  primary-source distillation (NNGroup, Slack, WCAG, accessibility
  research)

## quality bar (inherited from phase-1a)

- each feature done-done: works + has a Playwright test against real
  backend (no win.route mocks) before acceptance
- collaboration-continuity invariant preserved (chat input always
  reachable alongside any view)
- CSS tokens + nesting discipline; no per-component color hard-codes
