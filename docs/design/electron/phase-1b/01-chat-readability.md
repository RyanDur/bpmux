# phase-1b chat readability + a11y

Ryan's observation 2026-04-22: "the chat seems hard to distinguish
from a glance. I'm sure there is research into this."

Research confirms. This spec addresses visual differentiation for
sustained scan-use, grounded in the designer KB's
`chat-readability-multi-agent-team` topic and primary-source
distillation (NNGroup 20 chat guidelines, Slack design language,
Craig Abbott web-chat accessibility, WCAG POUR).

## layout (target)

```
┌─ chat ─────────────────────────────────────────┐
│ [all] · 201 hidden                             │
├────────────────────────────────────────────────┤
│ ┌──┐                                           │
│ │D │  designer  14:22  broadcast               │
│ └──┘  message text, wraps naturally…           │
│                                                │
│       [same sender, within threshold]  14:23   │  ← grouped: avatar hidden,
│       continuation message body                │     timestamp dim
│                                                │
│ ┌──┐                                           │
│ │Σ │  pm  14:25  [→ designer]                  │  ← scope badge distinct
│ └──┘  scoped message…                          │
│                                                │
│ ╔══╗                                           │  ← ryan: distinct anchor
│ ║R ║  ryan  14:27  broadcast                   │    shape (filled circle
│ ╚══╝  human's message reads distinct           │    vs AI-role squares)
│                                                │
│ ┌▎─┐                                           │
│ │D │  designer  14:30  [→ ryan]                │  ← addressed-to-me: left
│ └▎─┘  message to you; accented left border     │    border, full opacity
│                                                │
│       [message addressed to another role]      │  ← subtle dim opacity
│                                                │
├────────────────────────────────────────────────┤
│ ryan > type here, Enter to send                │  ← input always reachable
└────────────────────────────────────────────────┘
```

## message row anatomy

**First-of-group** (new sender OR >5 min since last from same sender):
- **avatar** (24–28px square, role-color background, role initial)
  at left margin — preattentive position anchor
- **role name** (colored per role, bold) + **timestamp**
  (viewer-local + tz suffix, dim) + **scope badge** (broadcast / →
  addressee / scope list)
- **body** indented under the avatar column; wraps naturally; code
  blocks via react-markdown as in phase-1a (#329, #334)

**Continuation** (same sender, within threshold):
- no avatar (occupies the space with indent alignment so body lines
  up with first-of-group body)
- no role name (covered by avatar continuity)
- **timestamp still visible** (more-dim than first-of-group); see
  principle 6 in kb topic — async AI-work timing matters per message
- body indented same as first-of-group

## Ryan-as-human-anchor (distinct shape)

Ryan's avatar is a **filled circle** (AI-role avatars are squared-
corner rounded-rect). This is preattentive shape differentiation on
top of role color — the eye finds owner-authority utterances in
a dense log without reading names.

Constraint: do NOT color Ryan's entire message background or add
full-row tint — that over-weights visually and reads as preferential.
Shape + role-color on the avatar is enough.

## addressing + scope

- **addressed-to-me (`addressee == viewer`)**: left-border accent
  (role-brand or accent-brand), full opacity message body. Same as
  phase-1a spec — reinforced.
- **addressed-to-other (`addressee != viewer && addressee != null`)**:
  subtle message-body opacity reduction (~0.82) — reads as "context,
  not my turn." Token: `--msg-fg-muted` (new).
- **unaddressed broadcast**: default opacity.
- **restricted-to (DM or group, `to != null`)**: scope badge shows
  recipient list compactly: `[→ pm]` / `[pm+designer]` / `[→ ryan]`.
  Position: immediately after timestamp in the header row.

## message-class differentiation (MVP: richer filter, not lanes)

Backend data model has `message_type`: chat, insight, activity,
system. Phase-1a MVP filter hides activity + system by default.

Phase-1b moves to a richer filter control:
- **filter pills**: `[all]` `[chat]` `[insight]` `[activity]`
  `[system]` — all / none combinable (click toggles)
- **persisted state**: last-selected filter survives reload
  (localStorage, similar to theme)
- **hidden-count indicator** per-class when filtered:
  "12 activity, 3 system hidden"

Message-class NOT rendered as a lane (deferred to phase-1c). Each
class gets a subtle **left-gutter icon** when visible:
- chat: no icon (default)
- insight: 💡 or `◊` diamond glyph
- activity: • dot (dim)
- system: ⚙ gear

Glyphs at ~11px dim-secondary — present but not loud. Reader can
scan to class without relying on filter.

## timestamps

Unchanged from phase-1a: viewer-local + tz suffix
(`8:35 PM CDT` / `01:35 UTC`). Rendered via `date-fns` format
`MMM d · h:mm a zzz`.

Grouped-continuation messages dim the timestamp further (from
`--fg-dim` to a new `--fg-dimmer` token). Timestamp stays VISIBLE —
do not collapse to first-of-group only.

## grouping threshold

**Primary rule**: same sender, `<= 5 minutes` since last message from
that sender.

