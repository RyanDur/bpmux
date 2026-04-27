# Electron native HTML primitive audit

Ryan 2026-04-25 21:37 UTC during acceptance walk on #444 (CollapsibleSection
hydration error from button-in-button structural defect): *"there is a
better element than making your own CollapsibleSection. https://developer
.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/details — please do
not reinvent the wheel."*

Concrete instance: custom CollapsibleSection wrapped each section header
in a `<button>`, which was then nested inside a parent `<button>` row
(StoryRow). React DOM hydration warning. The class is broader than this
one component — any custom-rolled primitive where native HTML exists is
a quality-paramount risk + accessibility-loss + maintenance burden.

This audit enumerates the renderer surface, categorizes each candidate
as **replace** / **keep-custom** / **defer**, and names the migration
plan for replace-class items so future stories can sequence the work.

## 3-field spec header

**user-job:** When the team builds new collapsible / modal / progress /
popover affordances in the Electron renderer, the default reach is the
native HTML primitive (`<details>` / `<dialog>` / `<progress>` / etc.)
unless a documented native-gap forces a custom component. The audit
makes the current state legible and gives each existing custom
component a verdict that the next migration story can act on without
re-deriving the analysis.

**observation-falsifier:** I would reject this audit if:

- it omits a custom primitive in the renderer that has a native
  equivalent (enumeration-incompleteness)
- a categorization (replace / keep / defer) lacks the source-grounding
  for its verdict (file path + native-equivalent + a11y-regression
  consideration)
- a replace-class verdict has no migration plan naming what changes +
  which usages move
- a keep-custom verdict has no explicit native-gap rationale (what
  native primitive is missing that forces custom)
- the audit recommends deletion-of-current-feature without separately
  filing the migration story (audit ≠ migration; audit names what + why,
  migration stories execute)

**team-shape-independence:** Native primitives behave consistently for
any user (owner-Ryan or agent-via-renderer-screenshot); the audit
prescribes the SHAPE, not who-clicks-what. The migration stories
inherit per-feature audience tagging via the team's filing discipline.

## Principle alignments

- **`<details>` / `<dialog>` / `<progress>` etc. carry built-in
  accessibility** — keyboard interactions, focus management,
  screen-reader semantics — that custom components must each
  re-implement (often partially). Choosing native is the
  quality-paramount move.
- **`feedback_prefer_native_html_primitives`** (Ryan 2026-04-25):
  *"do not reinvent the wheel"* — name `<details>` / `<dialog>` /
  `<progress>` at spec time; desk-check scans for custom-primitive
  smells.
- **`feedback_quality_is_paramount`** (Ryan 2026-04-25): hydration
  warnings on touched surfaces are auto-reject. Native primitives
  avoid the structural-violation class entirely.
- **`feedback_html_css_js_separation`**: prefer native HTML semantics
  + CSS presentation; reach for layout primitives (flex/grid/sticky)
  + native attributes (`open`, `[open]` selector, `@starting-style`)
  before useRef + scrollIntoView + layout effects.

## Audit summary

| # | Component | Native equivalent | Verdict | Sites |
|---|---|---|---|---|
| 1 | `CollapsibleSection` | `<details>` / `<summary>` | **replace** | Board.tsx (1 import; ~5 instances per board view) |
| 2 | `Reactions.Picker` | none (closest: `<menu>` for context-menus) | **keep-custom** | Reactions.tsx (2 instances per component) |
| 3 | `PinnedList` | `<dialog>` | **replace** | Chat.tsx (1 site) |
| 4 | `Metrics` | `<dialog>` | **replace** | Chat.tsx (1 site) |
| 5 | `DrillIn` reject-form modal | `<dialog>` | **replace** (form-portion only) | Board.tsx → DrillIn.tsx (1 site) |

5 candidates total. 4 replace; 1 keep-custom. Zero defer.

## 1. CollapsibleSection → `<details>` / `<summary>` (REPLACE)

**File:** `applications/electron/src/renderer/src/lib/CollapsibleSection.tsx`

