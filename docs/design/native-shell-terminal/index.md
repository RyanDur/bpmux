# native-shell terminal — review-time mockup (story-bound)

**Status:** review-time visual for the native-shell-terminal outcome. Surface
confirmed by Ryan 2026-06-01 (Cetus planning).

**Canonical home of the SPEC = the story ACs, not this doc.** Per the
documentation-honest-homes canon, surface decisions + observation-falsifiers
live in the story bodies (board-visible substrate). This directory is the
**visual reference only** — it deliberately does NOT restate the decisions, so
it cannot drift from the stories. (We lived that drift: the owner-view spec
ran ~5 refinements above Ryan's model before a reshape caught it.)

**Story-bound — RETIRE-WHEN the outcome ships (lead story `#1466`, the trunk).**
This visual dies with the work; it does not outlive it. Slice→story binding:

| slice | story | what the visual shows |
|-------|-------|------------------------|
| S1 spawn real shell      | `#1466` (reshaped, TRUNK) | ready terminal: my `$SHELL`, real cwd, prompt |
| S2 resize-reflow         | `#1594` | live `cols × rows · TERM` in the header |
| S3 N terminals           | `#1595` | `terminals` sidebar section + `＋` + `N · spawn-dir` |
| S4 lifecycle states      | `#1581` (reshaped) | state gallery: ready / spawning… / spawn-failed / exited |
| S5 close/kill            | `#1596` (absorbs #1582) | `✕` on the row (the `＋` companion) |
| S6 live-cwd header-track | `#1597` | header tracks cwd; sidebar stays static |
| S7 copy-fidelity         | `#1598` | Cmd-C copy (paste + Ctrl-C are free in S1) |
| verdict-freshness chore  | `#1599` | (not visual) |

**Contents:**
- `native-terminal.html` — interactive mockup (resize to feel the reflow); uses
  the shipped `tokens.css` dark palette.
- `render.png` — static full capture.

**Scope reminder baked into the visual:** the board (right column) is tagged
`EXISTING BOARD · UNCHANGED · NOT IN SCOPE` — the mockup mirrors shipped chrome
everywhere except the center column it actually specs. Drift there reads as a
rework order; it isn't one.
