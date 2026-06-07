# phase-1b transitions + animations

Ryan 2026-04-22: "some transitions and animations of opening, closing
and appearing of things might be nice." Plus the observation that
board groups already use native `<details>`/`<summary>` which are
natively animatable via modern CSS.

Intent: motion that *confirms* state change without *distracting*.
Snap-cuts feel unfinished; over-animated UIs feel slow. Target:
100–250ms, ease-out for appearing, ease-in for disappearing.

## discipline anchors

- **motion is a signal, not decoration**. Every animation answers a
  specific question: "did my click take effect?" / "where did that
  panel go?" / "is this the same element in a new state?". If no
  question is answered, no motion.
- **`prefers-reduced-motion: reduce` is load-bearing** (WCAG 2.3.3).
  Not a nice-to-have. Users with vestibular disorders, migraines, or
  a preference for stillness set this at the OS level. Every motion
  in the spec degrades to duration=0 + no-transform under that query.
- **don't re-introduce sub-12px content or sub-AA contrast via
  animation**. Mid-transition values still render; if they dip below
  the a11y substrate (#347 AC1/AC2), users mid-frame see failing
  state. Contrast + size floors hold through the whole transition.

## decomposition (pm's 2–3 slice guidance)

Proposed as three thin-vertical-slices matching the chat-readability
pattern:

1. **Motion substrate** (prerequisite Feature) — tokens,
   prefers-reduced-motion query, base utility classes. Architectural
   floor. Parallels #347 for a11y.
2. **High-signal transitions** (Feature) — drill-in, board toggle,
   search/metrics overlay, reject-form, theme color interpolation,
   `<details>` open/close. Significant surface state changes.
3. **Micro-interactions** (Feature) — filter pills, button
   hover/active, avatar appear, message new-arrival. Dozens of small
   polish wins; batch as one Feature rather than per-component.

## slice 1 — motion substrate (prerequisite)

### tokens (additions to `styles/tokens.css`)

```css
:root {
  /* Timing — picks align with Nielsen/Shneiderman interaction-
   * timing research. < 100ms reads as instant; 100–250ms reads as
   * responsive confirmation; > 250ms starts to feel slow unless the
   * motion is deliberate (overlay enter, drill-in reveal). */
  --motion-duration-instant: 80ms;
  --motion-duration-fast: 150ms;
  --motion-duration-medium: 250ms;

  /* Easing — ease-out feels "arriving" (fast to slow), ease-in
   * feels "leaving" (slow to fast). ease-in-out for bi-directional
   * position changes. Custom cubic-beziers keep the curves snappier
   * than CSS defaults which feel soft for UI. */
  --motion-ease-out: cubic-bezier(0.2, 0.8, 0.2, 1);
  --motion-ease-in: cubic-bezier(0.4, 0, 1, 1);
  --motion-ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
}

/* Load-bearing a11y: honor OS-level motion preference. Zero all
 * durations — keep opacity-only transitions since they don't trigger
 * the vestibular class of issues that position/transform animations
 * cause. If an animation MUST remove entirely under reduced-motion
 * (e.g., slide-in), the consumer sets duration to 0 via the token. */
@media (prefers-reduced-motion: reduce) {
  :root {
    --motion-duration-instant: 0ms;
    --motion-duration-fast: 0ms;
    --motion-duration-medium: 0ms;
  }
}
```

### utility: `.motion-safe-transform`

For animations using transform/translate (the class
prefers-reduced-motion is meant to suppress), wrap the transform
rule in `@media (prefers-reduced-motion: no-preference)` so the
transform doesn't apply at all under reduced-motion, while the
opacity change still does. Pattern:

```css
.thing {
  opacity: 0;
  transition: opacity var(--motion-duration-medium) var(--motion-ease-out);
}
@media (prefers-reduced-motion: no-preference) {
  .thing {
    transform: translateY(8px);
    transition:
      opacity var(--motion-duration-medium) var(--motion-ease-out),
      transform var(--motion-duration-medium) var(--motion-ease-out);
  }
}
.thing.is-enter {
  opacity: 1;
  transform: translateY(0);
}
```

### ACs (slice 1)

- Given `@media (prefers-reduced-motion: reduce)` is active, then
  computed style of `--motion-duration-fast` === `0ms`.
- Given an element has transition on `opacity` via motion tokens,
  then under reduced-motion, the transition duration computes to 0.
- Given the three timing tokens, then their values are 80ms / 150ms /
  250ms respectively (no magic-number drift).

### Playwright (slice 1)

- Emulate reduced-motion: `await page.emulateMedia({ reducedMotion: 'reduce' })` — Playwright has first-class support. Assert computed `--motion-duration-fast` === `'0ms'` after emulation.
- Without emulation (default), same token resolves to `150ms`.

## slice 2 — high-signal transitions

Each transition answers a state-change question. Durations cited
reference the tokens in slice 1.

### drill-in open/close

**Question answered**: "where did the board list go?"

- When user clicks a story row → DrillIn mounts.
- Enter: slide-in from right (translateX 16px → 0) + fade (opacity
  0 → 1), duration `--motion-duration-medium` (250ms), ease-out.
- Exit (Back clicked or Esc): reverse, duration
  `--motion-duration-fast` (150ms), ease-in.
- Board list is not animated separately; DrillIn overlays in the
  same board-region slot. List remains in DOM (display: none after
  transition) so scroll position persists.

### board toggle (Ctrl+P)

**Question answered**: "did the board appear/disappear as I expected?"

- Board region's flex-basis transitions from 0 ↔ natural width
  over `--motion-duration-medium`, ease-in-out. Use
  `interpolate-size: allow-keywords` so the width-to-auto
  interpolation works without hardcoded pixel values.
- Chat region expands into the freed space naturally via flex.

### search + metrics overlay enter/exit

**Question answered**: "is this a different surface or the same one
modified?"

- Overlay mounts via conditional render inside chat-region slot.
- Enter: fade-in (opacity 0 → 1) + subtle translateY(-8px → 0),
  duration `--motion-duration-medium`, ease-out. Under
  reduced-motion: opacity-only.
- Exit (Esc or close): fade-out, duration
  `--motion-duration-fast`, ease-in.
- MessageList behind the overlay: no transition — it stays where it
  was, overlay covers it (z-index: 20 per existing .search CSS).

### reject-form reveal in drill-in

**Question answered**: "did the Reject button take effect?"

- When user clicks Reject → mode = 'rejecting' → form replaces the
  menu row.
- Grid-row expand from 0 → auto via `interpolate-size` + opacity
  0 → 1. Duration `--motion-duration-medium`, ease-out.
- Cancel: reverse. Duration `--motion-duration-fast`, ease-in.

### theme toggle color interpolation

**Question answered**: "is this a different theme or a flash?"

- When `[data-theme]` attribute on `<html>` changes, every token
  consuming color transitions smoothly (bg, fg, border, accent).
- Add global transition on `*` for color/background-color/border-color
  at `--motion-duration-fast` (150ms), ease-in-out. Scoped to
  `<html>` level so it applies universally without per-component
  opt-in.
- Caveat: globally-applied `*` transitions can hurt perf on large
  lists. If perf is observable, scope to specific property list
  rather than `all` + limit to theme-layer-relevant elements.

### `<details>` animated open/close (board groups — 2024+ CSS)

**Question answered**: "is this group collapsing/expanding, or did I
misclick?"

Board groups (READY, IN PROGRESS, DELIVERED, etc) already use
`<details>`/`<summary>` (confirmed in Board.tsx). Modern CSS
(`::details-content` + `interpolate-size: allow-keywords`) lets us
animate open/close:

```css
details.board-group {
  interpolate-size: allow-keywords;
}
details.board-group::details-content {
  opacity: 0;
  height: 0;
  overflow: clip;
  transition:
    opacity var(--motion-duration-fast) var(--motion-ease-in),
    height var(--motion-duration-fast) var(--motion-ease-in-out);
}
details.board-group[open]::details-content {
  opacity: 1;
  height: auto;
}
```

Under reduced-motion, transitions zero and the `<details>` still
toggles open/closed — just instantly.

### ACs (slice 2)

- Given a story row is clicked, then DrillIn fades + slides in with
  motion-duration-medium timing (unless reduced-motion is reduce,
  then it appears instantly).
- Given Ctrl+P is pressed, then board region width transitions
  smoothly from 0 ↔ its natural width.
- Given search is toggled on, then the overlay fades in over the
  MessageList with motion-duration-medium timing.
- Given theme is toggled, then color-bearing properties transition
  smoothly between the old and new theme values.
- Given a `<details>` group is toggled, then its content area
  animates open/closed over motion-duration-fast.
- Given reduced-motion is reduce, then all of the above apply
  instantly with no transform/translate.

### Playwright (slice 2)

- Click a story, assert the drill-in element transitions into place.
  Can't easily test frame-by-frame timing; assert `transition-duration` computed on the element is the motion-medium token's value.
- Under `emulateMedia({ reducedMotion: 'reduce' })`, the same
  `transition-duration` resolves to `0ms`.
- Theme toggle → assert `transition-property` on `<html>` or the
  theme-transition style sheet includes `background-color, color`.

## slice 3 — micro-interactions

Small confirmations that a click/hover/toggle took effect. Each
transition is 80–150ms, ease-out.

### filter pill toggle

- Currently: `--accent-brand` color snap on press.
- Add: bg transition from transparent → accent-mix 8%, bg transition
  from accent-mix 8% → accent-mix 16% on active. Duration
  `--motion-duration-instant`.

### button hover / active

- Already partial (.drill-in-btn, .titlebar-theme-toggle). Extend
  to all `<button>` elements by applying a global rule to
  `button:not(.no-motion)` with bg/color transitions at
  `--motion-duration-fast`.

### avatar appear-on-first-mount

- When a new message arrives (via WS), the row's avatar fades in
  from opacity 0 → 1 + slight scale(0.92 → 1) over
  `--motion-duration-fast`, ease-out. Applied only on mount, not on
  re-renders (scroll, filter change). Track via
  `IntersectionObserver` entry or `useEffect([])` with a CSS class
  toggle.

### message new-arrival

- New message row slides up (translateY 4px → 0) + fades in,
  duration `--motion-duration-medium`. Under reduced-motion:
  opacity only.

### ACs (slice 3)

- Given a filter pill is pressed, then its background transitions
  over motion-instant (not a snap).
- Given any button is hovered, then its bg/color transitions over
  motion-fast.
- Given a new message arrives via WS, then its row animates in
  (fade + translate) unless reduced-motion is reduce.

### Playwright (slice 3)

- Focus a filter pill, assert `transition-duration` is the
  motion-instant token's value.
- Send a WebSocket message via the real-backend harness; observe
  the new row has `animationend` fire within
  `motion-duration-medium` + 100ms buffer.

## empirical questions (revisit after live-use)

These are calibration decisions best made from live feel. Ship with
the proposed defaults, adjust if feedback signals "feels too slow" /
"feels too snappy" / "feels cluttered":

- exact timing tokens — 80/150/250 is a reasonable starting scale;
  might tune to 100/180/280 if the current choices feel rushed.
- drill-in enter direction — slide-in from right vs fade only. Right
  chosen to reinforce "came from the list"; fade-only is more
  minimal.
- theme toggle `*` scope vs per-element opt-in — start with `*` at
  slice 2; if perf regressions land, scope.

## non-goals (explicit deferrals)

- parallax, particle effects, spring physics (over-design for a
  collaboration UI)
- entry/exit animations for EVERY ephemeral element (search
  results, filter changes) — add only where the state-change
  question actually exists
- custom scroll-bound animations on long chat lists (perf-unfriendly
  + reduced-motion-hostile)
- loading skeletons with shimmer (nice-to-have, not justified by
  current wait times)
