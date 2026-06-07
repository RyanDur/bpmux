# phase-1b lane-separation reshape (#389 — retroactive rework of #374)

Ryan 2026-04-22 ~23:38 UTC on #374: *"I don't understand what it
does. it seems like another visual accident. reject."* Same class as
#373 (illustrated glyphs) — visual feature without clear communication
of purpose. #374 had been pm-accepted in error; #389 captures the
actual reshape.

Current impl (MessageList.tsx:368-412) stacks 4 class-sections
(chat / insight / activity / system) vertically, each as a `<section>`
with `<h3>{classname}</h3>` + `<small>{count}</small>` header. No
iconography, no description, no differentiation between primary and
secondary classes, no toggle tooltip. When lanes mode is on, the chat
class is just one of four equally-weighted stacks — reads as an
accident because chat's primacy in bpmux isn't reflected in the
layout.

Five failure-classes stack here: no visual hierarchy + minimal
headers + no-iconography + no-tooltip + no-first-use-guidance.
Reshape addresses all five as a single coherent spec.

## 3-field spec header

**user-job:** When any agent (or Ryan) turns on lane mode, the
layout tells them at-a-glance that chat remains the primary surface
and insight/activity/system are ambient side-streams they can surface
on demand — WITHOUT consuming the primary reading area's vertical
real-estate, and WITHOUT reading as a random vertical stack of equal-
weight sections. The 5-second comprehension test passes: a first-time
viewer understands what lane mode does by looking at the layout, not
by reading documentation.

