# Per-domain store slices

Target architecture for the store layer. Status: spec agreed with owner
2026-06-30; execution sequenced below, not yet started.

## The rule (owner, verbatim intent)

> Each domain is a slice. The slices live **with their domain folder** and
> are **registered** with the central store. The central store is **not**
> under `product_management`. No grab-bag of mixed items to filter.

So:

- There is **one central store** — `crate::store::Store`
  (`src/store/mod.rs`), wrapping `kernel::Store<AppState, AppAction>`
  (`src/store/kernel.rs`). It lives at `src/store/`, outside
  `product_management/`.
- `src/store/` holds **only** the kernel + the registration/composition
  (`app.rs`: `AppState` / `AppAction` / `app_reduce`, and the listener
  wiring in `Store::new`). **No slice code lives under `src/store/`.**
- **Every slice lives in its own domain folder** and is registered with
  the central store: `app.rs` *imports* the slice's `State` / `Action` /
  `reduce` from the domain module and composes them; `Store::new` adds the
  slice's listener.
- Reads go through **selectors**; writes go through **dispatch**. Nothing
  outside the kernel touches state directly.
- Ingestion is split per-domain before dispatch (one poll fans out into
  per-slice actions, or per-domain pulls — see board section).

## Registration mechanism (how a domain registers a slice)

Verified against `src/store/app.rs`:

- `AppState` is a struct of `Arc<XState>` fields, one per slice.
- `AppAction` is an enum with one `Domain(XAction)` variant per slice,
  plus `From<XAction> for AppAction`.
- `app_reduce` matches each variant and delegates to `x::reduce(&state.x,
  action)`, replacing only that slice's `Arc` on `Some`.
- `Store::new` constructs the kernel with `app_reduce` and calls
  `add_listener` for each slice's broadcast listener.

To register a slice that lives in a domain folder, `app.rs` imports
`XState`/`XAction`/`reduce`/`XBroadcast` **from the domain module**
(e.g. `crate::ci`, `crate::product_management::outcome`) instead of from
`crate::store::ci`. The kernel and `app.rs` stay put; the slice code moves
to the domain.

## Slice homes

| Domain | Slice today | Domain folder (home) | Status / landmine |
|---|---|---|---|
| ci | `src/store/ci/` | `src/ci/` (exists: `mod.rs`, `broadcast.rs`, `classify.rs`) | clean-ish; wide consumers + `poll.rs` lives in the slice |
| reactions | `src/store/reactions/` | `src/reactions.rs` (exists) | **LANDMINE: `src/reactions.rs` is a *separate* file-backed `ReactionsStore` with its own `AggregatedReaction`, distinct from the slice's `ReactionsState`. Must reconcile the two before/while moving — not a mechanical move.** |
| roster | `src/store/roster/` | **none yet** | **OPEN: no `src/roster/`. Belongs in a new `src/roster/`, or under `session`? Owner decision.** |
| sync | `src/store/sync/` | **none yet** | **OPEN: no `src/sync/`. Belongs with `integrations/github` / a new `src/sync/`? Owner decision.** |
| story | board grab-bag | `product_management/story/` (exists) | new slice; part of board dissolution |
| iteration | board grab-bag | `product_management/iteration/` (exists) | new slice; resolve read-decode asymmetry |
| outcome | board grab-bag | `product_management/outcome/` (exists) | new slice; template (smallest board domain) |
| epic | board grab-bag | `product_management/epic.rs` (exists) | new slice |

## The board grab-bag (the core of the work)

Today the board data is a **second store** in the wrong place and wrong
shape: `BoardStore` (`product_management/board/store/`,
`kernel::Store<BoardState, BoardAction>`) holds **one**
`HashMap<StoryKey, ProductItem>` — all four domains mixed, served by
filtering (`BoardStore::outcomes()`/`iterations()`/`epics()`/
`all_stories()`).

Target:

- The four board domains become **slices of the one central store**
  (`AppState`), each owning its typed collection in its domain folder.
- `product_management/board/` becomes the **state machine** (lanes /
  transitions over the current iteration's stories) — a *consumer* of the
  story slice via a selector, **not** the data's home.
- `BoardStore`, `BoardState`, and the `ProductItem` mixed enum
  **dissolve** once the last domain has its own slice.
- Cross-domain rollups (epic counts ← child stories; outcome readiness ←
  iterations ← member stories) become **explicit selector reads across
  slices**, not implicit reaches into one map.
- Per-domain listeners replace the single board-broadcast loop that
  currently re-broadcasts all five wire channels on every change: each
  slice's listener broadcasts only its channel and enqueues only its
  mutations (outbox stays behind the slice).

## Execution sequence (gated, one slice at a time)

1. **Prove the mechanism** on the cleanest existing slice (NOT reactions —
   see landmine). `ci` is the most self-contained with a real home; do it
   first as the relocate+register template, behaviour-preserving + gated.
2. **Board domains** (the core ask), vertically: **outcome** first
   (smallest), then **iteration**, **story**, **epic**. Each: create the
   slice in its domain folder, register it, route ingest (split before
   dispatch), migrate reads → selectors and writes → dispatch, remove the
   domain from the `ProductItem` map. `BoardStore` dissolves when the last
   one leaves.
3. **Reconcile + relocate reactions** (resolve the duplicate-impl
   landmine), and **roster / sync** once their homes are decided.

Each step: nextest + vitest + e2e green, committed, pushed, ls-remote
verified — and never reported "done" until it is.

## Open decisions for the owner

- **roster** home: new `src/roster/`, or under `session`?
- **sync** home: new `src/sync/`, or with `integrations/github`?
- **reactions**: is `src/reactions.rs` (file-backed) live and separate
  from the slice, or should the two be unified into one `src/reactions/`?
