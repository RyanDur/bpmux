# Backend restructure — the App Continuum target

Status: converged with Ryan 2026-07-03. This is the authoritative design for the
backend's structural refactor. Sister to `board-projection-model.md` — that doc's
verdict collapse is **Phase 1** of this sequence. Source model:
https://www.appcontinuum.io/ (already the declared foundation in
`docs/architecture.md`; this doc supersedes that doc's stale module map).

## North star (must survive compaction)

> **The structure is the prompt.** Well-organized code prompts future agents into
> better patterns and decisions; badly organized code prompts badly. The backend
> follows the App Continuum: **thin applications** over **feature and support
> components**. The deliverable of this refactor is the organization itself —
> tests pin that behavior survives; readability is judged by eye.

## The applications — four thin shells

Applications are named by **delivery surface**: who consumes bpmux, through what door.

```
bpmux/                        ← the product (the repo owns the name)
├── applications/
│   ├── server/               ← the daemon: composition root; composes the domain,
│   │                            owns store wiring, REST/WS, persistence, GitHub sync
│   ├── electron/             ← the owner's door (desktop UI) — WS client of server
│   ├── cli/                  ← the terminal door (bpmux verbs) — REST client of server
│   └── mcp/                  ← the agents' door (the bridge) — client of server
└── components/               ← features, support, edges — all the substance
```

- **The product name belongs to the repo, never to a crate under `applications/`.**
  Today's `applications/bpmux` — the app is bpmux, and bpmux holds an application
  called bpmux — is the tell that shell and substance are one undifferentiated thing.
- An application is nearly empty: `main()`, config resolution, compose components, go.
- **Only `server` composes the domain.** The three client shells depend on
  protocol/client components. If a client shell can't build without linking the
  domain, the boundary is fake — fix it.
- CLI ≡ MCP ≡ human: same substance, cohesive views per door (standing canon).

## Current state (audited firsthand 2026-07-03)

Single crate, 24 flat top-level modules — **v3 eroding**. Features, support, and
edges all sit at one altitude; `telemetry` is a sibling of `product_management`.

| Class | Modules |
|---|---|
| FEATURE | `product_management` (~20k LOC, the core domain), `room`, `message`*, `roster`, `ci`, `reactions`, `memory`, `awareness`, `comprehension` |
| SUPPORT | `store` (redux kernel + app-store composition), `workflow` (policy), `view` (read-model), `routing`, `diag`, `log`, `telemetry` |
| EDGE | `transport` (~13k), `bridge` (~26k), `server` (composition root), `mcp`, `cli`, `storage`, `integrations`, `github` |

\* `message` is a shared-DTO grab-bag (domain `Message` + WS wire payloads) that
reaches up into `ci` and `store` (`message/state_payloads.rs:61,149,164`) — split it.

## The five violations blocking a clean lift

1. **`bridge` is a feature smuggled inside an edge.** The MCP/RPC delivery surface
   also contains the story-lifecycle engine (`bridge/tools_stories/**`: creation,
   transition, acceptance, authorization — the six gate handlers). ~26.3k LOC, the
   largest module in the backend. The engine belongs in `product_management`; the
   edge translates tool calls → dispatch, nothing more.
2. **The domain depends on its own adapters.** `product_management` imports the
   `transport` and `storage` edges (`board/orchestrator/builder.rs:33,79`,
   `board/orchestrator/mod.rs:53,61`, `owner_park.rs:70`). See the orchestrator
   finding below — most of this mass is misfiled, not mis-abstracted.
3. **`storage` knows every domain.** The persistence edge imports six feature
   modules (`storage/mod.rs:4-12`). Feature-first wants each feature to declare a
   repo port and storage to implement it — the arrow is backwards.
