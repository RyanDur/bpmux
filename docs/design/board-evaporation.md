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
| entity | `peer.rs` (`PeerId`) | domain (`product_management`/`story`) | backend-pure |

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

1. **Dissolve the façade.** Rewrite consumers off the pass-through re-exports
   (`board::{StoryEvent, MutationId, RefreshDriver, SyncOrchestrator, PersistError, …}`)
   to their real homes (`crate::product_management::story::events`, `crate::sync`).
   Large mechanical fan-out, zero behavior change — shrinks board to its genuine
   content and makes every remaining board import name real board content.
2. **Pin the wire.** Golden round-trip tests locking `StoryView`/`IterationView`/
   `OutcomeView`/`story_to_board_json` JSON (the message-split S1 move) so slices 4–5
   are provably byte-identical to the renderer.
3. **CODECs + MAPPERs → `integrations/github`.** The `GhLabelCodec` impl + the
   `*_decode` mappers + `label_fields`/`comment_codec`/`relationship_block`/
   `typed_label_mirror`. The `LabelCodec` trait *port* stays in the domain. Aligns
   with the existing `product_management ↛ crate::github` boundary test.
4. **VIEW_MODELs → `crate::view`.** `StoryView`/`IterationView`/`OutcomeView` +
   `json` + `relationships`. Guarded by the slice-2 wire goldens.
5. **PROJECTIONs → `crate::view`/selectors**; **`SourceAdapter` port + `in_memory`
   adapter → domain/adapters home**; **`PeerId` → domain**.
6. **Delete `board/mod.rs`.** Board has evaporated. Add a fitness test asserting the
   `product_management::board` path no longer exists (a divisor-less husk left behind
   is the regression to refuse).

## Key decisions (so they are not re-litigated)

- **Pure relocation, wire byte-identical.** No `StoryView` field is added, removed, or
  renamed in this arc; the renderer contract is frozen and pinned by goldens BEFORE
  any view-model file moves. A reshape is a later, coordinated step.
- **The port stays, the impl travels.** `SourceAdapter` and `LabelCodec` are ports the
  domain owns (gateway canon); their GitHub implementations move to `integrations/github`.
- **Façade dissolution first.** Rewriting the pass-through re-exports (slice 1) is what
  turns a 100+-consumer fear into four small clusters; it is behavior-preserving and
  gates nothing downstream on it beyond legibility.
- **`crate::view` is the read-model home** — already true for `roadmap`; the view-models
  join it, not a new module.

## Fitness functions

- Extend the wire goldens (slice 2) — they refuse a silent `StoryView` reshape.
- `product_management::boundary_tests` already forbids `crate::github`/`octocrab` in the
  domain; the CODEC/MAPPER move (slice 3) is *toward* that boundary (out of the domain
  edge-ward), so it strengthens, never weakens, the existing lock.
- Slice 6 adds a "`product_management::board` is gone" existence check (mirrors the
  storage/transport boundary-test pattern).