**observation-falsifier:** I would reject if:
- Chat is visually equal-weight with insight / activity / system
  (no primacy, as in current #374 impl)
- Lane mode hides chat behind a tab switch or drawer when a secondary
  lane is active — chat is never hidden
- Empty classes render a section header + zero rows (empty-section
  noise, same class as drill-in-reshape's `(none)` flag)
- Toggle has no tooltip explaining lane mode's purpose
- Secondary-lane affordance requires reading a class-name string
  to interpret (count + class-name alone is not self-evident)
- Layout breaks at ≤600px viewport width (Electron resize constraint)

**team-shape-independence:** the lane set is driven by the
`MESSAGE_CLASSES` taxonomy in `useMessageFilter.ts`
(`['chat', 'insight', 'activity', 'system']`). Reshape MUST NOT bake
in a fixed set of 3 secondary classes — iterate over
`MESSAGE_CLASSES.filter(c => c !== 'chat')` so adding a future class
propagates naturally.

## Layout shape

```
┌──────────────────────────────────────────┐
│  top nav                                 │
├──────────────────────────────────────────┤
│                                          │
│                                          │
│   CHAT (primary)                         │
│   message list + input                   │
│   ~70-100% of remaining vertical space   │
│                                          │
│                                          │
├──────────────────────────────────────────┤
│  [ Insights · 3 ] [ Activity · 12 ] [ System · 2 ]  │ ← pill row
├──────────────────────────────────────────┤
│                                          │
│   SECONDARY (only when pill active)      │
│   expanded class's message list          │
│   ~25-30% of remaining vertical space    │
│                                          │
└──────────────────────────────────────────┘
```

Two states:
- **All pills inactive** (default): chat zone expands to fill 100%
  of the post-navigation viewport; pill row is a compact affordance
  strip. Chat input stays at the bottom of the chat zone.
- **One pill active**: chat zone contracts to ~70%; expanded-lane
  zone appears below pill row at ~30%. Input stays with chat.
  Only one secondary lane can be active at a time.

Transition between states is a height animation on the chat zone +
expanded-lane zone (e.g., 180ms ease-out); avoids the instant layout-
snap that reads as a visual glitch.

## Pill row

Each pill represents one secondary class (insight / activity /
system). Only render pills for classes with `count > 0` in the current
scope — empty-class pills get hidden (drill-in-reshape precedent on
empty-noise).

Pill shape (tokenized against existing CSS):

```
[ <icon>  Insights  ·  3 ]
```

- Icon: a small glyph from the app's existing icon vocabulary. No
  emoji (per #373 / drill-in-reshape's avoid-decorative-glyph
  discipline). Use CSS-rendered symbols or a fresh inline-SVG. (NB: the
  `ClassGlyph` component this once named was **retired in #1647** — no
  longer available for reuse.)
- Label: human-readable class name (capitalized): "Insights" /
  "Activity" / "System". Not `insight` raw.
- Dot separator: mid-dot U+00B7 `·` with `margin: 0 var(--space-4)`.
- Count: numeric count of messages in that class in current scope.
- Active state: pill's background fills with class-accent color;
  inactive: subdued-muted.
- Hover state: bg-subtle tint to telegraph interactivity.
- Disabled state: N/A — absent pills simply don't render.

Pill row layout:
- Horizontal flex; `gap: var(--space-6)` between pills.
- Sticks at the break between chat zone and secondary zone (in-flow
  sticky, not floating-overlay).
- Row has thin top + bottom borders using `--border-subtle` to
  visually delineate the affordance strip.

## Toggle affordance

Lane mode's on/off toggle gets a tooltip explaining purpose:

> Lane mode — keeps chat primary; surfaces insights, activity, and
> system events as pills you can expand on demand.

First-use hint: on the first toggle-to-ON in a given session, surface
a non-modal toast below the pill row for ~5 seconds:

> Chat stays here. Tap a pill to surface its messages.

The toast auto-dismisses; one-shot per session (localStorage flag
`bpmux-lane-mode-intro-shown`). No hint if user already dismissed
it in a prior session.

## Interaction model

- Click inactive pill → that pill becomes active; expanded-lane zone
  renders its messages below. If another pill was active, deactivate
  it first (single-active-at-a-time).
- Click active pill → pill deactivates; expanded-lane zone collapses;
  chat zone expands to fill.
- `Esc` key while any pill active → deactivate; collapse.
- Toggle lane-mode off entirely → hide pill row + expanded-lane +
  restore interleaved `<ul>` render (the default non-lane render).

## Implementation shape (for dev)

Suggested component decomposition:

- `MessageLanes.tsx` — owns the split-zone layout (chat zone + pill
  row + expanded zone). Wraps the existing chat-list rendering for
  its primary zone. Renders per-class filter on its secondary zone.
- `LanePill.tsx` — single pill button, exposes `active` + `count`
  + `classname` + `onToggle` props. Self-contained, no cross-pill
  coordination.
- `LanePillRow.tsx` — reads `MESSAGE_CLASSES`, derives counts from
  the visible message-set, emits active-pill state.
- Lane state (`activePill: MessageClass | null`) lives in
  `MessageLanes` via useState. Toggle logic is local.

Extend the existing `lanes?: 'on' | 'off'` prop — when `'on'`, render
`<MessageLanes>` in place of the current stack; when `'off'`, keep
the interleaved default path untouched.

No new refs required. Pill state is `useState`. Transition heights
are CSS-driven (no JS measurement).

## Acceptance criteria

(mapped from #389 story body; sharpened to be measurable)

1. When lane mode is ON and no pill is active, the chat zone fills
   100% of the post-navigation viewport below the pill row. Chat
   input is visible at the bottom of the chat zone.
2. When lane mode is ON and one pill is active, the chat zone is
   reduced to ~70% and the expanded lane zone occupies ~30%. Chat
   remains primary by size.
3. Each pill renders `<icon> <capitalized-class-name> · <count>`.
   Empty classes do not render a pill.
4. Lane-mode toggle has a tooltip matching the text in this spec's
   Toggle-affordance section.
5. First-time lane-mode activation in a session shows a one-shot
   toast ("Chat stays here. Tap a pill to surface its messages.")
   that auto-dismisses after ~5s.
6. Clicking an inactive pill activates it + collapses any other
   active pill. Clicking an active pill collapses it. `Esc` while
   any pill is active collapses.
7. Layout does not break at 600px viewport width (pill row wraps to
   two rows if necessary; chat primacy preserved).
8. Turning lane mode OFF restores the interleaved `<ul>` default
   render.
9. An empty visible-scope (no messages at all) in lane mode renders
   the chat zone's default empty-state and no pill row.

## Out of scope

- Cross-scope pill behavior (pills only reflect counts in the
  current scope/channel; switching scope recomputes).
- User-configurable order of secondary lanes (fixed LANE_ORDER for
  MVP; revisit if user-feedback demands it).
- Per-message-in-secondary reply affordances beyond what MessageList
  already supports (reactions, pin, etc. work via the existing path).
- Mobile / narrow-viewport (<400px) layout — Electron desktop only.
- Animated pill-to-expanded-lane morph (straight height transition
  is enough for MVP; fancy morph can come later if needed).

## Dependency note

- Uses existing `useMessageFilter` + `MESSAGE_CLASSES` taxonomy for
  the lane set.
- Pill icons: CSS-rendered symbols or a fresh inline-SVG. (NB:
  `ClassGlyph` was **retired in #1647** — it's no longer in
  MessageList.tsx, so the prior "reuse the existing component" plan no
  longer applies.)
- CSS follows existing tokens (`--bg-subtle`, `--border-subtle`,
  `--accent-*`, `--space-*`); no new tokens required.
- Should pair cleanly with the absolute-rule discipline — no
  `useRef`, no class-components, no inline `//` comments. All
  interactivity via `useState` + controlled event handlers.

## Retro pointer

#373 + #374 both shipped as "visual" changes without shape-
communication; both were rejected by Ryan as accidents. Precedent:
when filing a visual feature, the spec MUST name what the user is
supposed to understand at a glance (the 5-second test), and the
implementation's layout MUST encode that understanding structurally
(hierarchy, spatial dominance, affordance) — not rely on explanatory
text or iconography decoration.
