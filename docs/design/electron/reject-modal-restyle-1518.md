# Reject-modal restyle (#1518 rework)

> ⚠️ **SUPERSEDED 2026-06-06 — this restyle SHIPPED and was REJECTED. Do not build to it.**
> The CSS restyle below was implemented (`abc471bc` "restyle the reject modal as an elevated
> comment-modal" + `b68b3b2a` "primary-rightmost button order") and is live on main. Ryan
> walked it (img_7, 2026-06-06) and rejected it as still-wrong ("the look is…"). So this doc
> no longer describes pending work — it describes the REJECTED build, and its "Current state
> = flat box" section below is **stale** (it predates the shipped restyle). Building to this
> doc re-creates the rejected look.
>
> **Current direction:** the live defect is PROXIMITY, not missing elements — the built form
> uses a uniform `--space-12` gap between title / field / actions (`DrillIn.css:284`), so no
> grouping reads. The proximity-grounded redesign (24px inter-group, 4–8px intra) + Ryan's
> pending taste-pick live in **`reject-modal-1518-mockup.html`** (same dir). Use that.
>
> Kept below for the lesson (what was tried + why it didn't satisfy), NOT as a build-spec.

---

Rework spec after Ryan's walk-rejection. Centering is DONE + accepted
(Ryan: "it **is** center") — the rework is **visual style/polish only**.
Ryan: *"it could use some better style, I would recommend looking up
other comment modal boxes."* So: bring the modal from "functional
bordered box" up to a polished comment-modal. dev implements the CSS;
designer motion/visual-verifies at desk-check; Ryan AM re-walks.

## Current state (grounded — `DrillIn.css:259-339`, `DrillInActions.tsx:185-234`)

```
<dialog class="drill-in-reject-form">          flat 1px --border-subtle box, --bg-base,
  <form>                                       padding --space-12, max-width 32rem,
    <label class="drill-in-reject-label">      NO radius, NO shadow
      <small>…</small>                         uppercase --fg-dim micro-label
      <textarea data-test="drill-in-reject-note"/>   --bg-highlighted, focus→--border-strong
    </label>
    <button data-test="drill-in-reject-send">Send</button>   ← unstyled / default
    <button>Cancel</button>                                   ← unstyled / default
  </form>
</dialog>
::backdrop  rgb(0 0 0 / 50%)
```

Reads as a plain box: no elevation (so it doesn't float above the
board), no radius (sharp, un-modal-like), no title (no clear "what is
this"), and the action buttons are unstyled — no primary/secondary
hierarchy. That's the "needs better style."

## The restyle (all bpmux tokens + css-canon — classes only, parent-owns-layout)

### 1. Container `.drill-in-reject-form` — make it FLOAT + soften
| prop | from | to | why |
|---|---|---|---|
| box-shadow | (none) | `0 8px 32px rgb(0 0 0 / 45%)` | elevation — a modal floats above; the flat border alone reads inline |
| border-radius | (none) | `8px` | rounded = modal, not panel (matches `.avatar-tooltip` 6px / pills) |
| border | `1px --border-subtle` | `1px --border-subtle` (keep) | edge definition under the shadow |
| padding | `--space-12` | `--space-16` | comfortable comment-modal breathing room |
| background | `--bg-base` | `--bg-elevated` | elevated surface token = visually "lifted" (matches tooltip/pre) |
| max-width | `32rem` | `32rem` (keep) | good comment-modal measure |
| gap (`[open]`) | `--space-8` | `--space-12` | a touch more between title / field / actions |

### 2. NEW — a title `.drill-in-reject-title`
A comment modal states what it is. Add a heading as the first child of
the form (dev: `<h2 class="drill-in-reject-title">Reject story</h2>`):
```
font-size: var(--font-size-15);   /* if absent, --font-size-16 */
font-weight: 600;
color: var(--fg-primary);
margin: 0;
```
(`<h2>` is correct semantics inside the dialog — native heading, no
ARIA. Also wire the dialog's `aria-labelledby` to the title id.)

### 3. Field label `.drill-in-reject-label > small` — keep, lower the shout
Keep the field-label, but it should read as a quiet field-label, not a
second heading now that there's a title:
```
small { color: var(--fg-secondary); text-transform: none;
        letter-spacing: normal; font-size: var(--font-size-12); }
```
Suggested copy: "Reason (sent to the team)" — gives the operator the
mental model that this becomes a visible comment.

### 4. Textarea — clearer focus + comfortable size
```
min-height: 80px;          /* from 64 — comfortable for a real reason */
border-radius: 6px;        /* from 4 — match container softening */
&:focus {
  border-color: var(--accent-brand);                 /* from --border-strong */
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent-brand) 30%, transparent);
}
```
Background stays `--bg-highlighted` (now sits well on the elevated
surface). The accent focus-ring is the single clearest "this is where
you type" cue + matches `:focus-visible` accent elsewhere.

### 5. Buttons — give them hierarchy + right-align (the biggest visible win)
Wrap the two buttons in `.drill-in-reject-actions` (dev adds the div):
```
.drill-in-reject-actions {
  display: flex;
  justify-content: flex-end;     /* actions right-aligned — modal convention */
  gap: var(--space-8);
  margin: 0;                     /* parent gap owns spacing */
}
```
**Send = primary** `.drill-in-reject-send`:
```
background: var(--accent-brand);
color: var(--bg-base);
border: 1px solid var(--accent-brand);
border-radius: 6px;
padding: var(--space-8) var(--space-16);   /* 2:1 h:v — Ryan's button grammar */
font-size: var(--font-size-13);
font-weight: 600;
cursor: pointer;
&:hover { filter: brightness(1.1); }
&:disabled { opacity: 0.6; cursor: default; }   /* the "Sending…" state */
&:focus-visible { outline: 2px solid var(--accent-brand); outline-offset: 2px; }
```
**Cancel = secondary/ghost** `.drill-in-reject-cancel`:
```
background: transparent;
color: var(--fg-secondary);
border: 1px solid var(--border-subtle);
border-radius: 6px;
padding: var(--space-8) var(--space-16);   /* 2:1 h:v — matches Send (Ryan's button grammar) */
font-size: var(--font-size-13);
cursor: pointer;
&:hover { color: var(--fg-primary); border-color: var(--border-strong); }
&:focus-visible { outline: 2px solid var(--accent-brand); outline-offset: 2px; }
```
Order: Cancel then Send in DOM is fine; right-aligned puts Send at the
far right (primary-rightmost convention). Keep the existing Enter→send /
Esc→close behavior.

### 6. Backdrop — keep (optionally a hair of blur)
`rgb(0 0 0 / 50%)` is good. Optional modern touch: add
`backdrop-filter: blur(2px)` — nice but skippable; don't block on it.

### 7. Motion — unchanged
The opacity fade-in on `[open]` (`@starting-style`) stays; it's a modal,
the gentle fade is correct (this is NOT the chat-mount story #1487).
Keep `prefers-reduced-motion` honored if a guard exists.

## observation-falsifier (desk-check + Ryan AM re-walk)
I would reject if: the modal still reads flat/inline (no perceptible
elevation distinguishing it from the board); OR there's no title; OR the
Send/Cancel buttons still have no primary/secondary distinction or
aren't right-aligned; OR the button padding is not Ryan's 2:1 h:v
grammar (`--space-8`/`--space-16` = 8/16); OR any value is a raw hex/px
instead of a token (css-canon); OR tag-selectors are used instead of the
classes above; OR a focus state is invisible (a11y); OR centering regressed.

## What's explicitly OUT
The centering (accepted), the dialog open/close logic, the reject
submit-flow — untouched. This is a CSS-only polish pass on the existing
structure (+ two wrapper/title elements dev adds).