**Current shape:** Renders a `<button>` for the section header (chevron
glyph "▸" / "▾" + label) plus a controlled `collapsed` prop and
`onToggle` callback. Children render conditionally on `!collapsed`.
LocalStorage persistence wired via `useCollapsibleSectionsStorage` hook
called from the parent (Board.tsx).

**Native fit:** `<details>` + `<summary>` provides the same affordance
out of the box — `open` attribute encodes state, `toggle` event fires
on user activation, no React-controlled wiring needed for the basic
open/close. LocalStorage persistence works against `toggle` events
identically.

**Native gains beyond parity:**

- **No more nested-button class.** A `<details>`/`<summary>` is not a
  `<button>` — `<summary>` is the activator. When CollapsibleSection
  wrapped its header in `<button>` and the parent (StoryRow) was also
  a `<button>`, React DOM rejected the hydration. With native, the
  parent can stay clickable and `<summary>` activates locally —
  structurally valid.
- **Built-in keyboard support.** Enter / Space toggle without
  per-component wiring. Disclosure marker is rendered + announced by
  AT.
- **Native chevron via `summary::-webkit-details-marker` or
  `::marker`.** Custom "▸ / ▾" glyph becomes purely CSS — same visual,
  no React state coupling.

**Migration plan (file as separate story):**

- One site: `Board.tsx` imports + uses `CollapsibleSection`. Each board
  section (4 state buckets in panel + 1 group view) maps 1:1.
- Preserve `data-test` attrs on `<summary>` (test surface).
- Replace `collapsed` / `onToggle` props with native `open` + a `toggle`
  event handler that writes to localStorage via the existing hook.
- Delete `CollapsibleSection.tsx` after the last consumer migrates
  (per `feedback_replace_story_includes_delete` — same story).
- Visual parity: chevron glyph styled via `summary::marker` (or
  `list-style: '▸'` on `<details>` not-open state, '▾' on `[open]`).

**A11y note:** strict win — built-in keyboard + AT + native disclosure
state.

## 2. Reactions Picker → no native equivalent (KEEP-CUSTOM)

**File:** `applications/electron/src/renderer/src/chat/Reactions.tsx`
(lines 36–104)

**Current shape:** Absolute-positioned emoji palette anchored to a
trigger button (the `+` reaction-add). Measures the anchor's
`getBoundingClientRect()` on mount, computes above/below placement +
flips when viewport space exhausts. Document-level click-outside +
Escape listeners for dismiss. Renders a button-grid of emoji choices.

**Native fit considered:**

- `<menu>` is the closest semantic peer — but `<menu>` is for context
  menus (right-click) + lacks the anchored-placement model. Browsers
  don't position `<menu>` at an arbitrary trigger; the popover is
  custom-positioned regardless.
- `<datalist>` is for `<input>` autocomplete only — wrong shape.
- The newer Popover API (`popover` attribute + `popovertarget`) gets
  closer for the dismiss + focus-restore semantics, but anchored
  positioning still requires `anchor-name` / `position-anchor` (CSS
  Anchor Positioning, partial support).

**Native gap:** No native primitive provides anchored emoji-grid
popover with above/below flip. `<menu>` + Popover API handle dismiss
behavior but not anchored placement; the placement heuristic is
genuinely custom UI logic.

**Verdict: KEEP-CUSTOM.** Native has no peer; reinventing the wheel
isn't possible because the wheel doesn't exist.

