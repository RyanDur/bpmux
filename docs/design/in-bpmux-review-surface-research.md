# In-bpmux design-review canvas — research distillation

Designer-curated from a deep-research pass (2026-06-06; 23 sources, 21 verified / 4 refuted claims).
Feeds the `in-bpmux-review-surface.md` design + the layout-wireframe. Research-input for the
Ryan+designer exploration — NOT a converged spec.

## North-star model (proven, from Figma/FigJam — emulate)

- **Comment = an anchored PIN** placed in a dedicated comment MODE (toggle on / Esc off, mutually
  exclusive with editing). Anchor by: clicking a point, click-dragging a REGION, or attaching to a
  specific OBJECT. Pins show author avatar + hover-preview. *(Figma help-docs, primary.)*
- **Inline threads ON the canvas** — comments live where they were placed and open reply threads
  in-context (not buried in a side panel only); replies thread under the original.
- **@mentions** notify; **resolve** closes the loop — hides the comment from both the canvas AND the
  list, reversible via a "show resolved" filter.
- **View-access-only commenting** (the key access-model): a reviewer comments with NO edit rights.
  → bpmux: **the owner reviews + pins comments without being a canvas editor; the designer edits.**
- **Separate the review surface from the design-of-record** (FigJam is distinct from the Figma file,
  to keep the source-of-truth clean). → bpmux: the canvas COEXISTS with chat, never clutters/replaces
  it — exactly Ryan's "keep the chat but see designs."
- **Real-time multiplayer** over a websocket single-source-of-truth eliminates file-emailing/drift
  (the collaboration foundation — but see the open-question on whether the MVP needs it).

## What to AVOID (verified pain-points)

- **Large/complex-file slowdown** (Figma documents ~2GB/tab, degradation from big rasters / nested
  components / heavy effects). bpmux is local-first Electron → **bound canvas/file complexity from day 1.**
- **Weak offline** (Figma has no full offline mode; offline edits can be silently lost). bpmux is a
  desktop app → **design offline-first.**
- **Comment-at-scale ergonomics**: one-comment-with-many-questions is hard to answer → Figma shrank the
  composer to nudge SHORT focused feedback; co-located pins overwhelm → Figma CLUSTERS them. Build both
  early (the owner is our primary commenter).

## Other-tools' lessons

- **InVision** ($2B, shut down 2024): a cautionary tale — bundle the review-loop with where the work
  lives, don't make it a separate siloed tool that gets out-competed. (Reinforces: canvas IN bpmux,
  not a separate app.)
- **tldraw** — a production-ready React infinite-canvas SDK (shapes/text/arrows + self-hosted
  real-time multiplayer/live-cursors); lets us skip canvas-engineering for the north-star.
  **CAVEAT (load-bearing): license is now source-available/commercial (free non-commercial, paid
  commercial; starter-kits MIT) and sync is self-hosted — verify terms before embedding.**
- **Miro / FigJam**: big-board lag is a real failure mode (bounds the complexity point above).

## Thin-first MVP → north-star (lean discipline)

- Pair the smallest slice with an **explicit learning hypothesis** + validate on the **narrowest
  audience** — here literally **designer + owner** (MVP Canvas / Lean Inception heuristic).
- **Smallest valuable slice:** render ONE design in a pane + a SINGLE anchored comment-pin the owner
  places, that **flows back into the team channel.** That validates the riskiest assumption (in-bpmux
  render + anchored-pin + flows-to-chat) with almost no canvas-engineering.
- **Incremental path:** region-anchoring → threaded replies → @mentions → resolve → pin-clustering →
  full infinite-canvas + multiplayer (tldraw, if licensing clears).

## Caveats (carry forward)

- Figma mechanics rest on Figma's own (primary, accurate but vendor-framed) docs. Multiplayer +
  pain-points lean on blogs but were corroborated by Figma's own engineering/help docs.
- **REFUTED — do NOT cite:** (a) pins are purely coordinate-anchored independent of object identity
  (they actually attach to + move WITH frames); (b) "feature bloat" as a verified Figma pain-point;
  (c) specific Figma file-heaviness warning strings. tldraw licensing + Figma offline/perf limits can
  change — re-verify at build time.

## Open questions for the Ryan+designer exploration (bpmux-distinctive — sources don't cover these)

1. **Canvas-comment → chat flow:** one-way notification, a two-way synced thread, or a chat message
   that deep-links to the pin? (This cross-surface routing is bpmux's distinctive need; closes the loop.)
2. **Anchoring target for bpmux designs:** pixel-pin / region-drag / object-attach — bpmux mockups are
   images, live React renders, OR TUI captures; which anchoring survives a design being re-rendered?
3. **Is real-time multiplayer even needed for the designer+owner MVP**, or is async pin-and-reply
   enough (avoiding the self-hosted-sync cost)? Likely async-first for the thin slice.
4. **The explicit learning hypothesis** the MVP validates (e.g., "anchored owner-comments measurably
   reduce the review-loop friction we hit with chat-only feedback like #1518's 'looks still wrong'").
