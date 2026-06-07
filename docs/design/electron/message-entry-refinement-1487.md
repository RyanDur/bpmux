# Message-entry mount refinement (#1487 rework)

Rework spec after Ryan's walk-rejection. The pure grid-fr height-grow
(his prior pick over the fade-rise) is **"different but could use some
refinement"** — he wants the entry to read like a real chat room:
*"how messages enter from the bottom, below the fold so to speak."*

So the refinement target is a felt one: a new message should appear to
**rise up from below the fold** (enter from the bottom edge), not grow
in place. dev implements; designer **motion-live-verifies** at
desk-check (this is a motion story — still-capture can't confirm it);
Ryan AM re-walks.

## Why the current mount doesn't read "from the bottom" (grounded)

`MessageList.css`:
- `.message-list` (`:82-89`) is **`flex-direction: column-reverse`** —
  newest message is DOM-first, rendered at the **visual bottom**, list
  pinned to bottom.
- `.message-row` (`:235-244`): `grid-template-rows: 1fr` with
  `@starting-style { grid-template-rows: 0fr }` + `& > article {
  overflow: hidden }`. The row grows 0fr→1fr and the article content,
  anchored to its **top** (default flow), is revealed **top-first /
  expanding downward**.

That top-anchored downward reveal is the "different" feel Ryan flagged.
A chat-room entry reads the opposite direction: the message **rises into
view from the bottom edge**.

## Ryan's earlier constraint still holds

He rejected the **fade**-rise, then chose pure height-grow. He is NOT
re-asking for a fade — he's asking for a **rise**. So:
- ✅ a rise / slide-up-from-below (transform)
- ❌ NO opacity fade

## The refinement (bpmux tokens + css-canon)

Keep the height-grow (it does the "make room" layout Ryan picked). ADD a
layout-agnostic **rise-from-below** on the article — a `translateY` that
starts below its resting position and settles up to 0, no fade:

```css
.message-row {
  /* …existing grid-template-rows height-grow stays… */

  & > article {
    overflow: hidden;
    min-height: 0;
    transform: translateY(0);
    transition: transform var(--motion-duration-medium) var(--motion-ease-out);

    @starting-style {
      transform: translateY(10px);   /* starts below the fold → rises up; NO opacity */
    }
  }
}
```

Why this shape:
- **translateY, not content-anchoring:** the article has 3 internal
  layouts (normal flex `:107-111`; `display:grid` for `--first` /
  `--grouped` `:260-272`). A `justify-content: flex-end` reveal-anchor
  would behave differently across them; a transform on the article is
  identical for all three.
- **composes with the height-grow:** rows still make room via
  0fr→1fr; the new message additionally slides up into the space — the
  two together read as "enters from the bottom."
- **`--motion-duration-medium`** (not `-fast`): the rise wants to be
  *perceptible* as a rise, not a snap. Tunable — I'll dial the exact
  feel at the live-walk with Ryan's "other chat rooms" as the bar.
- **`--motion-ease-out`:** decelerate into place = "settling in."

## Must-verify at implementation (dev) + desk-check (designer, live)

1. **Only NEW messages animate — not the whole list on channel-open.**
   `@starting-style` fires on first render; confirm switching channels /
   initial load does NOT cascade-animate every row (re-mount storm). If
   it does, the entry must be gated to genuinely-new arrivals. ← top
   risk; verify live first.
2. **Direction reads as UP-from-below** at the bottom-pinned list (the
   column-reverse interaction) — the message should appear to come from
   the bottom edge, not drop in from above.
3. **`prefers-reduced-motion: reduce`** honored — instant placement, no
   transform (sibling to the existing `.new-messages-pill` guard
   `:72-76`). 
4. **No fade** anywhere in the entry.

## observation-falsifier (desk-check live-walk + Ryan AM)
I would reject if: the message still reveals top-first / grows-downward
rather than rising from below; OR any opacity fade was added back; OR
the whole list re-animates on channel-switch/open; OR reduced-motion
isn't honored; OR raw px/hardcoded timing instead of motion tokens; OR
it reads as "dropping in from the top." The live bar is Ryan's: "like
other chat rooms — enters from the bottom, below the fold."

## Note on verification
Per motion-live-verification: I will NOT pass this on static capture. At
desk-check I rebuild the renderer, open a channel, and watch a live new
message arrive (real send) to confirm the rise-from-below feel before
PASS — then Ryan AM-walks the final taste-call.
