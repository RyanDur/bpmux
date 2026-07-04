# board/ evaporation — the final Phase-0 relocation

Status: mapped firsthand 2026-07-03 (dev). Sister to `app-continuum-restructure.md`
(this is the "board/ evaporates" bullet of its Phase 0) and to `board-projection-model.md`
(Phase 1's verdict collapse, which lands *after* this). Source model: the App
Continuum — the board is a *function*, not a module.

## North star (must survive compaction)

> `product_management/board/` is a fossil host. The orchestrator that gave it its
> name already moved out to `crate::sync` (earlier Phase 0). What remains is a
> **re-export façade** over `story::events` + `crate::sync`, wrapped around four
> small genuine clusters. Evaporation relocates each cluster to its right home and
> deletes the husk. **The entire move is behavior- and wire-preserving; done as pure
> relocations it needs no renderer/designer coordination.** The board becomes a set
> of projection functions and a GitHub adapter, exactly as the projection model predicts.

## Current state (audited firsthand 2026-07-03)

34 production files, ~2.3k LOC. `board/mod.rs` re-exports a surface of which **only
`PeerId`, `SourceAdapter`, `SyncError` originate inside `board/`**; `StoryEvent` is
`story::events`, and `MutationId`/`RefreshDriver`/`SyncOrchestrator`/`PersistError`/…
all pass through from the top-level `crate::sync` (aliased `board::orchestrator`).
So board/ is mostly a façade; its genuine content is four clusters:

| Cluster | Files | Destination | Coupling |
|---|---|---|---|
| **Label CODECs** | `label_codec.rs` (the port *trait*), `adapters/gh_label_codec/*`, `adapters/github/label_fields.rs`, `adapters/relationship_block.rs`, `adapters/github/comment_codec.rs`, `adapters/in_memory/typed_label_mirror.rs` | `integrations/github` (the impl); the `LabelCodec` trait *port* stays domain-side per the gateway canon | backend-pure |
| **GitHub MAPPERs** | `adapters/github/{story,iteration,outcome,epic}_decode.rs` (Value→domain) | `integrations/github` (mappers travel with the provider) | backend-pure |
| **VIEW_MODELs** | `view_model/{story,iteration,outcome}.rs` (`StoryView`/`IterationView`/`OutcomeView`), `adapters/json.rs`, `adapters/relationships.rs` | read-model (`crate::view` — where `roadmap` already lives) | wire-FACING |
| **PROJECTIONs** | `arrangement.rs` (`column_for`), `ready_order_label.rs` (`sort_board_ready_by_membership`), `velocity/*` | projection fns (`crate::view` / domain selectors) | backend-pure |
| SYNC port + in-mem adapter | `sync.rs` (`SourceAdapter` port), `adapters/in_memory/*` | domain keeps the port; in-memory adapter → an adapters home | backend-pure |
| entity | `peer.rs` (`PeerId`) | domain (`product_management/story/peer.rs`, beside `events.rs`) — **moves in slice 1**, see why below | backend-pure |

**Why `PeerId` moves first, not last (verified at substrate 2026-07-03):**
`story/events.rs:4` — the domain's own `EventMeta` — imports `board::peer::PeerId`,
so the domain's event type reaches INTO the fossil, not just through it. And ~44
references across ~30 files (all of `bridge/tools_stories`, all of `sync/`,
transport, storage tests) name `board::PeerId`/`board::peer::PeerId` at `EventMeta`
construction sites — largely the SAME files slice 1 rewrites off the pass-throughs.
Deferring `PeerId` to slice 5 would run the same mechanical fan-out over the same
file set twice and leave slice 1's endpoint dishonest. One fan-out, once.

## The classification: overwhelmingly backend-pure

Every cluster except the view-models produces domain values or GitHub label strings —
no renderer sees them. And the view-models are only wire-**facing**, not
wire-**changing**: `/api/board` serializes `StoryView`/`IterationView`/`OutcomeView`
+ `story_to_board_json` into `serde_json::Value`. **Relocating those to `crate::view`
while preserving their `Serialize` output is byte-identical on the wire** (the
renderer deserializes exactly the same JSON — the same discipline that kept the
message-split's wire JSON byte-identical).

Therefore the whole evaporation is a sequence of behavior/wire-preserving
relocations — the *same* move shape already executed this session (#4, #5,
storage-ports, message-split). The **only** work that would require designer/renderer
coordination is a deliberate wire **reshape** — the redux-slice endgame where the
~13 denormalized `Story` fields become selectors and the `raw_labels` shadow
collapses. That is a *separate, later* step (Phase-1+ board-as-slice), explicitly
NOT part of this relocation.

## Slices (each backend-pure, CI-green, fitness-locked where it caps a boundary)

1. **Dissolve the façade + home `PeerId`.** ✅ LANDED `7f3b0f1c` (2026-07-03).
   Moved `peer.rs` → `product_management/story/peer.rs` (verbatim 100% rename; it is
   `EventMeta`'s own field type, so it lives beside `events.rs`), and rewrote every
   consumer off the pass-through re-exports
   (`board::{StoryEvent, MutationId, RefreshDriver, SyncOrchestrator, PersistError, …}`)
   AND off `board::peer::PeerId`/`board::PeerId` to their real homes
   (`crate::product_management::story::{events, peer}`, `crate::sync`) — one mechanical
   fan-out over the shared consumer set, zero behavior change, wire byte-identical
   (174 files). `board/mod.rs` now declares only genuine content (adapters, arrangement,
   label_codec, ready_order_label, sync, velocity, view_model + the SourceAdapter/SyncError
   port re-export); every surviving `board::` reference names real board content. Two
   execution notes: `sync/orchestrator_tests/*` had been reaching back through the façade
   to `crate::sync` (where they already live) — that loop is cut; and the compiler caught
   the one grep-invisible class — brace-grouped imports `board::{A, B, C}` spanning
   multiple destination homes — which were split per-home.
2. **Remove the unfinished `blocked_by` mechanism (SUBTRACTION — Ryan, 2026-07-04).**
   ✅ LANDED `32842070` (2026-07-04). Net −557 LOC across 41 files, done on Opus.
   Execution note: the removal maps (from read-only Explore agents) covered unit tests
   but MISSED the Playwright e2e specs — the pre-push e2e gate caught 3 real failures in
   `desk-check-390.spec.ts` (an entire spec dedicated to the blocked visual indicator,
   incl. an assertion on the degenerate `/no blocker specified/` tooltip). That whole
   file was deleted and folded into the slice (amend, unpushed → atomic). Lesson for the
   remaining slices: enumerate `applications/electron/tests/*.spec.ts` too, not just
   `*.test.ts`. The other blocked e2e refs (`real-backend.spec.ts`) are all
   roster-presence — correctly kept.
   Verified at substrate: `blocked_by` has NO first-class affordance (settable only by
   poking a raw `blocked-by:N` label through the generic label-edit paths —
   `bridge/tools_stories/metadata.rs`, `transport/rest/labels.rs`); the bare
   `Tag::Blocked` flag is produced by NO bpmux path at all (only hand-labeling the
   GitHub issue) and renders degenerate ("blocked — (no blocker specified)"). A story
   exists on the board to build blockers RIGHT (first-class block action + relation,
   no labels) — carrying the half-built vestige through the refactor teaches the next
   agent the wrong pattern. Remove, one vertical slice: `Story.blocked_by` +
   `Tag::Blocked`; `BlockedBySet`/`BlockedByCleared` events + lifecycle reducer arms;
   codec pair (`encode/decode_blocked_by`, `BLOCKED_LABEL`, `BLOCKED_BY_PREFIX`) +
   `blocked_by_from_labels`; the label-edit set paths; `wire_labels` blocked entries;
   `integrations/github/mutation_events/blocked.rs`; `typed_label_mirror`/`story_decode`/
   `in_memory`/`sync/ingest`/`conflict.rs` arms; renderer board-side (`blocked` badge
   kind + `BlockerGlyph`, `blockerIdFromLabels`, the `--blocked` row class,
   `DrillIn`/`UnscheduledList` traces) + their tests. `Tag` shrinks to
   `{DeskCheckFail}`. CAUTION: roster-presence `Blocked` (owner-park;
   `usePresence.ts`, `Sidebar.tsx`, `MemberList.tsx`, `PresenceWire::Blocked`) is an
   UNRELATED concept sharing the name — do not touch. Inert data: any live
   `blocked-by:N` labels on GitHub issues simply stop being decoded — no crash, no
   migration. Subtraction runs BEFORE the goldens/relocations so we never pin or move
   code scheduled to die.
3. **Pin the wire.** ✅ LANDED `879b8b14` (2026-07-04). Golden round-trip tests locking
   `StoryView`/`IterationView`/`OutcomeView`/`story_to_board_json` JSON (the message-split
   S1 move) so slices 5–6 are provably byte-identical to the renderer. Pins the
   POST-subtraction wire (no blocked labels). `StoryView` was already pinned (json_tests.rs:
   full key-list + exact count 11); added `board/view_model/wire_golden_tests.rs` for
   `IterationView`/`OutcomeView` (the two that had NO wire test) — full key-set + exact
   count + nested `readiness`/`BuildingIterationView` shape, proven-to-falsify via a probe
   field. These goldens travel WITH the view-models to `crate::view` in slice 6.
4. **Complete the `LabelCodec` port; de-couple the read-model from the concrete codec.**
   The port is INCOMPLETE (`label_codec.rs`): it exposes `encode_ready_order`/
   `encode_iteration`/`encode_parent_epic` but NOT `encode_tag`/`encode_acceptor`
   (inherent methods on `GhLabelCodec`; the third backdoor, `encode_blocked_by`, dies
   in slice 2). So `StoryView::wire_labels` — a read-model, domain side — backdoors to
   the concrete `GhLabelCodec` (`view_model/story.rs`): GitHub on the WRONG side of
   the adapter. Fix: promote `encode_tag` + `encode_acceptor` onto the `LabelCodec`
   trait, and repoint `wire_labels` to the injected `codec: &C` for ALL labels (drop
   the `GhLabelCodec` import). After this NO production read-model names the concrete
   codec — the view is generic over the port. Byte-identical (method bodies move
   verbatim; slice-3 goldens prove it). MUST precede slice 5 — else moving
   `GhLabelCodec` to `integrations/github` while the view still references it would
   turn a domain-internal smell into a domain→github edge.
5. **CODECs + MAPPERs → `integrations/github`.** The `GhLabelCodec` impl + the
   `*_decode` mappers + `label_fields`/`comment_codec`/`relationship_block`/
   `typed_label_mirror`. The `LabelCodec` trait *port* stays in the domain. Aligns
   with the existing `product_management ↛ crate::github` boundary test. With slice 4
   done, the read-model no longer references the concrete, so this move is clean.
6. **VIEW_MODELs → `crate::view`.** `StoryView`/`IterationView`/`OutcomeView` +
   `json` + `relationships`. The view depends only on the `LabelCodec` port (slice 4),
   which is domain-side — `crate::view` never names the GitHub codec. Guarded by the
   slice-3 wire goldens.
7. **PROJECTIONs → their per-consumer homes**; **`SourceAdapter` port + `in_memory`
   adapter → domain/adapters home**. Each projection file's destination is decided
   by the who-consumes-it test (below), file-by-file at execution, and recorded here.
   (`in_memory/typed_label_mirror.rs` also names the concrete `GhLabelCodec` — an
   adapter-side coupling, not a read-model leak; resolve when the in-memory adapter moves.)
8. **Delete `board/mod.rs`.** Board has evaporated. Add a fitness test asserting the
   `product_management::board` path no longer exists (a divisor-less husk left behind
   is the regression to refuse).

## Key decisions (so they are not re-litigated)

- **Subtraction precedes relocation (Ryan, 2026-07-04).** Never spend slices pinning
  or carefully moving code that is scheduled to die. The `blocked_by` vestige comes
  out (slice 2) BEFORE the goldens pin the wire and before the codec/view relocations
  — every downstream slice moves less. Same reasoning killed the original slice-4 plan
  to promote `encode_blocked_by` onto the port.
- **`blocked_by` is removed, not finished (Ryan, 2026-07-04).** It was a GitHub-label
  vestige: no first-class affordance, unreachable bare flag, degenerate rendering. The
  board story for blockers builds the capability RIGHT later (a real block action +
  relation + glyph, no labels). Removing ≠ rejecting the feature — it clears the
  half-built mechanism so the future build starts clean. Successor design:
  `board-as-slice-reshape.md` records the target model.
- **Pure relocation, wire byte-identical.** Apart from the slice-2 subtraction (a
  deliberate, Ryan-directed removal), no `StoryView` field is added, removed, or
  renamed in this arc; the renderer contract is frozen and pinned by goldens BEFORE
  any view-model file moves. The broader reshape is a later step —
  `board-as-slice-reshape.md`.
- **The port stays, the impl travels.** `SourceAdapter` and `LabelCodec` are ports the
  domain owns (gateway canon); their GitHub implementations move to `integrations/github`.
- **GitHub stays on the far side of the adapter (Ryan, 2026-07-03, standing).** No
  read-model / projection / view — anything the domain or `crate::view` owns — may name
  the concrete `GhLabelCodec`; it depends only on the `LabelCodec` port. The concrete
  codec is injected from an edge/composition (REST handlers, bridge tools, test seeds —
  where naming it is correct). The port must be COMPLETE enough that no consumer needs
  to backdoor to the impl (slice 4 closes the current gap). The existing
  `product_management ↛ crate::github` fitness test guards the vendor mechanism; this
  principle extends it to the concrete codec once it lands in `integrations/github`.
- **Façade dissolution first.** Rewriting the pass-through re-exports (slice 1) is what
  turns a 100+-consumer fear into four small clusters; it is behavior-preserving and
  gates nothing downstream on it beyond legibility.
- **`crate::view` is the read-model home** — already true for `roadmap`; the view-models
  join it, not a new module.
- **Projection destinations resolve by the who-consumes-it test.** "`crate::view` /
  domain selectors" is two homes, not one: a projection whose output is serialized
  outward (feeds `/api/board` or another wire snapshot) goes to `crate::view`; a
  projection consumed by domain policy (a reducer, a gate, workflow logic) is a
  domain selector and stays in `product_management`. Decide per-file against actual
  consumers at slice 7, never by cluster-membership alone.
- **`PeerId` travels with slice 1, not the projections slice** — its consumer set overlaps the
  façade's (rationale in the cluster table above); the timing is a substrate-verified
  decision, not a preference.

## Fitness functions

- Extend the wire goldens (slice 3) — they refuse a silent `StoryView` reshape.
- `product_management::boundary_tests` already forbids `crate::github`/`octocrab` in the
  domain; the CODEC/MAPPER move (slice 5) is *toward* that boundary (out of the domain
  edge-ward), so it strengthens, never weakens, the existing lock.
- Slice 8 adds a "`product_management::board` is gone" existence check (mirrors the
  storage/transport boundary-test pattern).

## Successor arc

`board-as-slice-reshape.md` — labels off the wire entirely (typed facets), containers
own membership/order (epics-hold-children finish, iteration ordering), `DeskCheckFail`
collapses into the Phase-1 verdict model. Converged with Ryan 2026-07-04; sequenced
AFTER this evaporation (it is a wire reshape; this arc is wire-preserving).