4. **`transport` is a god-edge meshed with other edges.** It fans out to `bridge`,
   `integrations`, `github`. ~~and reaches into the composition root for
   `OrchestratorHandle`.~~ **The composition-root reach is RESOLVED (2026-07-03):**
   `OrchestratorHandle` now homes in `crate::sync` beside `RefreshDriver` (it is
   `Option<Arc<dyn RefreshDriver>>`), reached via a two-move arc that first lifted
   `CiStateStore`/`CiStateReport` out of `transport` into the `ci` feature — cutting
   the `sync ↔ transport` cycle that had blocked the alias's move. A fitness test
   (`transport::boundary_tests::transport_edge_names_no_composition_root`) now fails
   the build if any transport source names `crate::server`. Remaining #4 residue: the
   god-edge mesh itself (transport fanning out to bridge/integrations/github) — edges
   should be independent adapters over feature ports, not a mesh.
5. **Kernel and policy cycles.** ~~`store ↔ product_management`~~ **RESOLVED
   (2026-07-03):** the redux kernel (generic `<S,A>` engine) is promoted to a
   top-level `crate::kernel` leaf (domains register into it without importing the
   composition), and `owner_park`'s cross-slice orchestration `Listener` moved from
   the domain into the composition (`store/app`). A fitness test
   (`product_management::boundary_tests::product_management_production_imports_no_store_composition`)
   fails the build if pm production names `crate::store::`. The app-composition still
   aggregates domain slices (store→pm) — the correct "composition depends on domains"
   direction; the cycle (pm→store) is gone. The `workflow ↔ product_management` arm is
   mostly homed (`StageId` → `crate::workflow` `883cae11`; acceptor→GateClass mappers →
   pm `9b0c15ee`); its last strand (`workflow::verdicts` importing pm) relocates INTO
   pm during the Phase-1 verdict collapse, not before.

## The orchestrator finding — a fossil, and the cheap fix for violation 2

`product_management/board/orchestrator/` is not board logic. Its files say what it
is: `ingest.rs`, `fetch.rs`, `outbox.rs`, `write_through.rs`, `coalesce.rs`,
`race_window.rs`, `persist_error.rs` — **the eventually-consistent sync engine
between the store and GitHub**. It lives under `board/` because it once
orchestrated the BoardStore; that host was dissolved, the orchestrator stayed in
the dead host's house.

Consequence: violation 2 is mostly solved by a **relocation, not an inversion**.
Move the orchestrator to the sync component beside `integrations/github` and its
storage/CI dependencies stop being violations — sync machinery legitimately owns
them. No speculative port traits. The residue (`owner_park.rs:70` →
`transport::ws::short_name`) is a duplicated-helper smell (`short_name` also lives
in `routing/store.rs`); single-home it.

With the orchestrator gone, **`board/` evaporates** — exactly what the projection
model predicts (the board is a function, not a module): label codecs → the GitHub
adapter; `view_model` → the read-model; `arrangement` / `story_view_projection` →
projection functions in the domain. Mapped firsthand in `board-evaporation.md`
(the sequence, the destinations, and the finding that it is entirely backend-pure
wire-preserving relocation — no renderer coordination for the moves themselves).

## github vs integrations — right split, wrong altitude

Two levels of one gateway, both hoisted to top level: `github` = the vendor wire
mechanism (octocrab wrapper, rate-limit, error classification — a true leaf);
`integrations/github` = the domain-facing mapper (implements the domain's
`SourceAdapter` port). They regroup into one **github-sync** component cluster with
the orchestrator: wire client + mapper + sync engine, composed by `server`.

## Phases

**Phase 0 — relocations + cycle cuts (single crate, behavior-preserving moves).**
Orchestrator → sync component beside `integrations/github`. `OrchestratorHandle`
out of `server` to its owner. `StageId` / shared value types homed to cut
`workflow ↔ product_management`. Slice-registration seam out of `store` so the
kernel is a leaf. `message` split (domain type vs wire DTOs). `short_name`
single-homed. Storage's feature imports inverted to per-feature repo ports.
Cycles must be cut here **while they are still cheap** — inside one crate a cycle
is a smell; across crates it is a compile error fought simultaneously with the
design.

