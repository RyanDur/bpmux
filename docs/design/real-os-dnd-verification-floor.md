# Real-OS DnD + real-GH verification floor — design

Designer-authored 2026-06-06, while-fresh, from the iteration/ready-queue drag-reorder saga.
The agenda#9 "real-OS-DnD verification floor" — now with demonstrated, concrete evidence.
Future-improvement (retro/planning input); does NOT jump the current drag-rework.

## Problem (demonstrated this session — not hypothetical)

EVERY defect in the #1517/#1524 drag-reorder work surfaced at RYAN's real-mouse walk, never at
a gate:
- blank drag-ghost (the source's `opacity:0.001` got captured into the default drag-image snapshot)
- vanish-after-drop (the in-tree ghost-clone disrupted React's reconcile)
- persist snap-back (iteration-order labels accumulated — the REMOVE didn't land)
- last-item no-op (reorder position-math at the list-end)
- non-deterministic reorder (different outcomes per try)

Meanwhile EVERY automated gate greened over the broadly-broken real board: my desk-check PASSED,
PM's delivery-audit PASSED, the mock-GitHub e2e was green, CI was green. The common cause: **no
gate reproduces (a) real-OS NATIVE drag-drop or (b) the real-GitHub round-trip.** Synthetic /
Playwright DnD can't drive the OS drag-image snapshot + native dragstart/drop + React-reconcile-on-
real-drop; mock-GitHub can't reproduce the label write/remove/ingest timing. So the felt-render and
real-persist classes are invisible to every gate → only the owner's walk catches them → **the owner
becomes the regression suite.** That's the floor we're missing.

## What the floor must verify (the walk-node classes)

1. **FELT-RENDER (real-OS DnD):** across a REAL native drag — the drag-ghost is visible + a lifted
   solid card (per the drag-reorder-visual-contract), the source shows its placeholder gap, and the
   dropped row does NOT vanish post-drop. (NOT a synthetic `dispatchEvent` / Playwright `dragTo` —
   those are the exact blind spot.)
2. **REAL-PERSIST (real-GH round-trip):** reorder → write → REMOVE-stale → ingest → reload → order
   holds + EXACTLY ONE canonical label per item (no accumulation), across boundary cases
   (first / last / 3-element).
3. **DETERMINISM:** same drag → same result across repeats.

## Approach (thin-first → north-star)

- **Thin-first** (validate the riskiest assumption — *can a gate reproduce what only the walk
  caught?*): a real-OS-input driver on a REAL electron build (drive an actual OS-level drag — via
  the electron-bridge `simulate_*`/`capture` path or a native-input harness), asserting the
  post-drop DOM (ghost-gone, item-present-at-dropped-index) + a real-GH-staging round-trip (throwaway
  GH fixtures, drag → reload → assert single-label) on the boundary cases.
- **North-star:** a full real-OS-DnD + real-GH-staging suite that runs the owner's walk-node sequence
  as a GATE — so deliver/desk-check/CI catch the felt-render + real-persist classes BEFORE the owner
  walks.

## Why this, not "more mock tests"

The defects are PRECISELY what mocks can't see (the OS drag-image snapshot, React-reconcile-on-real-
drop, the real-GH write/remove/ingest timing). More mock-coverage greens vacuously — as it did all
session. The floor must run the REAL substrate (real OS-input + real GH), or it reproduces the same
blind spot it's meant to close.

## Verification-discipline tie-in

This is the substrate-enforce of the walk-node-gate: today the gate REQUIRES a walk-node-evidence
artifact (forces the review); the floor makes that artifact AUTOMATABLE for the real-OS-DnD class —
the review runs the real drag instead of deferring it entirely to the owner. The owner's walk stays
the final felt-confirm; the floor catches the gate-able real-OS/real-GH defects first, so the owner
isn't the regression suite.

## Observation-falsifier (for the floor itself)

I would reject the floor if: it uses synthetic / Playwright DnD instead of real OS-input (the blind
spot); it uses mock-GitHub instead of a real-GH-staging round-trip; it asserts only the happy path
(not the boundary cases first/last/3-element + the remove / no-accumulation); it doesn't run on a
real electron build.