**Watch-list (not in this story's scope):**

- When Chrome ships full CSS Anchor Positioning support across browsers,
  consider re-evaluating: `popover` + `anchor-name` could replace the
  custom placement heuristic. Trigger condition for re-audit: anchor
  positioning at >90% browser-share (today ~70% behind flag).

**A11y considerations preserved:**

- `aria-haspopup="true"` + `aria-expanded` on the trigger — already
  present. Continue to apply.
- Picker grid should support arrow-key navigation (current impl uses
  per-button click only — minor a11y nit, file separate Bug if not
  already covered).
- Escape dismisses + restores focus to trigger — already implemented.

## 3. PinnedList → `<dialog>` (REPLACE)

**File:** `applications/electron/src/renderer/src/chat/PinnedList.tsx`

**Current shape:** Full-height side panel showing pinned messages.
Visibility driven by a `data-visible` attribute used by CSS for
enter/exit animation; Escape + close-button dismiss; positioned via
absolute layout.

**Native fit:** `<dialog>` (or `<dialog open>` for non-modal /
side-anchored) provides:

- Native open/close API (`showModal()` / `show()` / `close()`)
- Built-in Escape-to-close
- Automatic focus management (focus first focusable on open, restore
  on close)
- `[open]` CSS selector for state-driven styling
- `@starting-style` for enter animation without `useAnimatedUnmount`
  hook gymnastics

**Native gains beyond parity:**

- The 350ms exit animation currently coordinated via
  `useAnimatedUnmount` becomes a pure CSS transition on `[open]` →
  `:not([open])` with `@starting-style` for the enter side. JS
  unmount-coordination disappears.
- Focus-trap is automatic for `showModal()`; if the side-panel is
  *non-modal* (Chat.tsx remains usable behind it), use `show()` + the
  panel doesn't trap focus. Match current behavior.

**Migration plan (file as separate story):**

- One site: `Chat.tsx` renders `PinnedList`.
- Choose between `showModal()` (modal, focus-trap, blocks chat) and
  `show()` (non-modal, both surfaces interactive). Current behavior
  is non-modal — use `show()`.
- Replace `data-visible` attribute + `useAnimatedUnmount` with native
  `open` attribute + CSS `[open]` styling + `@starting-style` for
  enter animation.
- Preserve `aria-label` on the dialog. Preserve close-button.

**A11y note:** strict win on focus-management; preserves Escape-handling
without listener boilerplate.

## 4. Metrics → `<dialog>` (REPLACE)

**File:** `applications/electron/src/renderer/src/metrics/Metrics.tsx`

**Current shape:** Same architecture as PinnedList — overlay,
`data-visible` driven CSS animation, Escape dismiss. Renders velocity
+ DORA metrics fetched async on render.

**Native fit:** identical analysis to PinnedList. `<dialog>` is the
right shape. Same migration mechanics.

**Migration plan (same story as PinnedList suggested):** the two
overlays share the same architectural pattern; folding both into one
"replace-overlays-with-`<dialog>`" story keeps audit-trail tight
(audit names both → one migration story → both ship together).

**A11y note:** strict win, same as PinnedList.

## 5. DrillIn reject-form → `<dialog>` (REPLACE — form portion only)

**File:** `applications/electron/src/renderer/src/board/DrillIn.tsx`

**Current shape:** DrillIn is a story-detail panel. It already uses
native `<details>` for the timeline section (line 246) — that part is
the canonical model and stays. The **reject-form** sub-component is a
custom modal-style form rendered conditionally + animated via
`useAnimatedUnmount` (150ms exit).

**Native fit:** `<dialog>` is the right shape for the reject-form
specifically. The form-portion is modal (blocks the underlying drill-in
while the reject-reason is being typed) → `showModal()` is the right
verb.

**Native gains:**

- Focus moves into the form on open + restores to the trigger on close
- Escape closes the form (and currently must be wired manually)
- 150ms exit animation as CSS transition on `:not([open])`

**Migration plan (file as separate story):**

- One site: `DrillIn.tsx`.
- Convert reject-form's outer wrapper to `<dialog>` + use `showModal()`
  on open / `close()` on submit-or-cancel.
- Preserve form fields + submit-handler.
- Keep timeline `<details>` untouched (it's already correct).

**A11y note:** strict win on focus-management for the form context.

## Out-of-scope discoveries (file separately)

The audit surfaced two adjacent quality-classes that are NOT
custom-primitive issues but warrant filing for separate work:

### StoryBadge button-in-button (structural defect)

**File:** `applications/electron/src/renderer/src/board/StoryBadge.tsx`
(lines 206–223)

Renders as `<button>` when a `handler` prop is provided. This button
is then a child of `<button>` StoryRow (Board.tsx lines 721–753).
Same structural class as #444 hydration warning. The fallback
(when `handler` is absent) renders as `<span>` — that path is correct.

**Recommended filing:** `[Bug]` — StoryBadge button-in-button structural
defect when handler prop present. Fix path: change StoryBadge to render
as `<a>` (if action is navigation) or change StoryRow's outer element
to a non-`<button>` (and use a click-handler on a non-interactive role).

### useRef + scrollIntoView smells

Three sites use `useRef` + `scrollIntoView` to drive scroll positioning
from JS rather than via CSS / native focus mechanics:

- `DrillIn.tsx` line 294 (timeline focus on open — RAF-double-frame
  pattern)
- `MessageBody.tsx` line 38 (mention-focus scroll)
- `Reactions.tsx` lines 37–51 (Picker position measurement in
  `useEffect` — not `useLayoutEffect`, so layout thrashing possible
  on browsers that batch)

These are HTML-CSS-JS-separation smells per
`feedback_html_css_js_separation`. Native alternatives to consider for
each (file per-site Chores if pursued):

- `scroll-margin` + native `:focus` + `scroll-padding-top` for the
  scroll-into-view-with-offset class (DrillIn timeline + MessageBody
  mention)
- CSS Anchor Positioning + `position: anchor()` for popover placement
  (Reactions Picker — coupled with the keep-custom watch-list above)
- `useLayoutEffect` instead of `useEffect` for synchronous DOM-read
  pre-paint (Reactions, if measurement stays JS-driven for now)

These don't fit native-primitive-replacement (the audit's user-job) —
filing separately keeps the audit's audit-trail tight.

## Migration sequencing recommendation

If migrations are split into separate stories (recommended), suggested
sequence by ROI / risk:

1. **CollapsibleSection → `<details>`** — closes the rejection-class
   from #444 (button-in-button). Highest user-impact + simplest
   substrate. Single migration site.
2. **PinnedList + Metrics → `<dialog>`** — bundle into one story
   (same architectural pattern, same Chat.tsx parent). Replaces
   `useAnimatedUnmount` for two overlays in one go.
3. **DrillIn reject-form → `<dialog>`** — narrow scope (one form, one
   site). Lower-risk; can ship after the bigger overlays.
4. **StoryBadge button-in-button** — separate Bug (out-of-scope here).

Audience tagging guidance for migration stories: each is **both-walk**
— Ryan observes the surface (board / pinned-list / metrics / drill-in
all surface to him); team-agents render against the same renderer in
Playwright tests + read the same components.

## Designer pre-pickup-review

This audit is the designer-authored input dev consumes before any
migration story picks up. Per #462 spec body: *"Designer-pre-pickup-
review on the audit before dev consumes."* The pre-pickup-review pass
verifies:

- Each replace-verdict has a named migration site + native-equivalent
  + a11y-gain
- Each keep-custom-verdict has a named native-gap rationale
- No defer-class items lack a trigger condition (this audit has zero
  defers, so trivially satisfied)
- Out-of-scope discoveries are named + recommended-filing-shapes are
  cited

Audit complete; ready for migration-story filing per the sequence above.

## Source

- Subagent enumeration of `applications/electron/src/renderer/src/`
  2026-04-27 ~00:46 UTC; designer-lane interpretive grading + native-fit
  + a11y rationale composed against the enumeration.
- Ryan 2026-04-25 21:37 UTC + retro convergence 22:11 UTC + designer's
  22:14 UTC commitment to post-walk audit.
- Saved lessons consulted: `feedback_prefer_native_html_primitives`,
  `feedback_quality_is_paramount`, `feedback_html_css_js_separation`,
  `feedback_replace_story_includes_delete`,
  `feedback_subagent_grounding_not_principal_grounding`.