**Phase 1 — the verdict collapse as relocation.** Per
`board-projection-model.md`: the six gate handlers re-cast as verdicts
(`Succeed | NeedsWork(target)` as the outcome facet of the existing evidence
`Verdict` — one type, never a rival). The write path is **born inside
`product_management`**; `bridge/tools_stories` dissolves; `bridge` becomes
translation only. Ordering is deliberate: the orchestrator moves out (Phase 0)
*before* the verdicts move in, so the new write path lands in a domain holding
only entities, policy, and projections. Runs WITH the board-as-slice reshape
(`board-as-slice-reshape.md`, converged 2026-07-04): labels collapse to the
GitHub adapter (the wire speaks typed facets; the renderer's mirror label-parser
dies), containers own membership/order (epics gain children refs; `parent_epic`/
`iteration_slot`/`ready_order` leave the story), and `Tag::DeskCheckFail`
dissolves into the verdict model.

**Phase 2 — the lift.** Components become crates under `components/`; the four
applications become thin shells. Cargo then *enforces* what Phase 0 established:
explicit declared dependencies, independent build and test per component, no
cycles possible. This is the Continuum's v4→v5 shape (packaged components,
multiple applications, one repository).

## Fitness functions — the boundary refuses regression

`product_management::boundary_tests` enforces today that the `board` god-module
stays gone (`product_management_board_module_is_gone`) and that the story slice
reducer does no GitHub label decode (`board_reducer_does_no_label_decode`,
forbidding `integrations::github` / `gh_label_codec` / `_from_labels` /
`project_labels` tokens under `src/story/slice/reducers`). The `octocrab` /
`reqwest` / `rate_limit` / `bridge::tracker` import ban this section once
described is now the crate boundary itself — `product_management`'s
`Cargo.toml` carries no dependency edge to any of them, so the compiler
refuses the import; no fitness test names them any more.
**Each Phase 0 move extends the forbidden list** — after the orchestrator leaves,
`crate::transport` and `crate::storage` join it. The structure doesn't just
prompt the next agent; it refuses their regression at compile time. At Phase 2
the crate graph subsumes these tests.

Landed boundary tests so far: `storage::boundary_tests` (persistence edge composes
per-feature repo ports only — violation #3); `transport::boundary_tests`
(`transport_edge_names_no_composition_root` — the transport edge names no
`crate::server`, locking the OrchestratorHandle relocation — violation #4);
`product_management::boundary_tests::product_management_production_imports_no_store_composition`
(pm production names no `crate::store::`, locking the store↔pm cycle-cut — violation #5).

## Key decisions (so they are not re-litigated)

- **The server is its own application** — a genuinely distinct deployable with its
  own lifecycle; the thing that must be up for any door to work. Not a CLI verb.
- **Four applications: server, electron, cli, mcp.** Named by delivery surface.
- **Relocate before inverting.** Where a dependency violation's mass is misfiled
  code (the orchestrator), move it; mint ports only for what remains.
- **Orchestrator out before verdicts in.** Phase 0 before Phase 1, always.
- **No `common/` / `util/`** — support components are named by purpose (the
  Continuum's own rule; those names prompt badly).
- **`docs/architecture.md` updates ship with each phase** — it is the prompt the
  next agent reads first; a stale map is worse than none. Its module map is ~2.5
  months behind today.

## Open (deliberately unresolved)

- **Component granularity at Phase 2** — exact crate list and names (e.g., whether
  `diag`/`log`/`telemetry` consolidate; whether `room` splits PTY mechanism from
  agent-terminal feature). Decide at the lift, informed by Phase 0's residue.
- **Binary packaging** — four applications as four binaries vs. shared packaging
  for distribution. A deployment concern, not a structure concern; defer.