**Secondary rule**: group breaks if message class changes
(chat → insight → chat re-shows the avatar even within 5 minutes).

**Secondary rule**: group breaks if addressee changes (broadcast →
[→ designer] re-shows the avatar even within 5 minutes).

bpmux cadence is longer-form than social chat — 5 min threshold
avoids breaking intra-thought bursts. If this reads wrong in live use
(AI agents double-posting with pause), raise to 10 min.

## accessibility substrate (prerequisite — must land)

### Contrast

- `--fg-dim: #808080` → `--fg-dim: #989898` (4.5:1 on bg-base #1a1a1a,
  meets WCAG AA normal)
- `--fg-dimmer: #747474` (new token, 3:1 on bg-base, for
  grouped-continuation timestamps; acceptable for UI-component text
  per WCAG)
- Light theme parallel: `--fg-dim: #6b7280` (4.5:1 on #ffffff)

### Typography

- **Minimum font size 0.75rem (12px)** everywhere. Current 0.6875rem
  (11px) uses lift to 0.75rem. Affected:
  - sidebar chips
  - `.drill-in-chip` labels
  - uppercase meta labels throughout
- **Font loading**: Inter self-hosted via `@font-face` (bundled in
  `applications/electron/src/renderer/src/styles/fonts/`), OR drop
  Inter from the `font-family` stack and rely on `-apple-system`,
  `BlinkMacSystemFont`, `Segoe UI` platform defaults. Pick one;
  current state (listed but not loaded) is worst of both.
- Line-height 1.6 on body retained.

### Semantic + ARIA

- **chat message list**: `<ul role="log" aria-label="Chat messages"
  aria-live="polite">` — per Craig Abbott + WCAG 4.1.3
- **each message row**: `<li>` wrapping `<article>` with
  `aria-labelledby` pointing to the sender name
- **visually-hidden sender prefix** before message body for screen
  readers: `<span class="visually-hidden">{role} said:</span>`
- **DOM order = visual order** (no CSS-column tricks; messages
  chronological top-to-bottom in source)

### Color-only state fallbacks

- **CI glyph**: already has ✓/✗/◌/? glyph alongside color — sufficient
- **drag-reorder drop indicator**: currently color-only accent line.
  Add shape: a 2px dashed border + 4px dot at drop-edge, using
  `--border-strong` not color-dependent.
- **i-search failing state**: currently `--accent-error` tint. Add
  text prefix "failing" already present; also add a `✗` glyph before
  the query text in the status strip for shape redundancy.

### HTML5 spec compliance (#338 follow-up)

- Metrics overlay `<dl class="metrics-grid">` wraps each `<dt>/<dd>`
  in `<div class="metrics-tile">`, not `<section>`. `<section>` is
  not a valid HTML5 grouping child of `<dl>`; screen readers can
  mispair term/definition.

### Keyboard parity

- Filter pills Tab-reachable + Enter-togglable
- Auto-scroll pausable when user scrolls up (already implemented;
  verify aria-live doesn't announce while paused)
- No focus trap in chat — focus moves freely between sidebar, chat
  input, and board panel

## ACs (boundary-level, enable refactor)

**Visual differentiation**:
- Given a message from role R, then the row renders with R's avatar
  (24–28px, role-color background, role initial) at the left margin.
- Given consecutive messages from same role R within 5 minutes, then
  subsequent messages render without avatar (body-indent preserved)
  and timestamp uses `--fg-dimmer`.
- Given a message from ryan, then the avatar is rendered as a filled
  circle (AI roles render as rounded-squares).
- Given a message where `addressee == viewer`, then the row has a
  left-border accent.
- Given a message where `addressee != viewer && addressee != null`,
  then the body renders at 0.82 opacity (--msg-fg-muted).

**Accessibility**:
- Given the chat message container, then it has `role="log"`,
  accessible name, and `aria-live="polite"`.
- Given any message row, then a screen reader announces the sender
  name before the body.
- Given fg-dim on bg-base in the current theme, then the contrast
  ratio measures ≥ 4.5:1 (WCAG AA normal text).
- Given any text element in the rendered UI, then its computed
  `font-size` is ≥ 12px.
- Given the drop indicator during drag-reorder, then it distinguishes
  via shape (dashed border + dot) not color alone.

**Filter**:
- Given the filter toggles `[chat]` + `[insight]` selected, then only
  messages with those message_types render.
- Given a reload of the app, then the last-selected filter state
  survives (persisted via localStorage).

**Playwright coverage** (against real backend, not win.route):
- Test: render chat with 3 consecutive messages from designer, assert
  only first has role name + avatar, continuation messages timestamp
  uses --fg-dimmer.
- Test: ryan's avatar renders as a filled circle (shape
  differentiation test).
- Test: filter-pill toggle survives reload.
- Test: chat message container has role="log" attribute set.

## phase-1c polish deferrals

- lane-separation of message classes (start with richer filter above)
- compact / comfortable density toggle
- avatar-hover shows additional sender-info (role description,
  current status)
- richer message-type glyphs (illustrated rather than unicode)
